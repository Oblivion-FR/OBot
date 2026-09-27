use askama::Template;
use axum::extract::{Form, Path, Query, State};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::Arc;

use super::auth::{LoggedIn, MaybeLoggedIn, PanelLang, User};
use super::members::{self, MembersTable};
use super::{Access, AppError, AppState, GuildSummary, RoleChip, RoleOption, Shell};
use crate::config::{self, HypixelGuildLink, RuleKind};
use crate::hypixel;
use crate::i18n::{Lang, tr};
use crate::nickname::{self, Field, NicknameFormat};

fn render(template: impl Template) -> Result<Response, AppError> {
    Ok(Html(template.render()?).into_response())
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage {
    lang: Lang,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomePage {
    lang: Lang,
    shell: Shell,
}

pub async fn home(
    State(state): State<Arc<AppState>>,
    MaybeLoggedIn(user): MaybeLoggedIn,
    PanelLang(lang): PanelLang,
) -> Result<Response, AppError> {
    match user {
        None => render(LoginPage { lang }),
        Some(user) => render(HomePage {
            lang,
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

/// Message id of an error code passed back in the query string
fn error_message(code: &str) -> Option<&'static str> {
    Some(match code {
        "guild_not_found" => "error-guild-not-found",
        "no_hypixel_guild" => "error-no-hypixel-guild",
        "missing_value" => "error-missing-value",
        "unknown_guild_rank" => "error-unknown-guild-rank",
        "guild_ranks_unavailable" => "error-guild-ranks-unavailable",
        "role_not_allowed" => "error-role-not-allowed",
        "missing_group_name" => "error-missing-group-name",
        "unknown_group" => "error-unknown-group",
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
    lang: Lang,
    shell: Shell,
    guild: GuildSummary,
    section: &'static str,
    /// Message id
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
    PanelLang(lang): PanelLang,
    Path(guild_id): Path<NonZeroU64>,
) -> Result<Response, AppError> {
    let ctx = or_respond!(guild_context(&state, user, guild_id).await);
    let guild_id = ctx.access.guild_id;
    let config = config::get_config(&state.db, guild_id).await?;
    let rules = config::list_rules(&state.db, guild_id).await?;
    let nickname_format = config::get_nickname_format(&state.db, guild_id).await?;

    render(OverviewPage {
        lang,
        shell: ctx.shell,
        guild: ctx.guild,
        section: "overview",
        section_title: "nav-overview",
        error: None,
        verified_role: config
            .verified_role_id
            .map(|role| state.role_chip(lang, guild_id, role)),
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
    lang: Lang,
    shell: Shell,
    guild: GuildSummary,
    section: &'static str,
    /// Message id
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
    PanelLang(lang): PanelLang,
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
        lang,
        roles: state.role_options(&ctx.access),
        shell: ctx.shell,
        guild: ctx.guild,
        section: "verification",
        section_title: "nav-verification",
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
    lang: Lang,
    shell: Shell,
    guild: GuildSummary,
    section: &'static str,
    /// Message id
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
    PanelLang(lang): PanelLang,
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
            RuleKind::HypixelRank if rule.value == hypixel::NO_RANK => lang.t("condition-no-rank"),
            RuleKind::HypixelRank => tr!(
                lang,
                "condition-hypixel-rank",
                rank = hypixel::rank_label(&rule.value)
            ),
            RuleKind::GuildMember => lang.t("condition-guild-member"),
            RuleKind::GuildRank => tr!(lang, "condition-guild-rank", rank = rule.value.as_str()),
        },
        role: state.role_chip(lang, guild_id, rule.role_id),
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
            separator: state.role_chip(lang, guild_id, group.separator_role_id),
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
        lang,
        roles: state.role_options(&ctx.access),
        shell: ctx.shell,
        guild: ctx.guild,
        section: "rules",
        section_title: "nav-rules",
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
    /// Message id
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

/// One value's texts in the form: the effective ones, and the defaults they are compared to
/// on save, so only what was actually changed becomes custom
struct ValueRow {
    /// The key stored with custom texts
    value: String,
    /// How the value is named in the panel
    name: String,
    prefix: String,
    label: String,
    suffix: String,
    default_prefix: String,
    default_label: String,
    default_suffix: String,
    custom: bool,
}

struct ValueTable {
    key: &'static str,
    /// Message id
    title: &'static str,
    /// Message id of why the table has no rows
    note: Option<&'static str>,
    rows: Vec<ValueRow>,
}

/// The per value texts of the Hypixel rank and guild rank fields. `guild_ranks` are the linked
/// guild's ranks with their tag, or the message id of why there are none.
fn value_tables(
    lang: Lang,
    format: &NicknameFormat,
    guild_ranks: Result<Vec<(String, String)>, &'static str>,
) -> Vec<ValueTable> {
    let hypixel_ranks = hypixel::RANKS
        .iter()
        .map(|&(key, label)| {
            // Players without a rank show nothing unless given a label
            if key == hypixel::NO_RANK {
                (key.to_owned(), lang.t("rank-no-rank"), String::new())
            } else {
                (key.to_owned(), label.to_owned(), label.to_owned())
            }
        })
        .collect();
    let (guild_values, guild_note) = match guild_ranks {
        Ok(ranks) => (
            ranks
                .into_iter()
                .map(|(name, tag)| (name.clone(), name, tag))
                .collect(),
            None,
        ),
        Err(note) => (Vec::new(), Some(note)),
    };

    [
        (
            Field::HypixelRank,
            "texts-hypixel-ranks",
            hypixel_ranks,
            None,
        ),
        (
            Field::GuildRankTag,
            "texts-guild-ranks",
            guild_values,
            guild_note,
        ),
    ]
    .into_iter()
    .map(|(field, title, values, note)| {
        let segment = format
            .segments
            .iter()
            .find(|segment| segment.field == field);
        let field_prefix = segment
            .map(|segment| segment.prefix.clone())
            .unwrap_or_default();
        let field_suffix = segment
            .map(|segment| segment.suffix.clone())
            .unwrap_or_default();
        let rows = values
            .into_iter()
            .map(|(value, name, default_label): (String, String, String)| {
                let custom = format.custom_text(field, &value);
                let part =
                    |custom: Option<&String>, default: &String| custom.unwrap_or(default).clone();
                ValueRow {
                    prefix: part(custom.and_then(|c| c.prefix.as_ref()), &field_prefix),
                    label: part(custom.and_then(|c| c.label.as_ref()), &default_label),
                    suffix: part(custom.and_then(|c| c.suffix.as_ref()), &field_suffix),
                    custom: custom.is_some_and(|custom| !custom.is_default()),
                    default_prefix: field_prefix.clone(),
                    default_label,
                    default_suffix: field_suffix.clone(),
                    value,
                    name,
                }
            })
            .collect();
        ValueTable {
            key: field.as_str(),
            title,
            note,
            rows,
        }
    })
    .collect()
}

/// The linked guild's ranks with their tag, highest first, or the message id of why there are none
async fn guild_rank_tags(
    state: &AppState,
    guild_id: serenity::GuildId,
) -> Result<Result<Vec<(String, String)>, &'static str>, AppError> {
    let Some(link) = config::get_config(&state.db, guild_id).await?.hypixel_guild else {
        return Ok(Err("texts-link-guild"));
    };
    Ok(match state.hypixel.guild(&link.id).await {
        Ok(Some(guild)) => Ok(guild
            .rank_names()
            .into_iter()
            .map(|name| {
                let tag = guild.rank_tag(&name).unwrap_or_default().to_owned();
                (name, tag)
            })
            .collect()),
        Ok(None) => Err("texts-guild-gone"),
        Err(error) => {
            eprintln!("Could not load Hypixel guild {}: {error}", link.id);
            Err("texts-guild-unavailable")
        }
    })
}

struct Preview {
    /// Message id
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
        (
            "preview-example",
            ("MVP_PLUS", "MVP+"),
            "Notch",
            ("Officer", "OFC"),
            "OBOT",
        ),
        (
            "preview-longest",
            ("SUPERSTAR", "MVP++"),
            "Sixteen_Chars_Ok",
            ("Elite", "ELITE"),
            "LONGTG",
        ),
    ];
    samples
        .into_iter()
        .map(
            |(label, (rank, rank_label), ign, (guild_rank, guild_rank_tag), guild_tag)| Preview {
                label,
                nickname: format.render(&nickname::Values {
                    hypixel_rank: Some(nickname::Keyed {
                        key: rank,
                        label: Some(rank_label),
                    }),
                    ign,
                    guild_rank: Some(nickname::Keyed {
                        key: guild_rank,
                        label: Some(guild_rank_tag),
                    }),
                    guild_tag: Some(guild_tag),
                }),
            },
        )
        .collect()
}

#[derive(Template)]
#[template(path = "nickname.html")]
struct NicknamePage {
    lang: Lang,
    shell: Shell,
    guild: GuildSummary,
    section: &'static str,
    /// Message id
    section_title: &'static str,
    error: Option<&'static str>,
    nickname_enabled: bool,
    nickname_separator: String,
    nickname_rows: Vec<NicknameRow>,
    value_tables: Vec<ValueTable>,
    previews: Vec<Preview>,
}

#[derive(Template)]
#[template(path = "nickname_preview.html")]
struct PreviewFragment {
    lang: Lang,
    previews: Vec<Preview>,
}

pub async fn nickname(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    PanelLang(lang): PanelLang,
    Path(guild_id): Path<NonZeroU64>,
) -> Result<Response, AppError> {
    let ctx = or_respond!(guild_context(&state, user, guild_id).await);
    let format = config::get_nickname_format(&state.db, ctx.access.guild_id).await?;
    let guild_ranks = guild_rank_tags(&state, ctx.access.guild_id).await?;

    render(NicknamePage {
        lang,
        shell: ctx.shell,
        guild: ctx.guild,
        section: "nickname",
        section_title: "nav-nickname",
        error: None,
        nickname_enabled: format.enabled,
        nickname_separator: format.separator.clone(),
        nickname_rows: NicknameRow::from_format(&format),
        value_tables: value_tables(lang, &format, guild_ranks),
        previews: previews(&format),
    })
}

const MAX_WRAPPING_LEN: usize = 8;
const MAX_SEPARATOR_LEN: usize = 5;
/// Rows read per value table, well above Hypixel's ranks and a guild's
const MAX_VALUE_ROWS: usize = 100;

fn limit(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

/// The nickname form has one set of inputs per field, named `<field>_<setting>`, and one per
/// value of the fields with value texts, named `text_<field>_<row>_<part>`
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

    let mut custom_texts = Vec::new();
    for field in Field::WITH_VALUE_TEXTS {
        let key = field.as_str();
        for row in 0..MAX_VALUE_ROWS {
            let input = |part: &str| form.get(&format!("text_{key}_{row}_{part}"));
            let Some(value) = input("value") else {
                break;
            };
            // A part is custom only when it differs from the default it was shown with
            let part = |name: &str, max: usize| {
                let shown = input(&format!("default_{name}")).map(String::as_str);
                input(name)
                    .map(|text| limit(text, max))
                    .filter(|text| Some(text.as_str()) != shown)
            };
            let text = nickname::ValueText {
                prefix: part("prefix", MAX_WRAPPING_LEN),
                label: part("label", nickname::MAX_LEN),
                suffix: part("suffix", MAX_WRAPPING_LEN),
            };
            if !text.is_default() {
                custom_texts.push(nickname::CustomText {
                    field,
                    value: value.clone(),
                    text,
                });
            }
        }
    }

    NicknameFormat {
        enabled: form.contains_key("nickname_enabled"),
        separator: limit(get("separator"), MAX_SEPARATOR_LEN),
        segments: positioned
            .into_iter()
            .map(|(_, _, segment)| segment)
            .collect(),
        custom_texts,
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
    PanelLang(lang): PanelLang,
    Path(guild_id): Path<NonZeroU64>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, AppError> {
    or_respond!(state.authorize(guild_id, &user).await);
    render(PreviewFragment {
        lang,
        previews: previews(&parse_nickname_form(&form)),
    })
}

#[cfg(test)]
mod tests;
