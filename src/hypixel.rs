use serde::Deserialize;

use crate::Error;

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

/// Rank keys stored in role rules, with their in-game display name
pub const RANKS: &[(&str, &str)] = &[
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

/// Fetches a player's Hypixel profile, `None` if they never joined Hypixel.
pub async fn fetch_player(
    http: &reqwest::Client,
    api_key: &str,
    uuid: &str,
) -> Result<Option<Player>, Error> {
    let response: PlayerResponse = http
        .get("https://api.hypixel.net/v2/player")
        .query(&[("uuid", uuid)])
        .header("API-Key", api_key)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(response.player.map(|player| Player {
        rank: player.rank(),
        discord: player
            .social_media
            .and_then(|social_media| social_media.links)
            .and_then(|links| links.discord),
    }))
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
    #[serde(default)]
    members: Vec<GuildMember>,
    #[serde(default)]
    ranks: Vec<GuildRank>,
}

#[derive(Deserialize)]
struct GuildRank {
    name: String,
    #[serde(default)]
    priority: i64,
}

/// Implicit rank of the guild owner, it isn't listed in the guild's `ranks`
pub const GUILD_MASTER: &str = "Guild Master";

#[derive(Deserialize)]
struct GuildMember {
    uuid: String,
    rank: String,
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

    /// Every rank a member can have, highest first
    pub fn rank_names(&self) -> Vec<String> {
        let mut ranks: Vec<&GuildRank> = self.ranks.iter().collect();
        ranks.sort_by_key(|rank| std::cmp::Reverse(rank.priority));
        std::iter::once(GUILD_MASTER.to_owned())
            .chain(ranks.into_iter().map(|rank| rank.name.clone()))
            .collect()
    }
}

pub enum GuildQuery<'a> {
    Id(&'a str),
    Name(&'a str),
    Player(&'a str),
}

pub async fn fetch_guild(
    http: &reqwest::Client,
    api_key: &str,
    query: GuildQuery<'_>,
) -> Result<Option<Guild>, Error> {
    let param = match query {
        GuildQuery::Id(id) => ("id", id),
        GuildQuery::Name(name) => ("name", name),
        GuildQuery::Player(uuid) => ("player", uuid),
    };
    let response: GuildResponse = http
        .get("https://api.hypixel.net/v2/guild")
        .query(&[param])
        .header("API-Key", api_key)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(response.guild)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rank(name: &str, priority: i64) -> GuildRank {
        GuildRank {
            name: name.to_owned(),
            priority,
        }
    }

    #[test]
    fn guild_ranks_are_listed_highest_first() {
        let guild = Guild {
            id: "id".to_owned(),
            name: "Guild".to_owned(),
            members: vec![GuildMember {
                uuid: "owner".to_owned(),
                rank: "GUILDMASTER".to_owned(),
            }],
            ranks: vec![rank("Member", 1), rank("Officer", 3)],
        };

        assert_eq!(guild.rank_names(), ["Guild Master", "Officer", "Member"]);
        assert_eq!(guild.member_rank("owner"), Some(GUILD_MASTER));
    }
}
