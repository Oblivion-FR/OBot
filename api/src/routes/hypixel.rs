use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::AppState;
use crate::error::{ApiError, ApiResult, ErrorBody};
use crate::hypixel::Guild;
use crate::mojang::normalize_uuid;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(get_player))
        .routes(routes!(get_guild))
        .routes(routes!(find_guild))
}

#[derive(Deserialize, IntoParams)]
pub struct CacheParams {
    /// Ask Hypixel again instead of using an answer up to 10 minutes old
    #[serde(default)]
    fresh: bool,
}

/// A player's Hypixel profile
#[derive(Serialize, ToSchema)]
pub struct HypixelPlayer {
    /// UUID without dashes
    uuid: String,
    /// `VIP`, `VIP_PLUS`, `MVP`, `MVP_PLUS`, `SUPERSTAR` (MVP++), `YOUTUBER` or `STAFF`, null
    /// without a rank
    #[schema(example = "MVP_PLUS")]
    rank: Option<String>,
    /// Discord username linked in the Hypixel social menu
    discord: Option<String>,
}

#[utoipa::path(
    get,
    path = "/players/{uuid}",
    tag = "hypixel",
    params(("uuid" = String, Path, description = "Minecraft UUID, with or without dashes"), CacheParams),
    responses(
        (status = OK, body = HypixelPlayer),
        (status = BAD_REQUEST, description = "`invalid_uuid`", body = ErrorBody),
        (status = NOT_FOUND, description = "`hypixel_player_not_found`: never joined Hypixel", body = ErrorBody),
        (status = BAD_GATEWAY, description = "`hypixel_unavailable`", body = ErrorBody),
    ),
    security(("api_key" = [])),
)]
async fn get_player(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
    Query(params): Query<CacheParams>,
) -> ApiResult<Json<HypixelPlayer>> {
    let uuid = normalize_uuid(&uuid)
        .ok_or_else(|| ApiError::bad_request("invalid_uuid", "not a Minecraft UUID"))?;
    let player = state
        .hypixel
        .player(&uuid, params.fresh)
        .await?
        .ok_or_else(|| {
            ApiError::not_found(
                "hypixel_player_not_found",
                "this player never joined Hypixel",
            )
        })?;
    Ok(Json(HypixelPlayer {
        uuid,
        rank: player.rank,
        discord: player.discord,
    }))
}

#[derive(Serialize, ToSchema)]
pub struct HypixelGuild {
    id: String,
    name: String,
    tag: Option<String>,
    /// Every rank, highest first, starting with `Guild Master`
    ranks: Vec<GuildRank>,
    members: Vec<GuildMember>,
}

#[derive(Serialize, ToSchema)]
pub struct GuildRank {
    name: String,
    /// Short tag shown in game, `GM` for the Guild Master
    tag: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct GuildMember {
    /// UUID without dashes
    uuid: String,
    rank: String,
    /// Unix time in seconds
    joined_at: Option<i64>,
    /// Guild XP earned over the last 7 days
    weekly_xp: u64,
}

impl From<&Guild> for HypixelGuild {
    fn from(guild: &Guild) -> Self {
        Self {
            id: guild.id.clone(),
            name: guild.name.clone(),
            tag: guild.tag.clone(),
            ranks: guild
                .rank_names()
                .into_iter()
                .map(|name| GuildRank {
                    tag: guild.rank_tag(&name).map(str::to_owned),
                    name,
                })
                .collect(),
            members: guild
                .member_uuids()
                .filter_map(|uuid| {
                    let stats = guild.member_stats(uuid)?;
                    Some(GuildMember {
                        uuid: uuid.to_owned(),
                        rank: guild.member_rank(uuid)?.to_owned(),
                        joined_at: stats.joined_at,
                        weekly_xp: stats.weekly_xp,
                    })
                })
                .collect(),
        }
    }
}

fn guild_not_found() -> ApiError {
    ApiError::not_found("hypixel_guild_not_found", "no such Hypixel guild")
}

#[utoipa::path(
    get,
    path = "/guilds/{id}",
    tag = "hypixel",
    params(("id" = String, Path, description = "Hypixel guild ID"), CacheParams),
    responses(
        (status = OK, body = HypixelGuild),
        (status = NOT_FOUND, description = "`hypixel_guild_not_found`", body = ErrorBody),
        (status = BAD_GATEWAY, description = "`hypixel_unavailable`", body = ErrorBody),
    ),
    security(("api_key" = [])),
)]
async fn get_guild(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<CacheParams>,
) -> ApiResult<Json<HypixelGuild>> {
    let guild = state
        .hypixel
        .guild(&id, params.fresh)
        .await?
        .ok_or_else(guild_not_found)?;
    Ok(Json(guild.as_ref().into()))
}

#[derive(Deserialize, IntoParams)]
pub struct FindGuildParams {
    /// Guild name, any capitalization
    name: String,
}

/// Looks a guild up by name. Always asks Hypixel.
#[utoipa::path(
    get,
    path = "/guilds",
    tag = "hypixel",
    params(FindGuildParams),
    responses(
        (status = OK, body = HypixelGuild),
        (status = BAD_REQUEST, description = "`invalid_guild_name`", body = ErrorBody),
        (status = NOT_FOUND, description = "`hypixel_guild_not_found`", body = ErrorBody),
        (status = BAD_GATEWAY, description = "`hypixel_unavailable`", body = ErrorBody),
    ),
    security(("api_key" = [])),
)]
async fn find_guild(
    State(state): State<AppState>,
    Query(params): Query<FindGuildParams>,
) -> ApiResult<Json<HypixelGuild>> {
    let name = params.name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request(
            "invalid_guild_name",
            "the guild name is empty",
        ));
    }
    let guild = state
        .hypixel
        .guild_by_name(name)
        .await?
        .ok_or_else(guild_not_found)?;
    Ok(Json(guild.as_ref().into()))
}
