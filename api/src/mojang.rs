use reqwest::StatusCode;
use serde::Deserialize;

use crate::error::ApiError;

pub const PROFILES_URL: &str = "https://api.mojang.com/users/profiles/minecraft";
pub const SESSION_URL: &str = "https://sessionserver.mojang.com/session/minecraft/profile";

#[derive(Deserialize)]
pub struct Profile {
    /// UUID without dashes
    pub id: String,
    /// Name with its real capitalization
    pub name: String,
}

/// Mojang's account lookups. Not rate limited or cached: they're only used when a member
/// verifies or a name may have changed.
pub struct Mojang {
    http: reqwest::Client,
    profiles_url: String,
    session_url: String,
}

impl Mojang {
    /// The URLs are [`PROFILES_URL`] and [`SESSION_URL`], or a local server in tests
    pub fn new(http: reqwest::Client, profiles_url: String, session_url: String) -> Self {
        Self {
            http,
            profiles_url,
            session_url,
        }
    }

    /// Resolves a Minecraft username to its profile, `None` if no such account exists. The
    /// name must be checked with [`valid_name`] first, since it goes in the URL's path.
    pub async fn by_name(&self, name: &str) -> Result<Option<Profile>, ApiError> {
        self.get(format!("{}/{name}", self.profiles_url)).await
    }

    /// Current profile of an account, `None` if it was deleted. Also picks up name changes.
    pub async fn by_uuid(&self, uuid: &str) -> Result<Option<Profile>, ApiError> {
        self.get(format!("{}/{uuid}", self.session_url)).await
    }

    async fn get(&self, url: String) -> Result<Option<Profile>, ApiError> {
        let upstream = |source| ApiError::Upstream {
            service: "mojang",
            source,
        };
        let response = self.http.get(url).send().await.map_err(upstream)?;
        match response.status() {
            StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => Ok(None),
            _ => Ok(Some(
                response
                    .error_for_status()
                    .map_err(upstream)?
                    .json()
                    .await
                    .map_err(upstream)?,
            )),
        }
    }
}

/// Minecraft names are 1 to 16 letters, digits or underscores. Old accounts can be shorter than
/// the current 3 character minimum.
pub fn valid_name(name: &str) -> bool {
    (1..=16).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// A Minecraft UUID with or without dashes, as 32 lowercase hex digits without dashes
pub fn normalize_uuid(uuid: &str) -> Option<String> {
    let digits: String = uuid.chars().filter(|&c| c != '-').collect();
    let dashes_ok = uuid.len() == 32 || (uuid.len() == 36 && uuid.matches('-').count() == 4);
    (dashes_ok && digits.len() == 32 && digits.chars().all(|c| c.is_ascii_hexdigit()))
        .then(|| digits.to_ascii_lowercase())
}

#[cfg(test)]
mod tests;
