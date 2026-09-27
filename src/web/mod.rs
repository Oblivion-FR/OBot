mod auth;
mod members;
mod pages;
mod privacy;

use axum::Router;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use std::num::NonZeroU64;
use std::sync::Arc;

use crate::Error;
use crate::cache::TtlCache;
use crate::hypixel::Hypixel;
use auth::User;
pub use auth::{OAuthConfig, Sessions};
pub use privacy::parse_admin_ids;

pub struct AppState {
    pub db: SqlitePool,
    pub cache: Arc<serenity::Cache>,
    pub discord: Arc<serenity::Http>,
    pub http_client: reqwest::Client,
    pub hypixel: Arc<Hypixel>,
    pub oauth: OAuthConfig,
    pub sessions: Sessions,
    pub manageable_guilds: ManageableGuildsCache,
    pub guild_members: GuildMembersCache,
    /// Who can see and erase everything stored about a person, from `PRIVACY_ADMIN_IDS`
    pub privacy_admins: Vec<serenity::UserId>,
}

/// Guilds listed in a user's server rail. Only navigation: each guild page still checks access live
pub type ManageableGuildsCache = TtlCache<serenity::UserId, Vec<serenity::GuildId>, 60>;
/// Member list shown in the verification page, dropped when an action changes someone's roles
pub type GuildMembersCache = TtlCache<serenity::GuildId, Arc<Vec<members::MemberInfo>>, 60>;

pub fn router(state: AppState) -> Router {
    let state = Arc::new(state);
    Router::new()
        .route("/", get(pages::home))
        .route("/login", get(auth::login))
        .route("/callback", get(auth::callback))
        .route("/logout", post(auth::logout))
        .route("/lang", post(auth::set_lang))
        .route("/data", get(privacy::page))
        .route("/data/erase", post(privacy::erase_person))
        .route("/static/app.css", get(stylesheet))
        .route("/static/app.js", get(script))
        .route("/guilds/{guild_id}", get(pages::overview))
        .route("/guilds/{guild_id}/verification", get(pages::verification))
        .route("/guilds/{guild_id}/roles", post(pages::save_roles))
        .route(
            "/guilds/{guild_id}/hypixel-guild",
            post(pages::save_hypixel_guild),
        )
        .route(
            "/guilds/{guild_id}/rules",
            get(pages::rules).post(pages::add_rule),
        )
        .route(
            "/guilds/{guild_id}/rules/{rule_id}",
            post(pages::update_rule),
        )
        .route(
            "/guilds/{guild_id}/rules/{rule_id}/group",
            post(pages::move_rule),
        )
        .route(
            "/guilds/{guild_id}/rules/{rule_id}/delete",
            post(pages::delete_rule),
        )
        .route("/guilds/{guild_id}/groups", post(pages::add_group))
        .route(
            "/guilds/{guild_id}/groups/{group_id}",
            post(pages::update_group),
        )
        .route(
            "/guilds/{guild_id}/groups/{group_id}/delete",
            post(pages::delete_group),
        )
        .route(
            "/guilds/{guild_id}/nickname",
            get(pages::nickname).post(pages::save_nickname),
        )
        .route(
            "/guilds/{guild_id}/settings",
            get(pages::settings).post(pages::save_settings),
        )
        .route(
            "/guilds/{guild_id}/verify-message",
            post(pages::post_verify_message),
        )
        .route(
            "/guilds/{guild_id}/members/{user_id}/reverify",
            post(members::reverify),
        )
        .route(
            "/guilds/{guild_id}/members/{user_id}/verify",
            post(members::admin_verify),
        )
        .route(
            "/guilds/{guild_id}/members/{user_id}/unverify",
            post(members::unverify),
        )
        .route(
            "/guilds/{guild_id}/nickname/preview",
            post(pages::preview_nickname),
        )
        .layer(middleware::from_fn_with_state(state.clone(), check_origin))
        .with_state(state)
}

async fn stylesheet() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        include_str!("../../static/app.css"),
    )
}

async fn script() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        include_str!("../../static/app.js"),
    )
}

pub struct AppError(Error);

impl<E: Into<Error>> From<E> for AppError {
    fn from(error: E) -> Self {
        Self(error.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        eprintln!("Panel error: {}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Something went wrong, check the bot logs.",
        )
            .into_response()
    }
}

/// Rejects cross-site form posts, on top of the `SameSite=Lax` session cookie
async fn check_origin(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let origin = request.headers().get(header::ORIGIN);
    let is_foreign =
        origin.is_some_and(|origin| origin.as_bytes() != state.oauth.public_url.as_bytes());
    if request.method() != axum::http::Method::GET && is_foreign {
        return StatusCode::FORBIDDEN.into_response();
    }
    next.run(request).await
}

/// What a panel user may do in a guild
struct Access {
    guild_id: serenity::GuildId,
    /// Position of the user's highest role, `None` for the owner who has no limit
    role_ceiling: Option<u16>,
}

impl Access {
    /// Same rule as Discord: users can only hand out roles below their highest one,
    /// otherwise the verified role could be used to grant yourself a role you can't assign
    fn can_grant(&self, role: &serenity::Role) -> bool {
        self.role_ceiling
            .is_none_or(|ceiling| role.position < ceiling)
    }
}

/// A guild as shown in the rail and the sidebar
struct GuildSummary {
    id: serenity::GuildId,
    name: String,
    icon_url: Option<String>,
    /// Shown instead of the icon when the guild has none, like Discord does
    initials: String,
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|word| word.chars().find(|c| c.is_alphanumeric()))
        .take(3)
        .collect()
}

/// Everything the page frame needs: the user and their server rail
struct Shell {
    user: User,
    guilds: Vec<GuildSummary>,
    current: Option<serenity::GuildId>,
    invite_url: String,
    /// Shows the link to data requests
    privacy_admin: bool,
}

struct RoleOption {
    id: serenity::RoleId,
    name: String,
    /// Below both the bot's and the user's highest role
    assignable: bool,
}

struct ChannelOption {
    id: serenity::ChannelId,
    name: String,
    /// The bot can see it and send messages in it
    sendable: bool,
}

struct RoleChip {
    name: String,
    /// CSS color, the role's own or a neutral one for uncolored roles
    color: String,
}

impl AppState {
    /// Checked live on each request, so losing the permission on Discord also revokes panel access
    async fn access(
        &self,
        guild_id: serenity::GuildId,
        user_id: serenity::UserId,
    ) -> Option<Access> {
        self.cache.guild(guild_id)?;
        let member = self.discord.get_member(guild_id, user_id).await.ok()?;
        let guild = self.cache.guild(guild_id)?;

        let permissions = guild.member_permissions(&member);
        let allowed = permissions.administrator()
            || (permissions.manage_guild() && permissions.manage_roles());
        if !allowed {
            return None;
        }
        let role_ceiling = (guild.owner_id != user_id).then(|| {
            guild
                .member_highest_role(&member)
                .map_or(0, |role| role.position)
        });
        Some(Access {
            guild_id,
            role_ceiling,
        })
    }

    /// Without access, pages send the user back home
    async fn authorize(&self, guild_id: NonZeroU64, user: &User) -> Result<Access, Redirect> {
        self.access(serenity::GuildId::from(guild_id), user.id)
            .await
            .ok_or_else(|| Redirect::to("/"))
    }

    fn guild_summary(&self, guild_id: serenity::GuildId) -> Option<GuildSummary> {
        let guild = self.cache.guild(guild_id)?;
        Some(GuildSummary {
            id: guild_id,
            name: guild.name.clone(),
            icon_url: guild.icon_url(),
            initials: initials(&guild.name),
        })
    }

    async fn manageable_guild_ids(&self, user_id: serenity::UserId) -> Vec<serenity::GuildId> {
        if let Some(ids) = self.manageable_guilds.get(&user_id) {
            return ids;
        }
        let mut ids = Vec::new();
        for guild_id in self.cache.guilds() {
            if self.access(guild_id, user_id).await.is_some() {
                ids.push(guild_id);
            }
        }
        self.manageable_guilds.insert(user_id, ids.clone());
        ids
    }

    async fn shell(&self, user: User, current: Option<serenity::GuildId>) -> Shell {
        let mut ids = self.manageable_guild_ids(user.id).await;
        // The current guild passed its access check, even if the cached rail predates it
        if let Some(current) = current
            && !ids.contains(&current)
        {
            ids.push(current);
        }
        let mut guilds: Vec<GuildSummary> = ids
            .into_iter()
            .filter_map(|id| self.guild_summary(id))
            .collect();
        guilds.sort_by_key(|guild| guild.name.to_lowercase());
        Shell {
            privacy_admin: self.privacy_admins.contains(&user.id),
            user,
            guilds,
            current,
            invite_url: self.invite_url(),
        }
    }

    fn invite_url(&self) -> String {
        // Roles and nicknames for verification, the rest to post in the log channel
        let permissions = serenity::Permissions::MANAGE_ROLES
            | serenity::Permissions::MANAGE_NICKNAMES
            | serenity::Permissions::VIEW_CHANNEL
            | serenity::Permissions::SEND_MESSAGES;
        format!(
            "https://discord.com/oauth2/authorize?client_id={}&scope=bot%20applications.commands&permissions={}",
            self.oauth.client_id,
            permissions.bits()
        )
    }

    async fn guild_rank_names(&self, hypixel_guild_id: &str) -> Result<Vec<String>, Error> {
        // A disbanded guild has no ranks left to pick
        Ok(self
            .hypixel
            .guild(hypixel_guild_id)
            .await?
            .map(|guild| guild.rank_names())
            .unwrap_or_default())
    }

    /// Whether the role can be put in the config: it exists, is grantable, and the user may grant it
    fn is_allowed_role(&self, access: &Access, role_id: serenity::RoleId) -> bool {
        let Some(guild) = self.cache.guild(access.guild_id) else {
            return false;
        };
        guild.roles.get(&role_id).is_some_and(|role| {
            role.id.get() != access.guild_id.get() && !role.managed && access.can_grant(role)
        })
    }

    /// Text channels in Discord's order: channels without a category first, then by category
    fn channel_options(&self, guild_id: serenity::GuildId) -> Vec<ChannelOption> {
        let Some(guild) = self.cache.guild(guild_id) else {
            return Vec::new();
        };
        let bot_id = self.cache.current_user().id;
        let bot = guild.members.get(&bot_id);
        let mut channels: Vec<&serenity::GuildChannel> = guild
            .channels
            .values()
            .filter(|channel| {
                matches!(
                    channel.kind,
                    serenity::ChannelType::Text | serenity::ChannelType::News
                )
            })
            .collect();
        let category_position = |channel: &serenity::GuildChannel| {
            channel
                .parent_id
                .and_then(|parent| guild.channels.get(&parent))
                .map_or(0, |category| u32::from(category.position) + 1)
        };
        channels.sort_by_key(|channel| (category_position(channel), channel.position));
        channels
            .into_iter()
            .map(|channel| ChannelOption {
                id: channel.id,
                name: channel.name.clone(),
                // Without the bot's member in cache, let Discord decide
                sendable: bot.is_none_or(|bot| {
                    let permissions = guild.user_permissions_in(channel, bot);
                    permissions.view_channel() && permissions.send_messages()
                }),
            })
            .collect()
    }

    fn is_sendable_channel(
        &self,
        guild_id: serenity::GuildId,
        channel_id: serenity::ChannelId,
    ) -> bool {
        self.channel_options(guild_id)
            .iter()
            .any(|channel| channel.id == channel_id && channel.sendable)
    }

    /// Roles that can appear in the config, highest first
    fn role_options(&self, access: &Access) -> Vec<RoleOption> {
        let Some(guild) = self.cache.guild(access.guild_id) else {
            return Vec::new();
        };
        let bot_id = self.cache.current_user().id;
        let bot_position = guild
            .members
            .get(&bot_id)
            .and_then(|bot| guild.member_highest_role(bot))
            .map(|role| role.position);
        let mut roles: Vec<&serenity::Role> = guild
            .roles
            .values()
            // @everyone and bot or integration roles can't be granted
            .filter(|role| role.id.get() != access.guild_id.get() && !role.managed)
            .collect();
        roles.sort_by_key(|role| std::cmp::Reverse(role.position));
        roles
            .into_iter()
            .map(|role| RoleOption {
                id: role.id,
                name: role.name.clone(),
                assignable: access.can_grant(role)
                    && bot_position.is_none_or(|position| role.position < position),
            })
            .collect()
    }

    fn role_chip(
        &self,
        lang: crate::i18n::Lang,
        guild_id: serenity::GuildId,
        role_id: serenity::RoleId,
    ) -> RoleChip {
        let role = self.cache.guild(guild_id).and_then(|guild| {
            guild
                .roles
                .get(&role_id)
                .map(|role| (role.name.clone(), role.colour))
        });
        match role {
            Some((name, colour)) => RoleChip {
                name,
                color: if colour.0 == 0 {
                    "var(--muted)".to_owned()
                } else {
                    format!("#{}", colour.hex())
                },
            },
            None => RoleChip {
                name: lang.t("deleted-role"),
                color: "var(--danger)".to_owned(),
            },
        }
    }
}

#[cfg(test)]
mod tests;
