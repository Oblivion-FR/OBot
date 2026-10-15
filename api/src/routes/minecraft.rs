use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::error::{ApiError, ApiResult, ErrorBody};
use crate::mojang::{Profile, normalize_uuid, valid_name};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(player_by_name))
        .routes(routes!(player_by_uuid))
}

/// A Minecraft account
#[derive(Serialize, ToSchema)]
pub struct MinecraftPlayer {
    /// UUID without dashes
    #[schema(example = "069a79f444e94726a5befca90e38aaf5")]
    uuid: String,
    /// Current name, with its real capitalization
    #[schema(example = "Notch")]
    name: String,
}

impl From<Profile> for MinecraftPlayer {
    fn from(profile: Profile) -> Self {
        Self {
            uuid: profile.id,
            name: profile.name,
        }
    }
}

fn not_found() -> ApiError {
    ApiError::not_found("minecraft_player_not_found", "no such Minecraft account")
}

#[utoipa::path(
    get,
    path = "/players/{name}",
    tag = "minecraft",
    params(("name" = String, Path, description = "Minecraft name, any capitalization")),
    responses(
        (status = OK, body = MinecraftPlayer),
        (status = BAD_REQUEST, description = "`invalid_minecraft_name`", body = ErrorBody),
        (status = NOT_FOUND, description = "`minecraft_player_not_found`", body = ErrorBody),
        (status = BAD_GATEWAY, description = "`mojang_unavailable`", body = ErrorBody),
    ),
    security(("api_key" = [])),
)]
async fn player_by_name(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<Json<MinecraftPlayer>> {
    if !valid_name(&name) {
        return Err(ApiError::bad_request(
            "invalid_minecraft_name",
            "Minecraft names are 1 to 16 letters, digits or underscores",
        ));
    }
    let profile = state.mojang.by_name(&name).await?.ok_or_else(not_found)?;
    Ok(Json(profile.into()))
}

/// The current name of an account, which picks up name changes
#[utoipa::path(
    get,
    path = "/profiles/{uuid}",
    tag = "minecraft",
    params(("uuid" = String, Path, description = "Minecraft UUID, with or without dashes")),
    responses(
        (status = OK, body = MinecraftPlayer),
        (status = BAD_REQUEST, description = "`invalid_uuid`", body = ErrorBody),
        (status = NOT_FOUND, description = "`minecraft_player_not_found`", body = ErrorBody),
        (status = BAD_GATEWAY, description = "`mojang_unavailable`", body = ErrorBody),
    ),
    security(("api_key" = [])),
)]
async fn player_by_uuid(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
) -> ApiResult<Json<MinecraftPlayer>> {
    let uuid = normalize_uuid(&uuid)
        .ok_or_else(|| ApiError::bad_request("invalid_uuid", "not a Minecraft UUID"))?;
    let profile = state.mojang.by_uuid(&uuid).await?.ok_or_else(not_found)?;
    Ok(Json(profile.into()))
}
