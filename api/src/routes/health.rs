use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(health))
}

#[derive(Serialize, ToSchema)]
pub struct Health {
    #[schema(example = "ok")]
    status: &'static str,
    /// Version of the API build
    #[schema(example = "0.1.0")]
    version: &'static str,
}

/// Whether the API is up. The only route without an API key.
#[utoipa::path(get, path = "/health", tag = "health", responses((status = OK, body = Health)))]
async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}
