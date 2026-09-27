use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use std::sync::Arc;

use super::limit::RateLimiter;
use super::{Guild, GuildResponse, Player, PlayerResponse};
use crate::Error;
use crate::cache::TtlCache;

/// How long Hypixel answers are reused. Callers only trust a cached answer when it lets them go
/// ahead: anything that would make them refuse (no Discord linked, not in the guild…) is asked
/// again, since the player may have just fixed it in game.
const CACHE_SECS: u64 = 600;

/// The Hypixel API, shared by the bot and the panel so they spend one rate limit budget and
/// share cached answers
pub struct Hypixel {
    http: reqwest::Client,
    api_key: String,
    limiter: RateLimiter,
    /// Players by UUID, `None` for players who never joined Hypixel
    players: TtlCache<String, Option<Player>, CACHE_SECS>,
    /// Guilds by ID, `None` for disbanded guilds
    guilds: TtlCache<String, Option<Arc<Guild>>, CACHE_SECS>,
}

impl Hypixel {
    pub fn new(http: reqwest::Client, api_key: String) -> Self {
        Self {
            http,
            api_key,
            limiter: RateLimiter::default(),
            players: TtlCache::default(),
            guilds: TtlCache::default(),
        }
    }

    /// Sends a request once the rate limit allows it. A 429 despite that (another program
    /// using the same key) waits for the next window and retries once.
    async fn request<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query: &[(&str, &str)],
    ) -> Result<T, Error> {
        let mut retried = false;
        loop {
            self.limiter.acquire().await;
            let response = self
                .http
                .get(format!("https://api.hypixel.net/v2/{endpoint}"))
                .query(query)
                .header("API-Key", &self.api_key)
                .send()
                .await?;
            let rate_limited = response.status() == StatusCode::TOO_MANY_REQUESTS;
            self.limiter.record(response.headers(), rate_limited);
            if rate_limited && !retried {
                retried = true;
                continue;
            }
            return Ok(response.error_for_status()?.json().await?);
        }
    }

    /// A player's profile from the cache, without calling Hypixel. The outer `None` means
    /// nothing is cached.
    pub fn cached_player(&self, uuid: &str) -> Option<Option<Player>> {
        self.players.get(&uuid.to_owned())
    }

    /// A player's current profile, `None` if they never joined Hypixel
    pub async fn player(&self, uuid: &str) -> Result<Option<Player>, Error> {
        let response: PlayerResponse = self.request("player", &[("uuid", uuid)]).await?;
        // No `player` in the response means they never joined Hypixel
        let player = response.player.map(Player::from);
        self.players.insert(uuid.to_owned(), player.clone());
        Ok(player)
    }

    /// A guild by ID, cached. Fine for lists and dropdowns; use `guild_with_member` to check
    /// membership.
    pub async fn guild(&self, id: &str) -> Result<Option<Arc<Guild>>, Error> {
        match self.guilds.get(&id.to_owned()) {
            Some(guild) => Ok(guild),
            None => self.fresh_guild(id).await,
        }
    }

    async fn fresh_guild(&self, id: &str) -> Result<Option<Arc<Guild>>, Error> {
        let response: GuildResponse = self.request("guild", &[("id", id)]).await?;
        Ok(self.store_guild(id, response.guild))
    }

    fn store_guild(&self, id: &str, guild: Option<Guild>) -> Option<Arc<Guild>> {
        let guild = guild.map(Arc::new);
        self.guilds.insert(id.to_owned(), guild.clone());
        guild
    }

    /// The guild if the player is one of its members. A cached guild only answers yes: a
    /// player missing from it may have joined since, so that is checked live.
    pub async fn guild_with_member(
        &self,
        id: &str,
        uuid: &str,
    ) -> Result<Option<Arc<Guild>>, Error> {
        let is_member = |guild: &Arc<Guild>| guild.member_rank(uuid).is_some();
        if let Some(Some(guild)) = self.guilds.get(&id.to_owned())
            && is_member(&guild)
        {
            return Ok(Some(guild));
        }
        Ok(self.fresh_guild(id).await?.filter(is_member))
    }

    /// Looks a guild up by name, for linking it. Always live, and cached by ID afterwards.
    pub async fn guild_by_name(&self, name: &str) -> Result<Option<Arc<Guild>>, Error> {
        let response: GuildResponse = self.request("guild", &[("name", name)]).await?;
        Ok(match response.guild {
            Some(guild) => {
                let id = guild.id.clone();
                self.store_guild(&id, Some(guild))
            }
            None => None,
        })
    }
}
