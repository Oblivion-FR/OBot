use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::AppState;
use crate::error::ApiError;

/// Keys start with this, so a leaked one is easy to recognize in logs or secret scanners
const KEY_PREFIX: &str = "hyt_";

/// The service that sent a request, added to every authenticated request. Per key rate limits
/// will hook here.
#[derive(Clone, Debug)]
pub struct ApiClient {
    pub name: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hash(key: &str) -> String {
    hex(&Sha256::digest(key.as_bytes()))
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

/// Creates a key for a service and returns it. It's only shown this once: the database keeps
/// its hash.
pub async fn create_key(db: &SqlitePool, name: &str) -> Result<String, sqlx::Error> {
    let bytes: [u8; 32] = rand::random();
    let key = format!("{KEY_PREFIX}{}", hex(&bytes));
    sqlx::query("INSERT INTO api_key (name, key_hash, created_at) VALUES (?, ?, ?)")
        .bind(name)
        .bind(hash(&key))
        .bind(now())
        .execute(db)
        .await?;
    Ok(key)
}

/// The service a key belongs to, `None` if the key is unknown or revoked
pub async fn find_client(db: &SqlitePool, key: &str) -> Result<Option<ApiClient>, sqlx::Error> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT name FROM api_key WHERE key_hash = ? AND revoked_at IS NULL")
            .bind(hash(key))
            .fetch_optional(db)
            .await?;
    Ok(row.map(|(name,)| ApiClient { name }))
}

/// Rejects requests without a valid `Authorization: Bearer <key>` header
pub async fn require_key(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let key = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|key| key.starts_with(KEY_PREFIX))
        .ok_or(ApiError::Unauthorized)?;
    let client = find_client(&state.db, key)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    tracing::Span::current().record("client", client.name.as_str());
    request.extensions_mut().insert(client);
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests;
