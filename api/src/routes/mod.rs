mod health;
mod hypixel;
mod minecraft;

use axum::Router;
use axum::extract::Request;
use axum::middleware;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::{ComponentsBuilder, OpenApi};
use utoipa::{Modify, OpenApi as _};
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::AppState;
use crate::auth;

#[derive(utoipa::OpenApi)]
#[openapi(
    info(title = "HyTools API", description = "Hypixel, Mojang and Discord data for the HyTools bot and dashboard"),
    modifiers(&ApiKeyScheme),
    tags(
        (name = "health", description = "Liveness"),
        (name = "minecraft", description = "Minecraft accounts, from Mojang"),
        (name = "hypixel", description = "Hypixel players and guilds, rate limited and cached"),
    ),
)]
struct ApiDoc;

/// Every route except `health` takes `Authorization: Bearer <key>`
struct ApiKeyScheme;

impl Modify for ApiKeyScheme {
    fn modify(&self, openapi: &mut OpenApi) {
        let components = openapi.components.take().unwrap_or_default();
        let mut components = ComponentsBuilder::from(components);
        components = components.security_scheme(
            "api_key",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
        openapi.components = Some(components.build());
    }
}

/// The routes, the OpenAPI spec at `/openapi.json` and Swagger UI at `/docs`
pub fn app(state: AppState) -> Router {
    let (router, spec) = openapi_router(state.clone()).split_for_parts();
    router
        .merge(SwaggerUi::new("/docs").url("/openapi.json", spec))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request| {
                    tracing::info_span!(
                        "request",
                        method = %request.method(),
                        path = %request.uri().path(),
                        client = tracing::field::Empty,
                    )
                })
                // One line per request, with its status and duration
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state)
}

fn openapi_router(state: AppState) -> OpenApiRouter<AppState> {
    let authenticated = OpenApiRouter::new()
        .nest("/minecraft", minecraft::router())
        .nest("/hypixel", hypixel::router())
        .route_layer(middleware::from_fn_with_state(state, auth::require_key));
    OpenApiRouter::with_openapi(ApiDoc::openapi()).nest(
        "/v1",
        OpenApiRouter::new()
            .merge(health::router())
            .merge(authenticated),
    )
}

#[cfg(test)]
mod tests;
