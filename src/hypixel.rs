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

#[derive(Deserialize)]
struct PlayerResponse {
    player: Option<Player>,
}

#[derive(Deserialize)]
struct Player {
    #[serde(rename = "socialMedia")]
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

pub enum LinkedDiscord {
    /// The player never joined Hypixel
    UnknownPlayer,
    /// The player joined Hypixel but has no Discord linked
    NotLinked,
    Linked(String),
}

/// Fetches the Discord account a player linked in their Hypixel social menu.
pub async fn fetch_linked_discord(
    http: &reqwest::Client,
    api_key: &str,
    uuid: &str,
) -> Result<LinkedDiscord, Error> {
    let response: PlayerResponse = http
        .get("https://api.hypixel.net/v2/player")
        .query(&[("uuid", uuid)])
        .header("API-Key", api_key)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let Some(player) = response.player else {
        return Ok(LinkedDiscord::UnknownPlayer);
    };
    let discord = player
        .social_media
        .and_then(|social_media| social_media.links)
        .and_then(|links| links.discord);

    Ok(match discord {
        Some(discord) => LinkedDiscord::Linked(discord),
        None => LinkedDiscord::NotLinked,
    })
}
