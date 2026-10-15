use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// Body of every error response. Frontends translate `code`; `message` is English, for logs and
/// the API docs only.
#[derive(Serialize, ToSchema)]
pub struct ErrorBody {
    /// Stable machine code, like `hypixel_player_not_found`
    #[schema(example = "hypixel_player_not_found")]
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{message}")]
    BadRequest { code: &'static str, message: String },
    #[error("missing or invalid API key")]
    Unauthorized,
    #[error("{message}")]
    NotFound { code: &'static str, message: String },
    /// Hypixel, Mojang or Discord failed or is unreachable
    #[error("{service} request failed: {source}")]
    Upstream {
        service: &'static str,
        source: reqwest::Error,
    },
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl ApiError {
    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::BadRequest {
            code,
            message: message.into(),
        }
    }

    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::NotFound {
            code,
            message: message.into(),
        }
    }

    fn parts(&self) -> (StatusCode, &'static str) {
        match self {
            Self::BadRequest { code, .. } => (StatusCode::BAD_REQUEST, code),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::NotFound { code, .. } => (StatusCode::NOT_FOUND, code),
            Self::Upstream { service, .. } => (
                StatusCode::BAD_GATEWAY,
                match *service {
                    "hypixel" => "hypixel_unavailable",
                    "mojang" => "mojang_unavailable",
                    _ => "upstream_unavailable",
                },
            ),
            Self::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = self.parts();
        let message = if status.is_server_error() {
            // The details stay in the logs: they can name internal hosts or queries
            tracing::error!(error = %self, "request failed");
            match self {
                Self::Upstream { service, .. } => format!("{service} is unavailable"),
                _ => "internal error".to_owned(),
            }
        } else {
            self.to_string()
        };
        (status, Json(ErrorBody { code, message })).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
