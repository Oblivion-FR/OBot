mod client;
mod limit;

use serde::Deserialize;

pub use client::{BASE_URL, Hypixel};

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
    /// `VIP`, `VIP_PLUS`, `MVP`, `MVP_PLUS`, `SUPERSTAR` (MVP++), `YOUTUBER` or `STAFF`, `None`
    /// for players without a rank
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

    /// UUIDs of every member
    pub fn member_uuids(&self) -> impl Iterator<Item = &str> {
        self.members.iter().map(|member| member.uuid.as_str())
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
