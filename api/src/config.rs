use std::env::var;

/// Settings read from the environment, and from a `.env` file in the working directory
pub struct Config {
    pub bind: String,
    pub database_url: String,
    pub hypixel_api_key: String,
}

fn optional(name: &str) -> Option<String> {
    var(name).ok().filter(|value| !value.is_empty())
}

fn required(name: &str) -> Result<String, String> {
    optional(name).ok_or_else(|| format!("missing `{name}` env var, see .env.example"))
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Ok(Self {
            bind: optional("API_BIND").unwrap_or_else(|| "127.0.0.1:8090".to_owned()),
            database_url: Self::database_url(),
            hypixel_api_key: required("HYPIXEL_API_KEY")?,
        })
    }

    /// Only what the CLI commands need, so creating a key doesn't require the Hypixel key
    pub fn database_url() -> String {
        optional("DATABASE_URL").unwrap_or_else(|| "sqlite://hytools.db".to_owned())
    }
}
