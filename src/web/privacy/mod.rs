//! Data requests: lets the instance's privacy admins, set by `PRIVACY_ADMIN_IDS`, find everything
//! OBot stores about a person and erase it, for the GDPR right of access and to erasure

use askama::Template;
use axum::extract::{Form, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::sync::Arc;

use super::auth::{LoggedIn, PanelLang};
use super::members::date_label;
use super::{AppError, AppState, Shell};
use crate::Error;
use crate::i18n::{Lang, tr};

/// Most people shown for one search
const MAX_PEOPLE: i64 = 20;

/// Discord user IDs separated by commas or spaces. Empty or unset means nobody has access.
pub fn parse_admin_ids(value: Option<&str>) -> Result<Vec<serenity::UserId>, Error> {
    value
        .unwrap_or_default()
        .split([',', ' '])
        .filter(|id| !id.trim().is_empty())
        .map(|id| {
            id.trim()
                .parse::<std::num::NonZeroU64>()
                .map(serenity::UserId::from)
                .map_err(|_| {
                    format!("`PRIVACY_ADMIN_IDS` must list Discord user IDs, got `{id}`").into()
                })
        })
        .collect()
}

fn to_db(id: serenity::UserId) -> i64 {
    id.get() as i64
}

fn user_from_db(id: i64) -> Option<serenity::UserId> {
    std::num::NonZeroU64::new(id as u64).map(serenity::UserId::from)
}

/// People with stored data matching a Discord ID, a Minecraft username or UUID, or a panel name.
/// IDs match exactly, names partially and without case.
async fn matching_people(db: &SqlitePool, query: &str) -> Result<Vec<serenity::UserId>, Error> {
    let query = query.trim();
    let id = query.parse::<u64>().ok().map(|id| id as i64);
    let text = query.to_lowercase();
    let uuid = text.replace('-', "");
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT user_id FROM verified_member
         WHERE user_id = ?1 OR instr(lower(minecraft_name), ?2) > 0
             OR replace(lower(minecraft_uuid), '-', '') = ?3
         UNION SELECT forced_by FROM verified_member WHERE forced_by = ?1
         UNION SELECT user_id FROM session WHERE user_id = ?1 OR instr(lower(name), ?2) > 0
         LIMIT ?4",
    )
    .bind(id)
    .bind(&text)
    .bind(&uuid)
    .bind(MAX_PEOPLE)
    .fetch_all(db)
    .await?;
    Ok(ids.into_iter().filter_map(user_from_db).collect())
}

struct VerificationRow {
    guild: String,
    minecraft_name: String,
    minecraft_uuid: String,
    verified_on: String,
    /// The admin who verified them, `None` when they did it themselves
    by: Option<serenity::UserId>,
}

struct Person {
    id: serenity::UserId,
    name: Option<String>,
    verifications: Vec<VerificationRow>,
    /// Verifications of other members made by this person as an admin
    verified_by_them: i64,
    sessions: i64,
}

/// Everything stored about one person
async fn person(state: &AppState, id: serenity::UserId) -> Result<Person, Error> {
    let rows: Vec<(i64, String, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT guild_id, minecraft_uuid, minecraft_name, verified_at, forced_by
         FROM verified_member WHERE user_id = ? ORDER BY verified_at",
    )
    .bind(to_db(id))
    .fetch_all(&state.db)
    .await?;
    let verified_by_them: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM verified_member WHERE forced_by = ?")
            .bind(to_db(id))
            .fetch_one(&state.db)
            .await?;
    let (sessions, session_name): (i64, Option<String>) =
        sqlx::query_as("SELECT COUNT(*), MAX(name) FROM session WHERE user_id = ?")
            .bind(to_db(id))
            .fetch_one(&state.db)
            .await?;

    let cached_name = state.cache.user(id).map(|user| user.name.clone());
    let name = match cached_name.or(session_name) {
        Some(name) => Some(name),
        None => state.discord.get_user(id).await.ok().map(|user| user.name),
    };
    let verifications = rows
        .into_iter()
        .map(
            |(guild_id, minecraft_uuid, minecraft_name, verified_at, forced_by)| {
                let guild_id = serenity::GuildId::new((guild_id as u64).max(1));
                VerificationRow {
                    guild: state
                        .cache
                        .guild(guild_id)
                        .map_or_else(|| guild_id.to_string(), |guild| guild.name.clone()),
                    minecraft_name,
                    minecraft_uuid,
                    verified_on: date_label(Some(verified_at)),
                    by: forced_by.and_then(user_from_db),
                }
            },
        )
        .collect();
    Ok(Person {
        id,
        name,
        verifications,
        verified_by_them,
        sessions,
    })
}

#[derive(Debug, PartialEq)]
struct Erased {
    verifications: u64,
    sessions: u64,
    /// Verifications of other members that named this person as the admin who made them
    admin_mentions: u64,
    guilds: Vec<serenity::GuildId>,
}

/// Deletes everything stored about a person, in one transaction
async fn erase(db: &SqlitePool, id: serenity::UserId) -> Result<Erased, Error> {
    let mut transaction = db.begin().await?;
    let guilds: Vec<i64> =
        sqlx::query_scalar("SELECT guild_id FROM verified_member WHERE user_id = ?")
            .bind(to_db(id))
            .fetch_all(&mut *transaction)
            .await?;
    let verifications = sqlx::query("DELETE FROM verified_member WHERE user_id = ?")
        .bind(to_db(id))
        .execute(&mut *transaction)
        .await?
        .rows_affected();
    // The verification stays with the member it belongs to, only the admin's ID goes
    let admin_mentions =
        sqlx::query("UPDATE verified_member SET forced_by = NULL WHERE forced_by = ?")
            .bind(to_db(id))
            .execute(&mut *transaction)
            .await?
            .rows_affected();
    let sessions = sqlx::query("DELETE FROM session WHERE user_id = ?")
        .bind(to_db(id))
        .execute(&mut *transaction)
        .await?
        .rows_affected();
    transaction.commit().await?;
    Ok(Erased {
        verifications,
        sessions,
        admin_mentions,
        guilds: guilds
            .into_iter()
            .filter_map(|id| std::num::NonZeroU64::new(id as u64).map(serenity::GuildId::from))
            .collect(),
    })
}

/// The search, and after an erasure what was erased
#[derive(Deserialize)]
pub struct SearchParams {
    #[serde(default)]
    q: String,
    erased: Option<u64>,
    #[serde(default)]
    verifications: u64,
    #[serde(default)]
    sessions: u64,
    #[serde(default)]
    admin: u64,
}

#[derive(Template)]
#[template(path = "data_requests.html")]
struct DataRequestsPage {
    lang: Lang,
    shell: Shell,
    query: String,
    /// `None` before searching
    people: Option<Vec<Person>>,
    /// What was just erased
    erased: Option<String>,
}

impl AppState {
    fn is_privacy_admin(&self, user_id: serenity::UserId) -> bool {
        self.privacy_admins.contains(&user_id)
    }
}

pub async fn page(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    PanelLang(lang): PanelLang,
    Query(params): Query<SearchParams>,
) -> Result<Response, AppError> {
    // Like any unknown page, so the page doesn't reveal it exists
    if !state.is_privacy_admin(user.id) {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    let query = params.q.trim().to_owned();
    let people = if query.is_empty() {
        None
    } else {
        let mut people = Vec::new();
        for id in matching_people(&state.db, &query).await? {
            people.push(person(&state, id).await?);
        }
        Some(people)
    };
    let page = DataRequestsPage {
        lang,
        shell: state.shell(user, None).await,
        query,
        people,
        erased: params.erased.map(|user| {
            tr!(
                lang,
                "notice-data-erased",
                user = user.to_string(),
                verifications = params.verifications,
                sessions = params.sessions,
                admin = params.admin
            )
        }),
    };
    Ok(Html(page.render()?).into_response())
}

#[derive(Deserialize)]
pub struct EraseForm {
    user_id: u64,
    /// The search to go back to
    #[serde(default)]
    q: String,
}

pub async fn erase_person(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Form(form): Form<EraseForm>,
) -> Result<Response, AppError> {
    if !state.is_privacy_admin(user.id) {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }
    let Some(target) = std::num::NonZeroU64::new(form.user_id).map(serenity::UserId::from) else {
        return Ok(Redirect::to("/data").into_response());
    };
    let erased = erase(&state.db, target).await?;
    for guild_id in &erased.guilds {
        state.guild_members.remove(guild_id);
    }
    // Kept in the bot's logs, to account for how the request was handled
    println!(
        "Data request: {} erased the data of {target}: {} verifications, {} sessions, {} admin mentions",
        user.id, erased.verifications, erased.sessions, erased.admin_mentions
    );
    let back = reqwest::Url::parse_with_params(
        "http://panel/data",
        &[
            ("q", form.q),
            ("erased", target.to_string()),
            ("verifications", erased.verifications.to_string()),
            ("sessions", erased.sessions.to_string()),
            ("admin", erased.admin_mentions.to_string()),
        ],
    )?;
    Ok(Redirect::to(&format!("/data?{}", back.query().unwrap_or_default())).into_response())
}

#[cfg(test)]
mod tests;
