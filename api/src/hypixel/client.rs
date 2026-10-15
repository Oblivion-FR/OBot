use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use std::sync::Arc;

use super::limit::RateLimiter;
use super::{Guild, GuildResponse, Player, PlayerResponse};
use crate::cache::TtlCache;
use crate::error::ApiError;

/// How long Hypixel answers are reused. Callers only trust a cached answer when it lets them go
/// ahead: anything that would make them refuse (no Discord linked, not in the guild…) is asked
/// again, since the player may have just fixed it in game.
const CACHE_SECS: u64 = 600;

pub const BASE_URL: &str = "https://api.hypixel.net/v2";

/// The Hypixel API, shared by every route so they spend one rate limit budget and share cached
/// answers
pub struct Hypixel {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    limiter: RateLimiter,
    /// Players by UUID, `None` for players who never joined Hypixel
    players: TtlCache<String, Option<Player>, CACHE_SECS>,
    /// Guilds by ID, `None` for disbanded guilds
    guilds: TtlCache<String, Option<Arc<Guild>>, CACHE_SECS>,
}

fn upstream(source: reqwest::Error) -> ApiError {
    ApiError::Upstream {
        service: "hypixel",
        source,
    }
}

impl Hypixel {
    /// `base_url` is [`BASE_URL`], or a local server in tests
    pub fn new(http: reqwest::Client, base_url: String, api_key: String) -> Self {
        Self {
            http,
            base_url,
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
    ) -> Result<T, ApiError> {
        let mut retried = false;
        loop {
            self.limiter.acquire().await;
            let response = self
                .http
                .get(format!("{}/{endpoint}", self.base_url))
                .query(query)
                .header("API-Key", &self.api_key)
                .send()
                .await
                .map_err(upstream)?;
            let rate_limited = response.status() == StatusCode::TOO_MANY_REQUESTS;
            self.limiter.record(response.headers(), rate_limited);
            if rate_limited && !retried {
                retried = true;
                continue;
            }
            return response
                .error_for_status()
                .map_err(upstream)?
                .json()
                .await
                .map_err(upstream);
        }
    }

    /// A player's profile, `None` if they never joined Hypixel. `fresh` skips the cache.
    pub async fn player(&self, uuid: &str, fresh: bool) -> Result<Option<Player>, ApiError> {
        if !fresh && let Some(player) = self.players.get(&uuid.to_owned()) {
            return Ok(player);
        }
        let response: PlayerResponse = self.request("player", &[("uuid", uuid)]).await?;
        // No `player` in the response means they never joined Hypixel
        let player = response.player.map(Player::from);
        self.players.insert(uuid.to_owned(), player.clone());
        Ok(player)
    }

    /// A guild by ID, `None` if it doesn't exist. `fresh` skips the cache.
    pub async fn guild(&self, id: &str, fresh: bool) -> Result<Option<Arc<Guild>>, ApiError> {
        if !fresh && let Some(guild) = self.guilds.get(&id.to_owned()) {
            return Ok(guild);
        }
        let response: GuildResponse = self.request("guild", &[("id", id)]).await?;
        Ok(self.store_guild(id, response.guild))
    }

    fn store_guild(&self, id: &str, guild: Option<Guild>) -> Option<Arc<Guild>> {
        let guild = guild.map(Arc::new);
        self.guilds.insert(id.to_owned(), guild.clone());
        guild
    }

    /// Looks a guild up by name, for linking it. Always live, and cached by ID afterwards.
    pub async fn guild_by_name(&self, name: &str) -> Result<Option<Arc<Guild>>, ApiError> {
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
