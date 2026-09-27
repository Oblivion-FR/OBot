use axum::extract::{Form, FromRequestParts, Query, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{AppError, AppState};
use crate::Error;
use crate::i18n::Lang;

const SESSION_COOKIE: &str = "obot_session";
const STATE_COOKIE: &str = "obot_oauth_state";
/// The language picked in the panel
const LANG_COOKIE: &str = "obot_lang";
const SESSION_TTL: Duration = Duration::from_secs(7 * 24 * 3600);

#[derive(Clone)]
pub struct User {
    pub id: serenity::UserId,
    pub name: String,
    pub avatar_url: String,
    /// Language of the Discord account, when OBot speaks it
    pub lang: Option<Lang>,
}

/// Panel logins, in the database so restarts and updates don't log everyone out
pub struct Sessions(pub SqlitePool);

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Sessions are stored and looked up by this, never by the token itself
fn token_hash(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}

impl Sessions {
    async fn create(&self, user: &User) -> Result<String, Error> {
        let token = random_token();
        let now = unix_now();
        sqlx::query("DELETE FROM session WHERE expires_at <= ?")
            .bind(now)
            .execute(&self.0)
            .await?;
        sqlx::query(
            "INSERT INTO session (token_hash, user_id, name, avatar_url, lang, expires_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(token_hash(&token))
        .bind(user.id.get() as i64)
        .bind(&user.name)
        .bind(&user.avatar_url)
        .bind(user.lang.map(Lang::code))
        .bind(now + SESSION_TTL.as_secs() as i64)
        .execute(&self.0)
        .await?;
        Ok(token)
    }

    /// A database error counts as logged out, the panel shows its login page
    async fn get(&self, token: &str) -> Option<User> {
        let row: Option<(i64, String, String, Option<String>)> = sqlx::query_as(
            "SELECT user_id, name, avatar_url, lang FROM session
             WHERE token_hash = ? AND expires_at > ?",
        )
        .bind(token_hash(token))
        .bind(unix_now())
        .fetch_optional(&self.0)
        .await
        .inspect_err(|error| eprintln!("Could not read a session: {error}"))
        .ok()?;
        let (user_id, name, avatar_url, lang) = row?;
        Some(User {
            id: std::num::NonZeroU64::new(user_id as u64)?.into(),
            name,
            avatar_url,
            lang: lang.as_deref().and_then(Lang::from_tag),
        })
    }

    async fn remove(&self, token: &str) -> Result<(), Error> {
        sqlx::query("DELETE FROM session WHERE token_hash = ?")
            .bind(token_hash(token))
            .execute(&self.0)
            .await?;
        Ok(())
    }
}

fn random_token() -> String {
    let bytes: [u8; 32] = rand::random();
    hex(&bytes)
}

pub struct OAuthConfig {
    pub client_id: serenity::ApplicationId,
    pub client_secret: String,
    /// Public URL of the panel, without trailing slash
    pub public_url: String,
}

impl OAuthConfig {
    fn redirect_uri(&self) -> String {
        format!("{}/callback", self.public_url)
    }

    fn cookie<'a>(&self, name: &'a str, value: String) -> Cookie<'a> {
        Cookie::build((name, value))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(self.public_url.starts_with("https://"))
            .build()
    }
}

/// Logged in panel user, redirects to the home page otherwise
pub struct LoggedIn(pub User);

impl FromRequestParts<Arc<AppState>> for LoggedIn {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        MaybeLoggedIn::from_request_parts(parts, state)
            .await?
            .0
            .map(LoggedIn)
            .ok_or_else(|| Redirect::to("/").into_response())
    }
}

/// The panel's language: the one picked in the panel, else the Discord account's, else the
/// browser's, else English
pub struct PanelLang(pub Lang);

impl FromRequestParts<Arc<AppState>> for PanelLang {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let picked = jar
            .get(LANG_COOKIE)
            .and_then(|cookie| Lang::from_tag(cookie.value()));
        // Only read the session when the language wasn't picked
        let account = match (picked, jar.get(SESSION_COOKIE)) {
            (None, Some(cookie)) => state
                .sessions
                .get(cookie.value())
                .await
                .and_then(|user| user.lang),
            _ => None,
        };
        let browser = || {
            parts
                .headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(',').find_map(Lang::from_tag))
        };
        Ok(PanelLang(
            picked.or(account).or_else(browser).unwrap_or_default(),
        ))
    }
}

#[derive(Deserialize)]
pub struct LangForm {
    lang: String,
}

/// Remembers the picked language and goes back to the page it was picked on
pub async fn set_lang(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    headers: HeaderMap,
    Form(form): Form<LangForm>,
) -> impl IntoResponse {
    // Only back to a page of the panel, never to another site
    let back = headers
        .get(header::REFERER)
        .and_then(|referer| referer.to_str().ok())
        .and_then(|referer| referer.strip_prefix(state.oauth.public_url.as_str()))
        .filter(|path| path.starts_with('/') && !path.starts_with("//"))
        .unwrap_or("/")
        .to_owned();
    let jar = match Lang::from_tag(&form.lang) {
        Some(lang) => {
            let mut cookie = state.oauth.cookie(LANG_COOKIE, lang.code().to_owned());
            cookie.set_max_age(cookie::time::Duration::days(365));
            jar.add(cookie)
        }
        None => jar,
    };
    (jar, Redirect::to(&back))
}

pub struct MaybeLoggedIn(pub Option<User>);

impl FromRequestParts<Arc<AppState>> for MaybeLoggedIn {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let user = match jar.get(SESSION_COOKIE) {
            Some(cookie) => state.sessions.get(cookie.value()).await,
            None => None,
        };
        Ok(MaybeLoggedIn(user))
    }
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let csrf_state = random_token();
    let oauth = &state.oauth;
    let url = reqwest::Url::parse_with_params(
        "https://discord.com/oauth2/authorize",
        &[
            ("client_id", oauth.client_id.to_string()),
            ("response_type", "code".to_owned()),
            ("scope", "identify".to_owned()),
            ("redirect_uri", oauth.redirect_uri()),
            ("state", csrf_state.clone()),
        ],
    )?;
    Ok((
        jar.add(oauth.cookie(STATE_COOKIE, csrf_state)),
        Redirect::to(url.as_str()),
    ))
}

#[derive(Deserialize)]
pub struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct DiscordUser {
    id: serenity::UserId,
    username: String,
    global_name: Option<String>,
    /// The Discord account's language, like `fr` or `en-US`
    locale: Option<String>,
    avatar: Option<String>,
}

impl DiscordUser {
    fn avatar_url(&self) -> String {
        match &self.avatar {
            Some(hash) => format!(
                "https://cdn.discordapp.com/avatars/{}/{hash}.png?size=64",
                self.id
            ),
            // Same default avatar Discord shows for users without one
            None => format!(
                "https://cdn.discordapp.com/embed/avatars/{}.png",
                (self.id.get() >> 22) % 6
            ),
        }
    }
}

pub async fn callback(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Query(params): Query<CallbackParams>,
) -> Result<Response, AppError> {
    let expected_state = jar
        .get(STATE_COOKIE)
        .map(|cookie| cookie.value().to_owned());
    let jar = jar.remove(Cookie::build(STATE_COOKIE).path("/"));
    // `code` is missing when the user cancels the Discord consent screen
    let (Some(code), Some(received_state)) = (params.code, params.state) else {
        return Ok((jar, Redirect::to("/")).into_response());
    };
    if expected_state.as_deref() != Some(received_state.as_str()) {
        return Ok((jar, Redirect::to("/")).into_response());
    }

    let oauth = &state.oauth;
    let token: TokenResponse = state
        .http_client
        .post("https://discord.com/api/v10/oauth2/token")
        .form(&[
            ("client_id", oauth.client_id.to_string()),
            ("client_secret", oauth.client_secret.clone()),
            ("grant_type", "authorization_code".to_owned()),
            ("code", code),
            ("redirect_uri", oauth.redirect_uri()),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let discord_user: DiscordUser = state
        .http_client
        .get("https://discord.com/api/v10/users/@me")
        .bearer_auth(token.access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let session = state
        .sessions
        .create(&User {
            id: discord_user.id,
            avatar_url: discord_user.avatar_url(),
            lang: discord_user.locale.as_deref().and_then(Lang::from_tag),
            name: discord_user.global_name.unwrap_or(discord_user.username),
        })
        .await?;
    let jar = jar.add(oauth.cookie(SESSION_COOKIE, session));
    Ok((jar, Redirect::to("/")).into_response())
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        state.sessions.remove(cookie.value()).await?;
    }
    Ok((
        jar.remove(Cookie::build(SESSION_COOKIE).path("/")),
        Redirect::to("/"),
    ))
}

#[cfg(test)]
mod tests;
