use askama::Template;
use axum::extract::{Form, Path, Query, State};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::Arc;

use super::auth::{LoggedIn, MaybeLoggedIn, User};
use super::members::{self, MembersTable};
use super::{Access, AppError, AppState, GuildSummary, RoleChip, RoleOption, Shell};
use crate::config::{self, HypixelGuildLink, RuleKind};
use crate::hypixel;
use crate::nickname::{self, Field, NicknameFormat};

fn render(template: impl Template) -> Result<Response, AppError> {
    Ok(Html(template.render()?).into_response())
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage;

#[derive(Template)]
#[template(path = "home.html")]
struct HomePage {
    shell: Shell,
}

pub async fn home(
    State(state): State<Arc<AppState>>,
    MaybeLoggedIn(user): MaybeLoggedIn,
) -> Result<Response, AppError> {
    match user {
        None => render(LoginPage),
        Some(user) => render(HomePage {
            shell: state.shell(user, None).await,
        }),
    }
}

/// What every guild page is built on
struct GuildContext {
    access: Access,
    shell: Shell,
    guild: GuildSummary,
}

async fn guild_context(
    state: &AppState,
    user: User,
    guild_id: NonZeroU64,
) -> Result<GuildContext, Redirect> {
    let access = state.authorize(guild_id, &user).await?;
    let guild = state
        .guild_summary(access.guild_id)
        .ok_or_else(|| Redirect::to("/"))?;
    let shell = state.shell(user, Some(access.guild_id)).await;
    Ok(GuildContext {
        access,
        shell,
        guild,
    })
}

/// Unwraps a `Result<_, Redirect>`, returning the redirect from the handler on error
macro_rules! or_respond {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(redirect) => return Ok(redirect.into_response()),
        }
    };
}

#[derive(Deserialize)]
pub struct PageParams {
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
        "missing_group_name" => "A group needs a name.",
        "unknown_group" => "This group doesn't exist anymore.",
        _ => return None,
    })
}

/// Back to a guild page after a form post, `section` is the path after the guild ID
fn section_redirect(guild_id: serenity::GuildId, section: &str, error: Option<&str>) -> Response {
    match error {
        Some(error) => Redirect::to(&format!("/guilds/{guild_id}{section}?error={error}")),
        None => Redirect::to(&format!("/guilds/{guild_id}{section}")),
    }
    .into_response()
}

// Overview

#[derive(Template)]
#[template(path = "overview.html")]
struct OverviewPage {
    shell: Shell,
    guild: GuildSummary,
    section: &'static str,
    section_title: &'static str,
    error: Option<&'static str>,
    verified_role: Option<RoleChip>,
    hypixel_guild: Option<String>,
    rule_count: usize,
    nickname_enabled: bool,
    nickname_example: String,
}

pub async fn overview(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
) -> Result<Response, AppError> {
    let ctx = or_respond!(guild_context(&state, user, guild_id).await);
    let guild_id = ctx.access.guild_id;
    let config = config::get_config(&state.db, guild_id).await?;
    let rules = config::list_rules(&state.db, guild_id).await?;
    let nickname_format = config::get_nickname_format(&state.db, guild_id).await?;

    render(OverviewPage {
        shell: ctx.shell,
        guild: ctx.guild,
        section: "overview",
        section_title: "Overview",
        error: None,
        verified_role: config
            .verified_role_id
            .map(|role| state.role_chip(guild_id, role)),
        hypixel_guild: config.hypixel_guild.map(|link| link.name),
        rule_count: rules.len(),
        nickname_enabled: nickname_format.enabled,
        nickname_example: previews(&nickname_format)
            .into_iter()
            .next()
            .map(|preview| preview.nickname)
            .unwrap_or_default(),
    })
}

// Verification

#[derive(Template)]
#[template(path = "verification.html")]
struct VerificationPage {
    shell: Shell,
    guild: GuildSummary,
    section: &'static str,
    section_title: &'static str,
    error: Option<&'static str>,
    roles: Vec<RoleOption>,
    verified_role_id: Option<serenity::RoleId>,
    unverified_role_id: Option<serenity::RoleId>,
    hypixel_guild: Option<String>,
    guild_id: serenity::GuildId,
    members: MembersTable,
}

#[derive(Deserialize)]
pub struct VerificationParams {
    error: Option<String>,
    #[serde(flatten)]
    table: members::TableParams,
}

pub async fn verification(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Query(params): Query<VerificationParams>,
    cookies: CookieJar,
) -> Result<Response, AppError> {
    let ctx = or_respond!(guild_context(&state, user, guild_id).await);
    let guild_id = ctx.access.guild_id;
    let config = config::get_config(&state.db, guild_id).await?;
    let hidden = members::hidden_columns(
        cookies
            .get(members::HIDDEN_COLUMNS_COOKIE)
            .map(|cookie| cookie.value()),
    );
    let members = members::table(&state, guild_id, &config, params.table, hidden).await?;

    render(VerificationPage {
        roles: state.role_options(&ctx.access),
        shell: ctx.shell,
        guild: ctx.guild,
        section: "verification",
        section_title: "Verification",
        error: params.error.as_deref().and_then(error_message),
        verified_role_id: config.verified_role_id,
        unverified_role_id: config.unverified_role_id,
        hypixel_guild: config.hypixel_guild.map(|link| link.name),
        guild_id,
        members,
    })
}

/// Empty select values mean "none"
fn parse_role(value: &str) -> Option<serenity::RoleId> {
    value.parse::<NonZeroU64>().ok().map(serenity::RoleId::from)
}

#[derive(Deserialize)]
pub struct RolesForm {
    // A selected but disabled option is not submitted at all
    #[serde(default)]
    verified_role_id: String,
    #[serde(default)]
    unverified_role_id: String,
}

pub async fn save_roles(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Form(form): Form<RolesForm>,
) -> Result<Response, AppError> {
    let access = or_respond!(state.authorize(guild_id, &user).await);
    let guild_id = access.guild_id;
    let verified = parse_role(&form.verified_role_id);
    let unverified = parse_role(&form.unverified_role_id);
    let all_allowed = [verified, unverified]
        .into_iter()
        .flatten()
        .all(|role| state.is_allowed_role(&access, role));
    if !all_allowed {
        return Ok(section_redirect(
            guild_id,
            "/verification",
            Some("role_not_allowed"),
        ));
    }
    config::set_roles(&state.db, guild_id, verified, unverified).await?;
    Ok(section_redirect(guild_id, "/verification", None))
}

#[derive(Deserialize)]
pub struct HypixelGuildForm {
    name: String,
}

pub async fn save_hypixel_guild(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Form(form): Form<HypixelGuildForm>,
) -> Result<Response, AppError> {
    let access = or_respond!(state.authorize(guild_id, &user).await);
    let guild_id = access.guild_id;
    let name = form.name.trim();
    let link = if name.is_empty() {
        None
    } else {
        match state.hypixel.guild_by_name(name).await? {
            // Store the ID, it survives guild renames
            Some(guild) => Some(HypixelGuildLink {
                id: guild.id.clone(),
                name: guild.name.clone(),
            }),
            None => {
                return Ok(section_redirect(
                    guild_id,
                    "/verification",
                    Some("guild_not_found"),
                ));
            }
        }
    };
    config::set_hypixel_guild(&state.db, guild_id, link).await?;
    Ok(section_redirect(guild_id, "/verification", None))
}

// Role rules

struct RuleRow {
    id: i64,
    condition: String,
    role: RoleChip,
}

struct GroupRow {
    id: i64,
    name: String,
    separator: RoleChip,
    rules: Vec<RuleRow>,
}

#[derive(Template)]
#[template(path = "rules.html")]
struct RulesPage {
    shell: Shell,
    guild: GuildSummary,
    section: &'static str,
    section_title: &'static str,
    error: Option<&'static str>,
    groups: Vec<GroupRow>,
    /// Rules outside any group
    ungrouped: Vec<RuleRow>,
    rule_count: usize,
    roles: Vec<RoleOption>,
    ranks: &'static [(&'static str, &'static str)],
    hypixel_guild: Option<String>,
    guild_ranks: Vec<String>,
    guild_ranks_error: bool,
}

pub async fn rules(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Query(params): Query<PageParams>,
) -> Result<Response, AppError> {
    let ctx = or_respond!(guild_context(&state, user, guild_id).await);
    let guild_id = ctx.access.guild_id;
    let config = config::get_config(&state.db, guild_id).await?;
    let rules = config::list_rules(&state.db, guild_id).await?;
    let groups = config::list_groups(&state.db, guild_id).await?;
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

    let row = |rule: &config::Rule| RuleRow {
        id: rule.id,
        condition: match rule.kind {
            RuleKind::HypixelRank if rule.value == hypixel::NO_RANK => "No Hypixel rank".to_owned(),
            RuleKind::HypixelRank => {
                format!("Hypixel rank is {}", hypixel::rank_label(&rule.value))
            }
            RuleKind::GuildMember => "Member of the Hypixel guild".to_owned(),
            RuleKind::GuildRank => format!("Guild rank is {}", rule.value),
        },
        role: state.role_chip(guild_id, rule.role_id),
    };
    let rows_in = |group_id: Option<i64>| {
        rules
            .iter()
            .filter(|rule| rule.group_id == group_id)
            .map(row)
            .collect::<Vec<_>>()
    };
    let group_rows = groups
        .iter()
        .map(|group| GroupRow {
            id: group.id,
            name: group.name.clone(),
            separator: state.role_chip(guild_id, group.separator_role_id),
            rules: rows_in(Some(group.id)),
        })
        .collect();
    // A rule of a group deleted meanwhile shows with the ungrouped ones
    let ungrouped = rules
        .iter()
        .filter(|rule| {
            rule.group_id
                .is_none_or(|id| !groups.iter().any(|group| group.id == id))
        })
        .map(row)
        .collect();

    render(RulesPage {
        roles: state.role_options(&ctx.access),
        shell: ctx.shell,
        guild: ctx.guild,
        section: "rules",
        section_title: "Role rules",
        error: params.error.as_deref().and_then(error_message),
        groups: group_rows,
        ungrouped,
        rule_count: rules.len(),
        ranks: hypixel::RANKS,
        hypixel_guild: config.hypixel_guild.map(|link| link.name),
        guild_ranks,
        guild_ranks_error,
    })
}

#[derive(Deserialize)]
pub struct RuleForm {
    kind: String,
    #[serde(default)]
    rank: String,
    #[serde(default)]
    guild_rank: String,
    role_id: String,
    /// Empty for no group
    #[serde(default)]
    group_id: String,
}

pub async fn add_rule(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Form(form): Form<RuleForm>,
) -> Result<Response, AppError> {
    let access = or_respond!(state.authorize(guild_id, &user).await);
    let guild_id = access.guild_id;
    let back = |error| Ok(section_redirect(guild_id, "/rules", error));
    let (Some(kind), Some(role_id)) = (RuleKind::parse(&form.kind), parse_role(&form.role_id))
    else {
        return back(None);
    };
    if !state.is_allowed_role(&access, role_id) {
        return back(Some("role_not_allowed"));
    }
    let group_id = match form.group_id.trim() {
        "" => None,
        id => {
            let groups = config::list_groups(&state.db, guild_id).await?;
            match groups.iter().find(|group| group.id.to_string() == id) {
                Some(group) => Some(group.id),
                None => return back(Some("unknown_group")),
            }
        }
    };
    let hypixel_guild = config::get_config(&state.db, guild_id).await?.hypixel_guild;
    // Values come from dropdowns, but a stale page or crafted post could send anything
    let value = match kind {
        RuleKind::HypixelRank => {
            let rank = form.rank.trim();
            if !hypixel::RANKS.iter().any(|(key, _)| *key == rank) {
                return back(Some("missing_value"));
            }
            rank.to_owned()
        }
        RuleKind::GuildMember | RuleKind::GuildRank => {
            let Some(link) = hypixel_guild else {
                return back(Some("no_hypixel_guild"));
            };
            if kind == RuleKind::GuildMember {
                String::new()
            } else {
                let names = match state.guild_rank_names(&link.id).await {
                    Ok(names) => names,
                    Err(error) => {
                        eprintln!("Could not load ranks of Hypixel guild {}: {error}", link.id);
                        return back(Some("guild_ranks_unavailable"));
                    }
                };
                let rank = form.guild_rank.trim();
                match names
                    .into_iter()
                    .find(|name| name.eq_ignore_ascii_case(rank))
                {
                    Some(name) => name,
                    None => return back(Some("unknown_guild_rank")),
                }
            }
        }
    };
    config::add_rule(&state.db, guild_id, kind, &value, role_id, group_id).await?;
    back(None)
}

pub async fn delete_rule(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path((guild_id, rule_id)): Path<(NonZeroU64, i64)>,
) -> Result<Response, AppError> {
    let access = or_respond!(state.authorize(guild_id, &user).await);
    config::delete_rule(&state.db, access.guild_id, rule_id).await?;
    Ok(section_redirect(access.guild_id, "/rules", None))
}

const MAX_GROUP_NAME_LEN: usize = 50;

#[derive(Deserialize)]
pub struct GroupForm {
    name: String,
    separator_role_id: String,
}

pub async fn add_group(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Form(form): Form<GroupForm>,
) -> Result<Response, AppError> {
    let access = or_respond!(state.authorize(guild_id, &user).await);
    let guild_id = access.guild_id;
    let back = |error| Ok(section_redirect(guild_id, "/rules", error));
    let name = form.name.trim();
    if name.is_empty() {
        return back(Some("missing_group_name"));
    }
    let name: String = name.chars().take(MAX_GROUP_NAME_LEN).collect();
    let Some(separator_role_id) = parse_role(&form.separator_role_id) else {
        return back(None);
    };
    if !state.is_allowed_role(&access, separator_role_id) {
        return back(Some("role_not_allowed"));
    }
    config::add_group(&state.db, guild_id, &name, separator_role_id).await?;
    back(None)
}

pub async fn delete_group(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path((guild_id, group_id)): Path<(NonZeroU64, i64)>,
) -> Result<Response, AppError> {
    let access = or_respond!(state.authorize(guild_id, &user).await);
    config::delete_group(&state.db, access.guild_id, group_id).await?;
    Ok(section_redirect(access.guild_id, "/rules", None))
}

// Nickname

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
#[template(path = "nickname.html")]
struct NicknamePage {
    shell: Shell,
    guild: GuildSummary,
    section: &'static str,
    section_title: &'static str,
    error: Option<&'static str>,
    nickname_enabled: bool,
    nickname_separator: String,
    nickname_rows: Vec<NicknameRow>,
    previews: Vec<Preview>,
}

#[derive(Template)]
#[template(path = "nickname_preview.html")]
struct PreviewFragment {
    previews: Vec<Preview>,
}

pub async fn nickname(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
) -> Result<Response, AppError> {
    let ctx = or_respond!(guild_context(&state, user, guild_id).await);
    let format = config::get_nickname_format(&state.db, ctx.access.guild_id).await?;

    render(NicknamePage {
        shell: ctx.shell,
        guild: ctx.guild,
        section: "nickname",
        section_title: "Nickname",
        error: None,
        nickname_enabled: format.enabled,
        nickname_separator: format.separator.clone(),
        nickname_rows: NicknameRow::from_format(&format),
        previews: previews(&format),
    })
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

pub async fn save_nickname(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, AppError> {
    let access = or_respond!(state.authorize(guild_id, &user).await);
    let format = parse_nickname_form(&form);
    config::set_nickname_format(&state.db, access.guild_id, &format).await?;
    Ok(section_redirect(access.guild_id, "/nickname", None))
}

pub async fn preview_nickname(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path(guild_id): Path<NonZeroU64>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, AppError> {
    or_respond!(state.authorize(guild_id, &user).await);
    render(PreviewFragment {
        previews: previews(&parse_nickname_form(&form)),
    })
}

#[cfg(test)]
mod tests;
