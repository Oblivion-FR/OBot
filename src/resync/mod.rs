//! Refreshes every verified member on a schedule, so ranks, guild changes and Minecraft names
//! reach Discord without members running `/verify` again

use poise::serenity_prelude as serenity;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{Instant, MissedTickBehavior};

use crate::Error;
use crate::config;
use crate::hypixel::Hypixel;
use crate::verification::{self, Services};

/// Hours between two refreshes when `RESYNC_INTERVAL_HOURS` isn't set
const DEFAULT_HOURS: u64 = 3;

/// The interval set by `RESYNC_INTERVAL_HOURS`, `None` when set to 0 to turn refreshes off
pub fn interval(hours: Option<String>) -> Result<Option<Duration>, Error> {
    let hours = match hours {
        None => DEFAULT_HOURS,
        Some(hours) => hours.trim().parse().map_err(|_| {
            format!("`RESYNC_INTERVAL_HOURS` must be a whole number of hours, got `{hours}`")
        })?,
    };
    Ok((hours > 0).then(|| Duration::from_secs(hours * 3600)))
}

/// What the refresh needs, owned so it can run in its own task
pub struct Resync {
    pub db: sqlx::SqlitePool,
    pub hypixel: Arc<Hypixel>,
    pub mojang: reqwest::Client,
    pub discord: Arc<serenity::Http>,
    pub cache: Arc<serenity::Cache>,
}

#[derive(Default)]
struct Summary {
    servers: usize,
    members: usize,
    updated: usize,
    /// Left the server, deleted their Minecraft account, or the server stopped verifying
    skipped: usize,
    failed: usize,
}

impl Resync {
    /// Refreshes everyone once per interval. The first run waits a full interval, so frequent
    /// redeploys don't refresh everyone each time.
    pub async fn run(self, every: Duration) {
        let mut ticks = tokio::time::interval_at(Instant::now() + every, every);
        // A run slower than the interval pushes the next one back instead of stacking them
        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticks.tick().await;
            let started = Instant::now();
            let summary = self.refresh_all().await;
            println!(
                "Refreshed {} verified members in {} servers: {} updated, {} skipped, {} failed, in {}s",
                summary.members,
                summary.servers,
                summary.updated,
                summary.skipped,
                summary.failed,
                started.elapsed().as_secs()
            );
        }
    }

    async fn refresh_all(&self) -> Summary {
        let mut summary = Summary::default();
        let guilds = match config::list_verifying_guilds(&self.db).await {
            Ok(guilds) => guilds,
            Err(error) => {
                eprintln!("Refresh: could not list servers: {error}");
                return summary;
            }
        };
        for guild_id in guilds {
            // The bot may have been removed from a server that still has settings
            if self.cache.guild(guild_id).is_none() {
                continue;
            }
            summary.servers += 1;
            if let Err(error) = self.refresh_guild(guild_id, &mut summary).await {
                eprintln!("Refresh of server {guild_id} stopped: {error}");
            }
        }
        summary
    }

    async fn refresh_guild(
        &self,
        guild_id: serenity::GuildId,
        summary: &mut Summary,
    ) -> Result<(), Error> {
        let services = Services {
            db: &self.db,
            hypixel: &self.hypixel,
            mojang: &self.mojang,
            discord: &self.discord,
            cache: &self.cache,
        };
        let config = config::get_config(&self.db, guild_id).await?;
        let members = config::list_verified_members(&self.db, guild_id).await?;
        for (user_id, stored) in members {
            summary.members += 1;
            // Members who left keep their record, in case they come back
            let Ok(member) = self.discord.get_member(guild_id, user_id).await else {
                summary.skipped += 1;
                continue;
            };
            match verification::refresh_member(&services, guild_id, &member, &config, &stored).await
            {
                Ok(Ok((_, outcome))) if outcome.changed() => summary.updated += 1,
                Ok(Ok(_)) => {}
                Ok(Err(_)) => summary.skipped += 1,
                Err(error) => {
                    summary.failed += 1;
                    eprintln!("Refresh of {user_id} in server {guild_id} failed: {error}");
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
