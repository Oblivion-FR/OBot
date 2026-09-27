use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;

use crate::Error;
use crate::i18n::Lang;
use crate::nickname::{self, CustomText, NicknameFormat, Segment, ValueText};

pub async fn connect(url: &str) -> Result<SqlitePool, Error> {
    let options = SqliteConnectOptions::from_str(url)?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

/// A fresh database for tests. One connection that never closes: each connection to
/// `:memory:` gets its own empty database.
#[cfg(test)]
pub async fn connect_in_memory() -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory database opens");
    sqlx::migrate!().run(&pool).await.expect("migrations apply");
    pool
}

// SQLite has no unsigned integers, Discord IDs fit in an i64 bit for bit
fn to_db(id: impl Into<u64>) -> i64 {
    id.into() as i64
}

fn role_from_db(id: i64) -> Option<serenity::RoleId> {
    std::num::NonZeroU64::new(id as u64).map(serenity::RoleId::from)
}

pub struct HypixelGuildLink {
    pub id: String,
    pub name: String,
}

#[derive(Default)]
pub struct GuildConfig {
    pub verified_role_id: Option<serenity::RoleId>,
    pub unverified_role_id: Option<serenity::RoleId>,
    pub hypixel_guild: Option<HypixelGuildLink>,
    /// Language of what OBot posts in the server
    pub language: Lang,
    pub log_channel_id: Option<serenity::ChannelId>,
}

#[derive(sqlx::FromRow)]
struct GuildConfigRow {
    verified_role_id: Option<i64>,
    unverified_role_id: Option<i64>,
    hypixel_guild_id: Option<String>,
    hypixel_guild_name: Option<String>,
    language: Option<String>,
    log_channel_id: Option<i64>,
}

pub async fn get_config(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
) -> Result<GuildConfig, Error> {
    let row: Option<GuildConfigRow> = sqlx::query_as(
        "SELECT verified_role_id, unverified_role_id, hypixel_guild_id, hypixel_guild_name,
                language, log_channel_id
         FROM guild_config WHERE guild_id = ?",
    )
    .bind(to_db(guild_id))
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Ok(GuildConfig::default());
    };
    Ok(GuildConfig {
        verified_role_id: row.verified_role_id.and_then(role_from_db),
        unverified_role_id: row.unverified_role_id.and_then(role_from_db),
        hypixel_guild: row
            .hypixel_guild_id
            .zip(row.hypixel_guild_name)
            .map(|(id, name)| HypixelGuildLink { id, name }),
        language: row
            .language
            .as_deref()
            .and_then(Lang::from_tag)
            .unwrap_or_default(),
        log_channel_id: row
            .log_channel_id
            .and_then(|id| std::num::NonZeroU64::new(id as u64))
            .map(serenity::ChannelId::from),
    })
}

pub async fn set_messages(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
    language: Lang,
    log_channel_id: Option<serenity::ChannelId>,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, language, log_channel_id) VALUES (?, ?, ?)
         ON CONFLICT (guild_id) DO UPDATE SET
             language = excluded.language,
             log_channel_id = excluded.log_channel_id",
    )
    .bind(to_db(guild_id))
    .bind(language.code())
    .bind(log_channel_id.map(to_db))
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_roles(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
    verified: Option<serenity::RoleId>,
    unverified: Option<serenity::RoleId>,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO guild_config (guild_id, verified_role_id, unverified_role_id) VALUES (?, ?, ?)
         ON CONFLICT (guild_id) DO UPDATE SET
             verified_role_id = excluded.verified_role_id,
             unverified_role_id = excluded.unverified_role_id",
    )
    .bind(to_db(guild_id))
    .bind(verified.map(to_db))
    .bind(unverified.map(to_db))
    .execute(db)
    .await?;
    Ok(())
}

pub async fn set_hypixel_guild(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
    link: Option<HypixelGuildLink>,
) -> Result<(), Error> {
    let (id, name) = link.map(|link| (link.id, link.name)).unzip();
    sqlx::query(
        "INSERT INTO guild_config (guild_id, hypixel_guild_id, hypixel_guild_name) VALUES (?, ?, ?)
         ON CONFLICT (guild_id) DO UPDATE SET
             hypixel_guild_id = excluded.hypixel_guild_id,
             hypixel_guild_name = excluded.hypixel_guild_name",
    )
    .bind(to_db(guild_id))
    .bind(id)
    .bind(name)
    .execute(db)
    .await?;
    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
pub enum RuleKind {
    /// `value` is a key of `hypixel::RANKS`
    HypixelRank,
    /// Member of the linked Hypixel guild, `value` is unused
    GuildMember,
    /// `value` is a rank name in the linked Hypixel guild
    GuildRank,
}

impl RuleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HypixelRank => "hypixel_rank",
            Self::GuildMember => "guild_member",
            Self::GuildRank => "guild_rank",
        }
    }

    pub fn parse(kind: &str) -> Option<Self> {
        [Self::HypixelRank, Self::GuildMember, Self::GuildRank]
            .into_iter()
            .find(|candidate| candidate.as_str() == kind)
    }
}

pub struct Rule {
    pub id: i64,
    pub kind: RuleKind,
    pub value: String,
    pub role_id: serenity::RoleId,
    pub group_id: Option<i64>,
}

pub async fn list_rules(db: &SqlitePool, guild_id: serenity::GuildId) -> Result<Vec<Rule>, Error> {
    let rows: Vec<(i64, String, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT id, kind, value, role_id, group_id FROM role_rule WHERE guild_id = ? ORDER BY id",
    )
    .bind(to_db(guild_id))
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(id, kind, value, role_id, group_id)| {
            Some(Rule {
                id,
                kind: RuleKind::parse(&kind)?,
                value,
                role_id: role_from_db(role_id)?,
                group_id,
            })
        })
        .collect())
}

pub async fn add_rule(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
    kind: RuleKind,
    value: &str,
    role_id: serenity::RoleId,
    group_id: Option<i64>,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO role_rule (guild_id, kind, value, role_id, group_id) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(to_db(guild_id))
    .bind(kind.as_str())
    .bind(value)
    .bind(to_db(role_id))
    .bind(group_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete_rule(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
    rule_id: i64,
) -> Result<(), Error> {
    // Scoped by guild so a panel user can only delete rules of a guild they manage
    sqlx::query("DELETE FROM role_rule WHERE id = ? AND guild_id = ?")
        .bind(rule_id)
        .bind(to_db(guild_id))
        .execute(db)
        .await?;
    Ok(())
}

/// Rules grouped under a separator role, like `━━ Ranks ━━` above the rank roles
pub struct RuleGroup {
    pub id: i64,
    pub name: String,
    pub separator_role_id: serenity::RoleId,
}

pub async fn list_groups(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
) -> Result<Vec<RuleGroup>, Error> {
    let rows: Vec<(i64, String, i64)> = sqlx::query_as(
        "SELECT id, name, separator_role_id FROM rule_group WHERE guild_id = ? ORDER BY id",
    )
    .bind(to_db(guild_id))
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(id, name, separator_role_id)| {
            Some(RuleGroup {
                id,
                name,
                separator_role_id: role_from_db(separator_role_id)?,
            })
        })
        .collect())
}

pub async fn add_group(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
    name: &str,
    separator_role_id: serenity::RoleId,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO rule_group (guild_id, name, separator_role_id) VALUES (?, ?, ?)")
        .bind(to_db(guild_id))
        .bind(name)
        .bind(to_db(separator_role_id))
        .execute(db)
        .await?;
    Ok(())
}

/// Deletes a group, its rules stay without a group
pub async fn delete_group(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
    group_id: i64,
) -> Result<(), Error> {
    let mut transaction = db.begin().await?;
    // Scoped by guild so a panel user can only change groups of a guild they manage
    sqlx::query("UPDATE role_rule SET group_id = NULL WHERE group_id = ? AND guild_id = ?")
        .bind(group_id)
        .bind(to_db(guild_id))
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM rule_group WHERE id = ? AND guild_id = ?")
        .bind(group_id)
        .bind(to_db(guild_id))
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(())
}

/// `(field, value, prefix, label, suffix)` of `nickname_value_text`
type ValueTextRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub async fn get_nickname_format(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
) -> Result<NicknameFormat, Error> {
    let mut format = NicknameFormat::default();
    let settings: Option<(bool, String)> = sqlx::query_as(
        "SELECT nickname_enabled, nickname_separator FROM guild_config WHERE guild_id = ?",
    )
    .bind(to_db(guild_id))
    .fetch_optional(db)
    .await?;
    if let Some((enabled, separator)) = settings {
        format.enabled = enabled;
        format.separator = separator;
    }

    let rows: Vec<(String, bool, String, String, u8)> = sqlx::query_as(
        "SELECT field, enabled, prefix, suffix, importance FROM nickname_segment
         WHERE guild_id = ? ORDER BY position",
    )
    .bind(to_db(guild_id))
    .fetch_all(db)
    .await?;
    let mut segments: Vec<Segment> = rows
        .into_iter()
        .filter_map(|(field, enabled, prefix, suffix, importance)| {
            Some(Segment {
                field: nickname::Field::parse(&field)?,
                enabled,
                prefix,
                suffix,
                importance,
            })
        })
        .collect();
    // Fields never saved keep their default, at the end
    for default in &format.segments {
        if !segments
            .iter()
            .any(|segment| segment.field == default.field)
        {
            segments.push(default.clone());
        }
    }
    format.segments = segments;

    let texts: Vec<ValueTextRow> = sqlx::query_as(
        "SELECT field, value, prefix, label, suffix FROM nickname_value_text WHERE guild_id = ?",
    )
    .bind(to_db(guild_id))
    .fetch_all(db)
    .await?;
    format.custom_texts = texts
        .into_iter()
        .filter_map(|(field, value, prefix, label, suffix)| {
            Some(CustomText {
                field: nickname::Field::parse(&field)?,
                value,
                text: ValueText {
                    prefix,
                    label,
                    suffix,
                },
            })
        })
        .collect();
    Ok(format)
}

pub async fn set_nickname_format(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
    format: &NicknameFormat,
) -> Result<(), Error> {
    let mut transaction = db.begin().await?;
    sqlx::query(
        "INSERT INTO guild_config (guild_id, nickname_enabled, nickname_separator) VALUES (?, ?, ?)
         ON CONFLICT (guild_id) DO UPDATE SET
             nickname_enabled = excluded.nickname_enabled,
             nickname_separator = excluded.nickname_separator",
    )
    .bind(to_db(guild_id))
    .bind(format.enabled)
    .bind(&format.separator)
    .execute(&mut *transaction)
    .await?;

    sqlx::query("DELETE FROM nickname_segment WHERE guild_id = ?")
        .bind(to_db(guild_id))
        .execute(&mut *transaction)
        .await?;
    for (position, segment) in format.segments.iter().enumerate() {
        sqlx::query(
            "INSERT INTO nickname_segment
                 (guild_id, field, position, enabled, prefix, suffix, importance)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(to_db(guild_id))
        .bind(segment.field.as_str())
        .bind(position as i64)
        .bind(segment.enabled)
        .bind(&segment.prefix)
        .bind(&segment.suffix)
        .bind(segment.importance)
        .execute(&mut *transaction)
        .await?;
    }

    sqlx::query("DELETE FROM nickname_value_text WHERE guild_id = ?")
        .bind(to_db(guild_id))
        .execute(&mut *transaction)
        .await?;
    for custom in &format.custom_texts {
        if custom.text.is_default() {
            continue;
        }
        sqlx::query(
            "INSERT INTO nickname_value_text (guild_id, field, value, prefix, label, suffix)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT (guild_id, field, value) DO NOTHING",
        )
        .bind(to_db(guild_id))
        .bind(custom.field.as_str())
        .bind(&custom.value)
        .bind(&custom.text.prefix)
        .bind(&custom.text.label)
        .bind(&custom.text.suffix)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(())
}

pub struct VerifiedMember {
    pub minecraft_uuid: String,
    pub minecraft_name: String,
    /// Unix time in seconds of the last verification
    pub verified_at: i64,
    pub forced_by: Option<serenity::UserId>,
}

type VerifiedMemberRow = (i64, String, String, i64, Option<i64>);

fn user_from_db(id: i64) -> Option<serenity::UserId> {
    std::num::NonZeroU64::new(id as u64).map(serenity::UserId::from)
}

fn verified_member_from_row(row: VerifiedMemberRow) -> Option<(serenity::UserId, VerifiedMember)> {
    let (user_id, minecraft_uuid, minecraft_name, verified_at, forced_by) = row;
    Some((
        user_from_db(user_id)?,
        VerifiedMember {
            minecraft_uuid,
            minecraft_name,
            verified_at,
            forced_by: forced_by.and_then(user_from_db),
        },
    ))
}

pub async fn record_verification(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    minecraft_uuid: &str,
    minecraft_name: &str,
    forced_by: Option<serenity::UserId>,
) -> Result<(), Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64);
    sqlx::query(
        "INSERT INTO verified_member
             (guild_id, user_id, minecraft_uuid, minecraft_name, verified_at, forced_by)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT (guild_id, user_id) DO UPDATE SET
             minecraft_uuid = excluded.minecraft_uuid,
             minecraft_name = excluded.minecraft_name,
             verified_at = excluded.verified_at,
             forced_by = excluded.forced_by",
    )
    .bind(to_db(guild_id))
    .bind(to_db(user_id))
    .bind(minecraft_uuid)
    .bind(minecraft_name)
    .bind(now)
    .bind(forced_by.map(to_db))
    .execute(db)
    .await?;
    Ok(())
}

/// Follows a name change of a verified member's account, keeping when and by whom they were
/// verified
pub async fn update_minecraft_name(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    minecraft_name: &str,
) -> Result<(), Error> {
    sqlx::query("UPDATE verified_member SET minecraft_name = ? WHERE guild_id = ? AND user_id = ?")
        .bind(minecraft_name)
        .bind(to_db(guild_id))
        .bind(to_db(user_id))
        .execute(db)
        .await?;
    Ok(())
}

/// Servers where verification is set up
pub async fn list_verifying_guilds(db: &SqlitePool) -> Result<Vec<serenity::GuildId>, Error> {
    let ids: Vec<i64> =
        sqlx::query_scalar("SELECT guild_id FROM guild_config WHERE verified_role_id IS NOT NULL")
            .fetch_all(db)
            .await?;
    Ok(ids
        .into_iter()
        .filter_map(|id| std::num::NonZeroU64::new(id as u64).map(serenity::GuildId::from))
        .collect())
}

pub async fn get_verified_member(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
) -> Result<Option<VerifiedMember>, Error> {
    let row: Option<VerifiedMemberRow> = sqlx::query_as(
        "SELECT user_id, minecraft_uuid, minecraft_name, verified_at, forced_by
         FROM verified_member
         WHERE guild_id = ? AND user_id = ?",
    )
    .bind(to_db(guild_id))
    .bind(to_db(user_id))
    .fetch_optional(db)
    .await?;
    Ok(row
        .and_then(verified_member_from_row)
        .map(|(_, member)| member))
}

pub async fn list_verified_members(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
) -> Result<std::collections::HashMap<serenity::UserId, VerifiedMember>, Error> {
    let rows: Vec<VerifiedMemberRow> = sqlx::query_as(
        "SELECT user_id, minecraft_uuid, minecraft_name, verified_at, forced_by
         FROM verified_member
         WHERE guild_id = ?",
    )
    .bind(to_db(guild_id))
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(verified_member_from_row)
        .collect())
}

/// The member linked to a Minecraft account in this guild
pub async fn find_account_owner(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
    minecraft_uuid: &str,
) -> Result<Option<serenity::UserId>, Error> {
    let user_id: Option<i64> = sqlx::query_scalar(
        "SELECT user_id FROM verified_member WHERE guild_id = ? AND minecraft_uuid = ?",
    )
    .bind(to_db(guild_id))
    .bind(minecraft_uuid)
    .fetch_optional(db)
    .await?;
    Ok(user_id.and_then(user_from_db))
}

pub async fn delete_verification(
    db: &SqlitePool,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
) -> Result<(), Error> {
    sqlx::query("DELETE FROM verified_member WHERE guild_id = ? AND user_id = ?")
        .bind(to_db(guild_id))
        .bind(to_db(user_id))
        .execute(db)
        .await?;
    Ok(())
}
