mod client;
mod limit;

use serde::Deserialize;

use crate::Error;
pub use client::Hypixel;

#[derive(Deserialize)]
pub struct MojangProfile {
    /// UUID without dashes
    pub id: String,
    /// Name with its real capitalization
    pub name: String,
}

/// Resolves a Minecraft username to its profile, `None` if no such account exists.
pub async fn fetch_mojang_profile(
    http: &reqwest::Client,
    username: &str,
) -> Result<Option<MojangProfile>, Error> {
    let response = http
        .get(format!(
            "https://api.mojang.com/users/profiles/minecraft/{username}"
        ))
        .send()
        .await?;

    match response.status() {
        reqwest::StatusCode::NO_CONTENT | reqwest::StatusCode::NOT_FOUND => Ok(None),
        _ => Ok(Some(response.error_for_status()?.json().await?)),
    }
}

/// Current profile of an account, `None` if it was deleted. Also picks up name changes.
pub async fn fetch_mojang_profile_by_uuid(
    http: &reqwest::Client,
    uuid: &str,
) -> Result<Option<MojangProfile>, Error> {
    let response = http
        .get(format!(
            "https://sessionserver.mojang.com/session/minecraft/profile/{uuid}"
        ))
        .send()
        .await?;

    match response.status() {
        reqwest::StatusCode::NO_CONTENT | reqwest::StatusCode::NOT_FOUND => Ok(None),
        _ => Ok(Some(response.error_for_status()?.json().await?)),
    }
}

/// Rank rule value for players without any paid or special rank
pub const NO_RANK: &str = "NO_RANK";

/// Rank keys stored in role rules, with their in-game display name
pub const RANKS: &[(&str, &str)] = &[
    (NO_RANK, "No rank"),
    ("VIP", "VIP"),
    ("VIP_PLUS", "VIP+"),
    ("MVP", "MVP"),
    ("MVP_PLUS", "MVP+"),
    ("SUPERSTAR", "MVP++"),
    ("YOUTUBER", "YOUTUBER"),
    ("STAFF", "STAFF"),
];

pub fn rank_label(key: &str) -> &str {
    RANKS
        .iter()
        .find(|(candidate, _)| *candidate == key)
        .map_or(key, |(_, label)| label)
}

#[derive(Deserialize)]
struct PlayerResponse {
    player: Option<RawPlayer>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPlayer {
    rank: Option<String>,
    monthly_package_rank: Option<String>,
    new_package_rank: Option<String>,
    package_rank: Option<String>,
    social_media: Option<SocialMedia>,
}

#[derive(Deserialize)]
struct SocialMedia {
    links: Option<Links>,
}

#[derive(Deserialize)]
struct Links {
    #[serde(rename = "DISCORD")]
    discord: Option<String>,
}

#[derive(Clone)]
pub struct Player {
    /// The Discord account linked in the Hypixel social menu
    pub discord: Option<String>,
    /// A key of `RANKS`, `None` for players without a rank
    pub rank: Option<String>,
}

impl RawPlayer {
    fn rank(&self) -> Option<String> {
        let real = |rank: &Option<String>| {
            rank.clone()
                .filter(|rank| !matches!(rank.as_str(), "NONE" | "NORMAL"))
        };
        // `rank` is only set for special ranks, it hides the purchased ones
        match real(&self.rank).as_deref() {
            Some("YOUTUBER") => Some("YOUTUBER".to_owned()),
            Some(_) => Some("STAFF".to_owned()),
            None => real(&self.monthly_package_rank)
                .or_else(|| real(&self.new_package_rank))
                .or_else(|| real(&self.package_rank)),
        }
    }
}

impl From<RawPlayer> for Player {
    fn from(player: RawPlayer) -> Self {
        Self {
            rank: player.rank(),
            discord: player
                .social_media
                .and_then(|social_media| social_media.links)
                .and_then(|links| links.discord),
        }
    }
}

#[derive(Deserialize)]
struct GuildResponse {
    guild: Option<Guild>,
}

#[derive(Deserialize)]
pub struct Guild {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub tag: Option<String>,
    #[serde(default)]
    members: Vec<GuildMember>,
    #[serde(default)]
    ranks: Vec<GuildRank>,
}

#[derive(Deserialize)]
struct GuildRank {
    name: String,
    tag: Option<String>,
    #[serde(default)]
    priority: i64,
}

/// Implicit rank of the guild owner, it isn't listed in the guild's `ranks`
pub const GUILD_MASTER: &str = "Guild Master";

#[derive(Deserialize)]
struct GuildMember {
    uuid: String,
    rank: String,
    /// Unix time in milliseconds
    joined: Option<i64>,
    /// Guild XP earned per day, for the last 7 days
    #[serde(rename = "expHistory", default)]
    exp_history: std::collections::HashMap<String, u64>,
}

pub struct GuildMemberStats {
    /// Unix time in seconds
    pub joined_at: Option<i64>,
    /// Guild XP earned over the last 7 days
    pub weekly_xp: u64,
}

impl Guild {
    /// The guild rank of a player, `None` if they are not a member.
    pub fn member_rank(&self, uuid: &str) -> Option<&str> {
        self.members
            .iter()
            .find(|member| member.uuid == uuid)
            // Old guilds still use the legacy spelling
            .map(|member| match member.rank.as_str() {
                "GUILDMASTER" => GUILD_MASTER,
                rank => rank,
            })
    }

    /// When a player joined the guild and how much guild XP they earned this week,
    /// `None` if they are not a member
    pub fn member_stats(&self, uuid: &str) -> Option<GuildMemberStats> {
        let member = self.members.iter().find(|member| member.uuid == uuid)?;
        Some(GuildMemberStats {
            joined_at: member.joined.map(|millis| millis / 1000),
            weekly_xp: member.exp_history.values().sum(),
        })
    }

    /// Every rank a member can have, highest first
    pub fn rank_names(&self) -> Vec<String> {
        let mut ranks: Vec<&GuildRank> = self.ranks.iter().collect();
        ranks.sort_by_key(|rank| std::cmp::Reverse(rank.priority));
        std::iter::once(GUILD_MASTER.to_owned())
            .chain(ranks.into_iter().map(|rank| rank.name.clone()))
            .collect()
    }

    /// The short tag of a guild rank, `None` if that rank has no tag
    pub fn rank_tag(&self, rank: &str) -> Option<&str> {
        if rank.eq_ignore_ascii_case(GUILD_MASTER) {
            return Some("GM");
        }
        self.ranks
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(rank))
            .and_then(|candidate| candidate.tag.as_deref())
            .filter(|tag| !tag.is_empty())
    }
}

#[cfg(test)]
mod tests;
