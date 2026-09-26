use axum::extract::{FromRequestParts, Query, State};
use axum::http::request::Parts;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::{AppError, AppState};

const SESSION_COOKIE: &str = "obot_session";
const STATE_COOKIE: &str = "obot_oauth_state";
const SESSION_TTL: Duration = Duration::from_secs(7 * 24 * 3600);

#[derive(Clone)]
pub struct User {
    pub id: serenity::UserId,
    pub name: String,
}

struct Session {
    user: User,
    expires_at: Instant,
}

/// In memory, so a restart logs everyone out
#[derive(Default)]
pub struct Sessions(Mutex<HashMap<String, Session>>);

impl Sessions {
    fn create(&self, user: User) -> String {
        let token = random_token();
        let mut sessions = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        sessions.retain(|_, session| session.expires_at > now);
        sessions.insert(
            token.clone(),
            Session {
                user,
                expires_at: now + SESSION_TTL,
            },
        );
        token
    }

    fn get(&self, token: &str) -> Option<User> {
        let sessions = self.0.lock().unwrap_or_else(|e| e.into_inner());
        sessions
            .get(token)
            .filter(|session| session.expires_at > Instant::now())
            .map(|session| session.user.clone())
    }

    fn remove(&self, token: &str) {
        let mut sessions = self.0.lock().unwrap_or_else(|e| e.into_inner());
        sessions.remove(token);
    }
}

fn random_token() -> String {
    let bytes: [u8; 32] = rand::random();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
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

pub struct MaybeLoggedIn(pub Option<User>);

impl FromRequestParts<Arc<AppState>> for MaybeLoggedIn {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let user = jar
            .get(SESSION_COOKIE)
            .and_then(|cookie| state.sessions.get(cookie.value()));
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

    let session = state.sessions.create(User {
        id: discord_user.id,
        name: discord_user.global_name.unwrap_or(discord_user.username),
    });
    let jar = jar.add(oauth.cookie(SESSION_COOKIE, session));
    Ok((jar, Redirect::to("/")).into_response())
}

pub async fn logout(State(state): State<Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        state.sessions.remove(cookie.value());
    }
    (
        jar.remove(Cookie::build(SESSION_COOKIE).path("/")),
        Redirect::to("/"),
    )
}
