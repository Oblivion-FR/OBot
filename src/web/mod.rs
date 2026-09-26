mod auth;

use askama::Template;
use axum::Router;
use axum::extract::{Form, Path, Query, Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::Error;
use crate::config::{self, HypixelGuildLink, RuleKind};
use crate::hypixel::{self, GuildQuery};
use crate::nickname::{self, Field, NicknameFormat};
use auth::{LoggedIn, MaybeLoggedIn, User};
pub use auth::{OAuthConfig, Sessions};

pub struct AppState {
    pub db: SqlitePool,
    pub cache: Arc<serenity::Cache>,
    pub discord: Arc<serenity::Http>,
    pub http_client: reqwest::Client,
    pub hypixel_api_key: String,
    pub oauth: OAuthConfig,
    pub sessions: Sessions,
    pub guild_ranks: GuildRankCache,
}

const GUILD_RANKS_TTL: Duration = Duration::from_secs(5 * 60);

/// Rank names by Hypixel guild ID, kept a few minutes so page loads don't eat the API rate limit
#[derive(Default)]
pub struct GuildRankCache(Mutex<HashMap<String, (Instant, Vec<String>)>>);

impl GuildRankCache {
    fn get(&self, hypixel_guild_id: &str) -> Option<Vec<String>> {
        let cache = self.0.lock().unwrap_or_else(|e| e.into_inner());
        cache
            .get(hypixel_guild_id)
            .filter(|(fetched_at, _)| fetched_at.elapsed() < GUILD_RANKS_TTL)
            .map(|(_, names)| names.clone())
    }

    fn insert(&self, hypixel_guild_id: String, names: Vec<String>) {
        let mut cache = self.0.lock().unwrap_or_else(|e| e.into_inner());
        cache.insert(hypixel_guild_id, (Instant::now(), names));
    }
}

pub fn router(state: AppState) -> Router {
    let state = Arc::new(state);
    Router::new()
        .route("/", get(home))
        .route("/login", get(auth::login))
        .route("/callback", get(auth::callback))
        .route("/logout", post(auth::logout))
        .route("/guilds/{guild_id}", get(guild_page))
        .route("/guilds/{guild_id}/roles", post(save_roles))
        .route("/guilds/{guild_id}/hypixel-guild", post(save_hypixel_guild))
        .route("/guilds/{guild_id}/rules", post(add_rule))
        .route(
            "/guilds/{guild_id}/rules/{rule_id}/delete",
            post(delete_rule),
        )
        .route("/guilds/{guild_id}/nickname", post(save_nickname))
        .route(
            "/guilds/{guild_id}/nickname/preview",
            post(preview_nickname),
        )
        .layer(middleware::from_fn_with_state(state.clone(), check_origin))
        .with_state(state)
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

    async fn authorize(&self, guild_id: NonZeroU64, user: &User) -> Result<Access, Response> {
        self.access(serenity::GuildId::from(guild_id), user.id)
            .await
            .ok_or_else(|| Redirect::to("/").into_response())
    }

    async fn guild_rank_names(&self, hypixel_guild_id: &str) -> Result<Vec<String>, Error> {
        if let Some(names) = self.guild_ranks.get(hypixel_guild_id) {
            return Ok(names);
        }
        let query = GuildQuery::Id(hypixel_guild_id);
        let guild = hypixel::fetch_guild(&self.http_client, &self.hypixel_api_key, query).await?;
        // A disbanded guild has no ranks left to pick
        let names = guild.map(|guild| guild.rank_names()).unwrap_or_default();
        self.guild_ranks
            .insert(hypixel_guild_id.to_owned(), names.clone());
        Ok(names)
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
}

fn render(template: impl Template) -> Result<Response, AppError> {
    Ok(Html(template.render()?).into_response())
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage;

struct GuildEntry {
    id: serenity::GuildId,
    name: String,
}

#[derive(Template)]
#[template(path = "guilds.html")]
struct GuildsPage {
    user: User,
    guilds: Vec<GuildEntry>,
}

async fn home(
    State(state): State<Arc<AppState>>,
    MaybeLoggedIn(user): MaybeLoggedIn,
) -> Result<Response, AppError> {
    let Some(user) = user else {
        return render(LoginPage);
    };
    let mut guilds = Vec::new();
    for guild_id in state.cache.guilds() {
        if state.access(guild_id, user.id).await.is_some() {
            let name = state.cache.guild(guild_id).map(|guild| guild.name.clone());
            guilds.push(GuildEntry {
                id: guild_id,
                name: name.unwrap_or_default(),
            });
        }
    }
    guilds.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    render(GuildsPage { user, guilds })
}

struct RoleOption {
    id: serenity::RoleId,
    name: String,
    /// Below both the bot's and the user's highest role
    assignable: bool,
}

struct RuleRow {
    id: i64,
    condition: String,
    role: String,
}

#[derive(Template)]
#[template(path = "guild.html")]
struct GuildPage {
    user: User,
    guild_id: serenity::GuildId,
    guild_name: String,
    roles: Vec<RoleOption>,
    verified_role_id: Option<serenity::RoleId>,
    unverified_role_id: Option<serenity::RoleId>,
    hypixel_guild: Option<String>,
    guild_ranks: Vec<String>,
    guild_ranks_error: bool,
    rules: Vec<RuleRow>,
    ranks: &'static [(&'static str, &'static str)],
    nickname_enabled: bool,
    nickname_separator: String,
    nickname_rows: Vec<NicknameRow>,
    previews: Vec<Preview>,
    error: Option<&'static str>,
}

struct NicknameRow {
    key: &'static str,
    label: &'static str,
    position: usize,
    enabled: bool,
    prefix: String,
    suffix: String,
    importance: u8,
}

impl NicknameRow {
    fn from_format(format: &NicknameFormat) -> Vec<Self> {
        format
            .segments
            .iter()
            .enumerate()
            .map(|(index, segment)| NicknameRow {
                key: segment.field.as_str(),
                label: segment.field.label(),
                position: index + 1,
                enabled: segment.enabled,
                prefix: segment.prefix.clone(),
                suffix: segment.suffix.clone(),
                importance: segment.importance,
            })
            .collect()
    }
}

struct Preview {
    label: &'static str,
    nickname: String,
}

impl Preview {
    fn length(&self) -> usize {
        self.nickname.chars().count()
    }
}

/// Sample members: a typical one, and one with every field at its longest to show what gets dropped
fn previews(format: &NicknameFormat) -> Vec<Preview> {
    let samples = [
        ("Example", "MVP+", "Notch", "OFC", "OBOT"),
        ("Longest", "MVP++", "Sixteen_Chars_Ok", "ELITE", "LONGTG"),
    ];
    samples
        .into_iter()
        .map(|(label, rank, ign, guild_rank_tag, guild_tag)| Preview {
            label,
            nickname: format.render(&nickname::Values {
                hypixel_rank: Some(rank),
                ign,
                guild_rank_tag: Some(guild_rank_tag),
                guild_tag: Some(guild_tag),
            }),
        })
        .collect()
}

#[derive(Template)]
#[template(path = "nickname_preview.html")]
struct PreviewFragment {
    previews: Vec<Preview>,
}

#[derive(Deserialize)]
struct PageParams {
    error: Option<String>,
}

fn error_message(code: &str) -> Option<&'static str> {
    Some(match code {
        "guild_not_found" => "No Hypixel guild has this name.",
        "no_hypixel_guild" => "Link a Hypixel guild before adding guild rules.",
        "missing_value" => "This rule needs a value.",
        "unknown_guild_rank" => "This rank doesn't exist in the Hypixel guild anymore.",
        "guild_ranks_unavailable" => "Couldn't load the guild ranks from Hypixel, try again later.",
        "role_not_allowed" => "You can only pick roles below your highest role.",
        _ => return None,
    })
}

async fn guild_page(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Query(params): Query<PageParams>,
) -> Result<Response, AppError> {
    let access = match state.authorize(guild_id, &user).await {
        Ok(access) => access,
        Err(response) => return Ok(response),
    };
    let guild_id = access.guild_id;
    let config = config::get_config(&state.db, guild_id).await?;
    let rules = config::list_rules(&state.db, guild_id).await?;
    let nickname_format = config::get_nickname_format(&state.db, guild_id).await?;
    let (guild_ranks, guild_ranks_error) = match &config.hypixel_guild {
        None => (Vec::new(), false),
        Some(link) => match state.guild_rank_names(&link.id).await {
            Ok(names) => (names, false),
            Err(error) => {
                eprintln!("Could not load ranks of Hypixel guild {}: {error}", link.id);
                (Vec::new(), true)
            }
        },
    };

    let (guild_name, roles) = {
        let Some(guild) = state.cache.guild(guild_id) else {
            return Ok(Redirect::to("/").into_response());
        };
        let bot_id = state.cache.current_user().id;
        let bot_position = guild
            .members
            .get(&bot_id)
            .and_then(|bot| guild.member_highest_role(bot))
            .map(|role| role.position);
        let mut roles: Vec<_> = guild
            .roles
            .values()
            // @everyone and bot or integration roles can't be granted
            .filter(|role| role.id.get() != guild_id.get() && !role.managed)
            .map(|role| {
                (
                    role.position,
                    RoleOption {
                        id: role.id,
                        name: role.name.clone(),
                        assignable: access.can_grant(role)
                            && bot_position.is_none_or(|position| role.position < position),
                    },
                )
            })
            .collect();
        roles.sort_by(|a, b| b.0.cmp(&a.0));
        (
            guild.name.clone(),
            roles.into_iter().map(|(_, role)| role).collect::<Vec<_>>(),
        )
    };

    let role_name = |id: serenity::RoleId| {
        roles.iter().find(|role| role.id == id).map_or_else(
            || "(deleted role)".to_owned(),
            |role| format!("@{}", role.name),
        )
    };
    let rules = rules
        .iter()
        .map(|rule| RuleRow {
            id: rule.id,
            condition: match rule.kind {
                RuleKind::HypixelRank => {
                    format!("Hypixel rank is {}", hypixel::rank_label(&rule.value))
                }
                RuleKind::GuildMember => "Member of the Hypixel guild".to_owned(),
                RuleKind::GuildRank => format!("Guild rank is {}", rule.value),
            },
            role: role_name(rule.role_id),
        })
        .collect();

    render(GuildPage {
        user,
        guild_id,
        guild_name,
        roles,
        verified_role_id: config.verified_role_id,
        unverified_role_id: config.unverified_role_id,
        hypixel_guild: config.hypixel_guild.map(|link| link.name),
        guild_ranks,
        guild_ranks_error,
        rules,
        ranks: hypixel::RANKS,
        nickname_enabled: nickname_format.enabled,
        nickname_separator: nickname_format.separator.clone(),
        nickname_rows: NicknameRow::from_format(&nickname_format),
        previews: previews(&nickname_format),
        error: params.error.as_deref().and_then(error_message),
    })
}

fn guild_redirect(guild_id: serenity::GuildId, error: Option<&str>) -> Response {
    match error {
        Some(error) => Redirect::to(&format!("/guilds/{guild_id}?error={error}")),
        None => Redirect::to(&format!("/guilds/{guild_id}")),
    }
    .into_response()
}

/// Empty select values mean "none"
fn parse_role(value: &str) -> Option<serenity::RoleId> {
    value.parse::<NonZeroU64>().ok().map(serenity::RoleId::from)
}

#[derive(Deserialize)]
struct RolesForm {
    // A selected but disabled option is not submitted at all
    #[serde(default)]
    verified_role_id: String,
    #[serde(default)]
    unverified_role_id: String,
}

async fn save_roles(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Form(form): Form<RolesForm>,
) -> Result<Response, AppError> {
    let access = match state.authorize(guild_id, &user).await {
        Ok(access) => access,
        Err(response) => return Ok(response),
    };
    let guild_id = access.guild_id;
    let verified = parse_role(&form.verified_role_id);
    let unverified = parse_role(&form.unverified_role_id);
    let all_allowed = [verified, unverified]
        .into_iter()
        .flatten()
        .all(|role| state.is_allowed_role(&access, role));
    if !all_allowed {
        return Ok(guild_redirect(guild_id, Some("role_not_allowed")));
    }
    config::set_roles(&state.db, guild_id, verified, unverified).await?;
    Ok(guild_redirect(guild_id, None))
}

#[derive(Deserialize)]
struct HypixelGuildForm {
    name: String,
}

async fn save_hypixel_guild(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Form(form): Form<HypixelGuildForm>,
) -> Result<Response, AppError> {
    let access = match state.authorize(guild_id, &user).await {
        Ok(access) => access,
        Err(response) => return Ok(response),
    };
    let guild_id = access.guild_id;
    let name = form.name.trim();
    let link = if name.is_empty() {
        None
    } else {
        let query = GuildQuery::Name(name);
        match hypixel::fetch_guild(&state.http_client, &state.hypixel_api_key, query).await? {
            // Store the ID, it survives guild renames
            Some(guild) => Some({
                state
                    .guild_ranks
                    .insert(guild.id.clone(), guild.rank_names());
                HypixelGuildLink {
                    id: guild.id,
                    name: guild.name,
                }
            }),
            None => return Ok(guild_redirect(guild_id, Some("guild_not_found"))),
        }
    };
    config::set_hypixel_guild(&state.db, guild_id, link).await?;
    Ok(guild_redirect(guild_id, None))
}

#[derive(Deserialize)]
struct RuleForm {
    kind: String,
    #[serde(default)]
    rank: String,
    #[serde(default)]
    guild_rank: String,
    role_id: String,
}

async fn add_rule(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Form(form): Form<RuleForm>,
) -> Result<Response, AppError> {
    let access = match state.authorize(guild_id, &user).await {
        Ok(access) => access,
        Err(response) => return Ok(response),
    };
    let guild_id = access.guild_id;
    let (Some(kind), Some(role_id)) = (RuleKind::parse(&form.kind), parse_role(&form.role_id))
    else {
        return Ok(guild_redirect(guild_id, None));
    };
    if !state.is_allowed_role(&access, role_id) {
        return Ok(guild_redirect(guild_id, Some("role_not_allowed")));
    }
    let hypixel_guild = config::get_config(&state.db, guild_id).await?.hypixel_guild;
    // Values come from dropdowns, but a stale page or crafted post could send anything
    let value = match kind {
        RuleKind::HypixelRank => {
            let rank = form.rank.trim();
            if !hypixel::RANKS.iter().any(|(key, _)| *key == rank) {
                return Ok(guild_redirect(guild_id, Some("missing_value")));
            }
            rank.to_owned()
        }
        RuleKind::GuildMember | RuleKind::GuildRank => {
            let Some(link) = hypixel_guild else {
                return Ok(guild_redirect(guild_id, Some("no_hypixel_guild")));
            };
            if kind == RuleKind::GuildMember {
                String::new()
            } else {
                let names = match state.guild_rank_names(&link.id).await {
                    Ok(names) => names,
                    Err(error) => {
                        eprintln!("Could not load ranks of Hypixel guild {}: {error}", link.id);
                        return Ok(guild_redirect(guild_id, Some("guild_ranks_unavailable")));
                    }
                };
                let rank = form.guild_rank.trim();
                match names
                    .into_iter()
                    .find(|name| name.eq_ignore_ascii_case(rank))
                {
                    Some(name) => name,
                    None => return Ok(guild_redirect(guild_id, Some("unknown_guild_rank"))),
                }
            }
        }
    };
    config::add_rule(&state.db, guild_id, kind, &value, role_id).await?;
    Ok(guild_redirect(guild_id, None))
}

async fn delete_rule(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path((guild_id, rule_id)): Path<(NonZeroU64, i64)>,
) -> Result<Response, AppError> {
    let access = match state.authorize(guild_id, &user).await {
        Ok(access) => access,
        Err(response) => return Ok(response),
    };
    let guild_id = access.guild_id;
    config::delete_rule(&state.db, guild_id, rule_id).await?;
    Ok(guild_redirect(guild_id, None))
}

const MAX_WRAPPING_LEN: usize = 8;
const MAX_SEPARATOR_LEN: usize = 5;

fn limit(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

/// The nickname form has one set of inputs per field, named `<field>_<setting>`
fn parse_nickname_form(form: &HashMap<String, String>) -> NicknameFormat {
    let get = |name: &str| form.get(name).map(String::as_str).unwrap_or_default();
    let mut positioned: Vec<(u8, usize, nickname::Segment)> = Field::ALL
        .into_iter()
        .enumerate()
        .map(|(index, field)| {
            let key = field.as_str();
            let number = |setting: &str| {
                get(&format!("{key}_{setting}"))
                    .trim()
                    .parse::<u8>()
                    .unwrap_or(9)
                    .clamp(1, 9)
            };
            let segment = nickname::Segment {
                field,
                enabled: form.contains_key(&format!("{key}_enabled")),
                prefix: limit(get(&format!("{key}_prefix")), MAX_WRAPPING_LEN),
                suffix: limit(get(&format!("{key}_suffix")), MAX_WRAPPING_LEN),
                importance: number("importance"),
            };
            (number("position"), index, segment)
        })
        .collect();
    positioned.sort_by_key(|(position, index, _)| (*position, *index));

    NicknameFormat {
        enabled: form.contains_key("nickname_enabled"),
        separator: limit(get("separator"), MAX_SEPARATOR_LEN),
        segments: positioned
            .into_iter()
            .map(|(_, _, segment)| segment)
            .collect(),
    }
}

async fn save_nickname(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, AppError> {
    let access = match state.authorize(guild_id, &user).await {
        Ok(access) => access,
        Err(response) => return Ok(response),
    };
    let format = parse_nickname_form(&form);
    config::set_nickname_format(&state.db, access.guild_id, &format).await?;
    Ok(guild_redirect(access.guild_id, None))
}

async fn preview_nickname(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, AppError> {
    if let Err(response) = state.authorize(guild_id, &user).await {
        return Ok(response);
    }
    render(PreviewFragment {
        previews: previews(&parse_nickname_form(&form)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role(id: u64, name: &str, assignable: bool) -> RoleOption {
        RoleOption {
            id: serenity::RoleId::new(id),
            name: name.to_owned(),
            assignable,
        }
    }

    #[test]
    fn guild_page_renders_config() {
        let page = GuildPage {
            user: User {
                id: serenity::UserId::new(1),
                name: "admin".to_owned(),
            },
            guild_id: serenity::GuildId::new(2),
            guild_name: "Test <server>".to_owned(),
            roles: vec![
                role(10, "Admin", false),
                role(11, "Verified", true),
                role(12, "Unverified", true),
            ],
            verified_role_id: Some(serenity::RoleId::new(11)),
            unverified_role_id: None,
            hypixel_guild: Some("My Guild".to_owned()),
            guild_ranks: vec!["Guild Master".to_owned(), "Officer".to_owned()],
            guild_ranks_error: false,
            rules: vec![RuleRow {
                id: 5,
                condition: "Hypixel rank is MVP+".to_owned(),
                role: "@Verified".to_owned(),
            }],
            ranks: hypixel::RANKS,
            nickname_enabled: true,
            nickname_separator: " ".to_owned(),
            nickname_rows: NicknameRow::from_format(&NicknameFormat::default()),
            previews: previews(&NicknameFormat::default()),
            error: error_message("role_not_allowed"),
        };
        let html = page.render().expect("template renders");

        assert!(
            html.contains("Test &#60;server&#62;"),
            "guild name is escaped"
        );
        assert!(html.contains(r#"<option value="11" selected>@Verified</option>"#));
        assert!(html.contains(r#"<option value="10" disabled>@Admin</option>"#));
        assert!(html.contains(r#"value="My Guild""#));
        assert!(html.contains("/guilds/2/rules/5/delete"));
        assert!(html.contains("You can only pick roles below your highest role."));
        assert!(html.contains("[MVP+] Notch [OFC]"));
        assert!(html.contains(r#"<option value="Officer">Officer</option>"#));
        assert!(html.contains(r#"name="hypixel_rank_prefix" value="[""#));
    }

    #[test]
    fn nickname_form_reorders_and_limits() {
        let form: HashMap<String, String> = [
            ("nickname_enabled", "on"),
            ("separator", " "),
            ("ign_enabled", "on"),
            ("ign_position", "1"),
            ("ign_importance", "1"),
            ("hypixel_rank_enabled", "on"),
            ("hypixel_rank_position", "2"),
            ("hypixel_rank_prefix", "(((((((((((("),
            ("hypixel_rank_suffix", ")"),
            ("hypixel_rank_importance", "not a number"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect();
        let format = parse_nickname_form(&form);

        assert!(format.enabled);
        let order: Vec<_> = format
            .segments
            .iter()
            .map(|segment| segment.field)
            .collect();
        assert!(order[..2] == [Field::Ign, Field::HypixelRank]);
        assert_eq!(format.segments[1].prefix.len(), MAX_WRAPPING_LEN);
        assert_eq!(format.segments[1].importance, 9);
        assert!(!format.segments[2].enabled, "unchecked fields are disabled");
    }
}
