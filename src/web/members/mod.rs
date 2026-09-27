use askama::Template;
use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::Arc;

use super::auth::LoggedIn;
use super::{Access, AppError, AppState};
use crate::Error;
use crate::config::{self, GuildConfig, VerifiedMember};
use crate::hypixel::{self, Player};
use crate::verification::{self, Conflict, NicknameChange, Outcome, Record, RefreshFailure};

const PAGE_SIZE: usize = 50;

/// A member as listed in the panel, without the parts of `Member` the table doesn't need
#[derive(Clone)]
pub struct MemberInfo {
    id: serenity::UserId,
    name: String,
    username: String,
    avatar_url: String,
    joined_at: Option<i64>,
    roles: Vec<serenity::RoleId>,
}

impl MemberInfo {
    fn from_member(member: &serenity::Member) -> Self {
        Self {
            id: member.user.id,
            name: member.display_name().to_owned(),
            username: member.user.name.clone(),
            avatar_url: member.face(),
            joined_at: member.joined_at.map(|joined| joined.unix_timestamp()),
            roles: member.roles.clone(),
        }
    }
}

impl AppState {
    /// Every human member of the guild. Listing members needs the Server Members intent
    /// enabled in the developer portal, even though the bot doesn't use it on the gateway.
    async fn guild_members(
        &self,
        guild_id: serenity::GuildId,
    ) -> Result<Arc<Vec<MemberInfo>>, Error> {
        if let Some(members) = self.guild_members.get(&guild_id) {
            return Ok(members);
        }
        let mut members = Vec::new();
        let mut after = None;
        loop {
            let page = self
                .discord
                .get_guild_members(guild_id, Some(1000), after)
                .await?;
            after = page.last().map(|member| member.user.id.get());
            let full = page.len() == 1000;
            members.extend(
                page.iter()
                    .filter(|member| !member.user.bot)
                    .map(MemberInfo::from_member),
            );
            if !full {
                break;
            }
        }
        let members = Arc::new(members);
        self.guild_members.insert(guild_id, members.clone());
        Ok(members)
    }

    fn services(&self) -> verification::Services<'_> {
        verification::Services {
            db: &self.db,
            hypixel: &self.hypixel,
            mojang: &self.http_client,
            discord: &self.discord,
            cache: &self.cache,
        }
    }
}

pub struct Notice {
    /// `ok`, `warn` or `error`, used as the CSS class
    pub(super) kind: &'static str,
    pub(super) text: String,
}

pub struct MemberRow {
    pub(super) id: serenity::UserId,
    pub(super) name: String,
    pub(super) username: String,
    pub(super) avatar_url: String,
    /// `verified`, `role` (has the verified role but no stored account) or `none`
    pub(super) status: &'static str,
    pub(super) minecraft_name: Option<String>,
    pub(super) forced: bool,
    /// Raw values used for sorting, next to their display text
    joined_at: Option<i64>,
    pub(super) joined: String,
    verified_at: Option<i64>,
    pub(super) verified: String,
    guild_joined_at: Option<i64>,
    pub(super) guild_joined: String,
    weekly_xp: Option<u64>,
    pub(super) xp: String,
    pub(super) notice: Option<Notice>,
}

fn date_label(seconds: Option<i64>) -> String {
    seconds
        .and_then(|seconds| serenity::Timestamp::from_unix_timestamp(seconds).ok())
        .map(|date| date.to_string().chars().take(10).collect())
        .unwrap_or_default()
}

/// `1234567` → `1,234,567`
fn xp_label(xp: u64) -> String {
    let digits = xp.to_string();
    let mut label = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            label.push(',');
        }
        label.push(digit);
    }
    label
}

/// `guild` is the server's linked Hypixel guild, for the guild join date and XP
fn member_row(
    member: &MemberInfo,
    verified: Option<&VerifiedMember>,
    verified_role_id: Option<serenity::RoleId>,
    guild: Option<&hypixel::Guild>,
) -> MemberRow {
    let status = match verified {
        Some(_) => "verified",
        None if verified_role_id.is_some_and(|role| member.roles.contains(&role)) => "role",
        None => "none",
    };
    let stats = verified
        .zip(guild)
        .and_then(|(verified, guild)| guild.member_stats(&verified.minecraft_uuid));
    let verified_at = verified.map(|verified| verified.verified_at);
    let guild_joined_at = stats.as_ref().and_then(|stats| stats.joined_at);
    let weekly_xp = stats.as_ref().map(|stats| stats.weekly_xp);
    MemberRow {
        id: member.id,
        name: member.name.clone(),
        username: member.username.clone(),
        avatar_url: member.avatar_url.clone(),
        status,
        minecraft_name: verified.map(|verified| verified.minecraft_name.clone()),
        forced: verified.is_some_and(|verified| verified.forced_by.is_some()),
        joined_at: member.joined_at,
        joined: date_label(member.joined_at),
        verified_at,
        verified: date_label(verified_at),
        guild_joined_at,
        guild_joined: date_label(guild_joined_at),
        weekly_xp,
        xp: weekly_xp.map(xp_label).unwrap_or_default(),
        notice: None,
    }
}

/// Table columns as `(key, label)`, in display order. All can be sorted, and all but the
/// member column can be hidden.
const COLUMNS: [(&str, &str); 7] = [
    ("name", "Member"),
    ("minecraft", "Minecraft"),
    ("status", "Status"),
    ("joined", "Joined server"),
    ("verified", "Verified on"),
    ("guild_joined", "Joined guild"),
    ("xp", "Guild XP (7 days)"),
];

/// Hidden until the viewer turns them on, most servers don't track guild XP
const DEFAULT_HIDDEN: [&str; 1] = ["xp"];

/// Comma separated keys of the columns a viewer hid, set by the panel's script
pub const HIDDEN_COLUMNS_COOKIE: &str = "obot_hidden_columns";

pub fn hidden_columns(cookie: Option<&str>) -> Vec<&'static str> {
    match cookie {
        None => DEFAULT_HIDDEN.to_vec(),
        Some(value) => COLUMNS
            .iter()
            .map(|(key, _)| *key)
            .filter(|key| *key != "name" && value.split(',').any(|hidden| hidden == *key))
            .collect(),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Ascending,
    Descending,
}

/// A column header: clicking cycles ascending → descending → unsorted
pub struct SortHeader {
    pub(super) key: &'static str,
    pub(super) label: &'static str,
    /// Link to the next state of this column
    pub(super) url: String,
    /// `↕` unsorted, `↓` ascending, `↑` descending
    pub(super) arrow: &'static str,
    /// Value of the header's `aria-sort`
    pub(super) aria: &'static str,
    pub(super) hidden: bool,
}

pub struct MembersTable {
    pub(super) rows: Vec<MemberRow>,
    /// Why the list couldn't be loaded
    pub(super) error: Option<&'static str>,
    /// Why the guild columns are empty
    pub(super) guild_note: Option<&'static str>,
    pub(super) total: usize,
    pub(super) verified_count: usize,
    /// Members matching the search
    pub(super) matching: usize,
    pub(super) headers: Vec<SortHeader>,
    /// `hide-<key>` classes of the hidden columns, for the table element
    pub(super) hidden_classes: String,
    /// Current sort, kept by the search form: field key and `asc` or `desc`
    pub(super) sort: Option<(&'static str, &'static str)>,
    pub(super) search: String,
    pub(super) page: usize,
    pub(super) pages: usize,
    pub(super) prev_url: Option<String>,
    pub(super) next_url: Option<String>,
    pub(super) can_act: bool,
}

#[derive(Deserialize, Default)]
pub struct TableParams {
    sort: Option<String>,
    dir: Option<String>,
    q: Option<String>,
    // Flattened into the page's params, where serde only sees strings
    page: Option<String>,
}

fn page_url(
    guild_id: serenity::GuildId,
    sort: Option<(&str, &str)>,
    search: &str,
    page: usize,
) -> String {
    let mut params = Vec::new();
    if let Some((field, dir)) = sort {
        params.push(("sort", field.to_owned()));
        params.push(("dir", dir.to_owned()));
    }
    if !search.is_empty() {
        params.push(("q", search.to_owned()));
    }
    if page > 1 {
        params.push(("page", page.to_string()));
    }
    let query = params
        .iter()
        .map(|(key, value)| format!("{key}={}", urlencode(value)))
        .collect::<Vec<_>>()
        .join("&");
    if query.is_empty() {
        format!("/guilds/{guild_id}/verification#members")
    } else {
        format!("/guilds/{guild_id}/verification?{query}#members")
    }
}

fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn dir_param(direction: Direction) -> &'static str {
    match direction {
        Direction::Ascending => "asc",
        Direction::Descending => "desc",
    }
}

fn headers(
    guild_id: serenity::GuildId,
    sort: Option<(&'static str, Direction)>,
    search: &str,
    hidden: &[&str],
) -> Vec<SortHeader> {
    COLUMNS
        .iter()
        .map(|&(key, label)| {
            let current = sort.filter(|(field, _)| *field == key).map(|(_, dir)| dir);
            let next = match current {
                None => Some((key, "asc")),
                Some(Direction::Ascending) => Some((key, "desc")),
                Some(Direction::Descending) => None,
            };
            let (arrow, aria) = match current {
                None => ("↕", "none"),
                Some(Direction::Ascending) => ("↓", "ascending"),
                Some(Direction::Descending) => ("↑", "descending"),
            };
            SortHeader {
                key,
                label,
                url: page_url(guild_id, next, search, 1),
                arrow,
                aria,
                hidden: hidden.contains(&key),
            }
        })
        .collect()
}

fn status_order(status: &str) -> u8 {
    match status {
        "verified" => 0,
        "role" => 1,
        _ => 2,
    }
}

/// Sorts rows by a column. Members without a value (no Minecraft account, not in the guild…)
/// stay at the end in both directions, ties are broken by name.
fn sort_rows(rows: &mut [MemberRow], field: &str, direction: Direction) {
    let name = |row: &MemberRow| row.name.to_lowercase();
    let missing = |row: &MemberRow| match field {
        "minecraft" => row.minecraft_name.is_none(),
        "joined" => row.joined_at.is_none(),
        "verified" => row.verified_at.is_none(),
        "guild_joined" => row.guild_joined_at.is_none(),
        "xp" => row.weekly_xp.is_none(),
        _ => false,
    };
    rows.sort_by(|a, b| {
        let main = match field {
            "minecraft" => a
                .minecraft_name
                .as_deref()
                .map(str::to_lowercase)
                .cmp(&b.minecraft_name.as_deref().map(str::to_lowercase)),
            "status" => status_order(a.status).cmp(&status_order(b.status)),
            "joined" => a.joined_at.cmp(&b.joined_at),
            "verified" => a.verified_at.cmp(&b.verified_at),
            "guild_joined" => a.guild_joined_at.cmp(&b.guild_joined_at),
            "xp" => a.weekly_xp.cmp(&b.weekly_xp),
            _ => name(a).cmp(&name(b)),
        };
        let main = match direction {
            Direction::Ascending => main,
            Direction::Descending => main.reverse(),
        };
        missing(a)
            .cmp(&missing(b))
            .then(main)
            .then_with(|| name(a).cmp(&name(b)))
    });
}

impl AppState {
    /// The server's linked Hypixel guild. Guild columns are extras, so a Hypixel failure
    /// empties them instead of failing the page.
    async fn linked_guild(
        &self,
        config: &GuildConfig,
    ) -> Result<Option<Arc<hypixel::Guild>>, &'static str> {
        let Some(link) = &config.hypixel_guild else {
            return Err("Link a Hypixel guild to fill the guild columns.");
        };
        match self.hypixel.guild(&link.id).await {
            Ok(guild) => Ok(guild),
            Err(error) => {
                eprintln!("Could not load Hypixel guild {}: {error}", link.id);
                Err("Couldn't load the guild from Hypixel, the guild columns are empty.")
            }
        }
    }
}

pub async fn table(
    state: &AppState,
    guild_id: serenity::GuildId,
    config: &GuildConfig,
    params: TableParams,
    hidden: Vec<&'static str>,
) -> Result<MembersTable, Error> {
    let sort = COLUMNS
        .iter()
        .find(|(key, _)| Some(*key) == params.sort.as_deref())
        .map(|&(key, _)| {
            let direction = match params.dir.as_deref() {
                Some("desc") => Direction::Descending,
                _ => Direction::Ascending,
            };
            (key, direction)
        });
    let search = params.q.unwrap_or_default().trim().to_owned();
    let mut table = MembersTable {
        rows: Vec::new(),
        error: None,
        guild_note: None,
        total: 0,
        verified_count: 0,
        matching: 0,
        headers: headers(guild_id, sort, &search, &hidden),
        hidden_classes: hidden
            .iter()
            .map(|key| format!("hide-{key}"))
            .collect::<Vec<_>>()
            .join(" "),
        sort: sort.map(|(field, direction)| (field, dir_param(direction))),
        search,
        page: 1,
        pages: 1,
        prev_url: None,
        next_url: None,
        can_act: config.verified_role_id.is_some(),
    };

    let members = match state.guild_members(guild_id).await {
        Ok(members) => members,
        Err(error) => {
            eprintln!("Could not list members of {guild_id}: {error}");
            table.error = Some(
                "Couldn't list the server members. Enable Server Members Intent in the Discord \
                 developer portal (Bot → Privileged Gateway Intents), then reload.",
            );
            return Ok(table);
        }
    };
    let verified: HashMap<_, _> = config::list_verified_members(&state.db, guild_id).await?;
    let guild = match state.linked_guild(config).await {
        Ok(guild) => guild,
        Err(note) => {
            table.guild_note = Some(note);
            None
        }
    };

    let mut rows: Vec<MemberRow> = members
        .iter()
        .map(|member| {
            member_row(
                member,
                verified.get(&member.id),
                config.verified_role_id,
                guild.as_deref(),
            )
        })
        .collect();
    table.total = rows.len();
    table.verified_count = rows.iter().filter(|row| row.status == "verified").count();

    let needle = table.search.to_lowercase();
    if !needle.is_empty() {
        rows.retain(|row| {
            [
                Some(&row.name),
                Some(&row.username),
                row.minecraft_name.as_ref(),
            ]
            .into_iter()
            .flatten()
            .any(|text| text.to_lowercase().contains(&needle))
        });
    }
    // Unsorted keeps Discord's order
    if let Some((field, direction)) = sort {
        sort_rows(&mut rows, field, direction);
    }

    table.matching = rows.len();
    table.pages = rows.len().div_ceil(PAGE_SIZE).max(1);
    let page = params.page.and_then(|page| page.parse().ok()).unwrap_or(1);
    table.page = page.clamp(1, table.pages);
    table.rows = rows
        .into_iter()
        .skip((table.page - 1) * PAGE_SIZE)
        .take(PAGE_SIZE)
        .collect();
    table.prev_url =
        (table.page > 1).then(|| page_url(guild_id, table.sort, &table.search, table.page - 1));
    table.next_url = (table.page < table.pages)
        .then(|| page_url(guild_id, table.sort, &table.search, table.page + 1));
    Ok(table)
}

#[derive(Template)]
#[template(path = "member_row.html")]
struct RowFragment {
    guild_id: serenity::GuildId,
    can_act: bool,
    row: MemberRow,
}

/// Why an action on a member was refused, shown to the admin
struct Refused(String);

impl From<&str> for Refused {
    fn from(reason: &str) -> Self {
        Self(reason.to_owned())
    }
}

fn describe(
    state: &AppState,
    guild_id: serenity::GuildId,
    prefix: String,
    outcome: &Outcome,
    nothing_changed: &str,
) -> Notice {
    let names = |roles: &std::collections::BTreeSet<serenity::RoleId>| {
        roles
            .iter()
            .map(|role| format!("@{}", state.role_chip(guild_id, *role).name))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut parts = vec![prefix];
    if !outcome.added.is_empty() {
        parts.push(format!("Added {}.", names(&outcome.added)));
    }
    if !outcome.removed.is_empty() {
        parts.push(format!("Removed {}.", names(&outcome.removed)));
    }
    let mut kind = "ok";
    match &outcome.nickname {
        NicknameChange::Unchanged => {}
        NicknameChange::Set(nickname) => parts.push(format!("Nickname set to {nickname}.")),
        NicknameChange::Reset => parts.push("Nickname reset.".to_owned()),
        NicknameChange::Skipped { nickname, why } => {
            kind = "warn";
            parts.push(format!("Nickname not changed to {nickname}: {why}."));
        }
    }
    let nickname_changed = !matches!(outcome.nickname, NicknameChange::Unchanged);
    if outcome.added.is_empty() && outcome.removed.is_empty() && !nickname_changed {
        parts.push(nothing_changed.to_owned());
    }
    Notice {
        kind,
        text: parts.join(" "),
    }
}

async fn row_for(
    state: &AppState,
    guild_id: serenity::GuildId,
    member: &serenity::Member,
    config: &GuildConfig,
    notice: Notice,
) -> Result<Response, AppError> {
    let verified = config::get_verified_member(&state.db, guild_id, member.user.id).await?;
    let guild = state.linked_guild(config).await.ok().flatten();
    let mut row = member_row(
        &MemberInfo::from_member(member),
        verified.as_ref(),
        config.verified_role_id,
        guild.as_deref(),
    );
    row.notice = Some(notice);
    let fragment = RowFragment {
        guild_id,
        can_act: config.verified_role_id.is_some(),
        row,
    };
    Ok(Html(fragment.render()?).into_response())
}

/// Access and target checks shared by both actions
async fn prepare(
    state: &AppState,
    user_id: serenity::UserId,
    guild_id: NonZeroU64,
    target_id: NonZeroU64,
) -> Result<Result<(Access, serenity::Member, GuildConfig), Response>, AppError> {
    let Some(access) = state
        .access(serenity::GuildId::from(guild_id), user_id)
        .await
    else {
        return Ok(Err((
            StatusCode::FORBIDDEN,
            "You can't manage this server anymore.",
        )
            .into_response()));
    };
    let target_id = serenity::UserId::from(target_id);
    let Ok(member) = state.discord.get_member(access.guild_id, target_id).await else {
        return Ok(Err(
            (StatusCode::NOT_FOUND, "This member left the server.").into_response()
        ));
    };
    if member.user.bot {
        return Ok(Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            "Bots can't be verified.",
        )
            .into_response()));
    }
    let config = config::get_config(&state.db, access.guild_id).await?;
    Ok(Ok((access, member, config)))
}

async fn run_sync(
    state: &AppState,
    access: &Access,
    member: &serenity::Member,
    config: &GuildConfig,
    profile: &hypixel::MojangProfile,
    player: &Player,
    verified_by: serenity::UserId,
) -> Result<Result<Outcome, Refused>, Error> {
    let Some(verified_role_id) = config.verified_role_id else {
        return Ok(Err("Pick a verified role first.".into()));
    };
    let outcome = verification::sync_member(
        &state.services(),
        access.guild_id,
        member,
        verified_role_id,
        config.unverified_role_id,
        config.hypixel_guild.as_ref().map(|link| link.id.as_str()),
        profile,
        player,
        Record::Verified {
            by: Some(verified_by),
        },
    )
    .await?;
    state.guild_members.remove(&access.guild_id);
    Ok(Ok(outcome))
}

/// Refreshes roles and nickname from the account the member proved before
pub async fn reverify(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path((guild_id, target_id)): Path<(NonZeroU64, NonZeroU64)>,
) -> Result<Response, AppError> {
    let (access, member, config) = match prepare(&state, user.id, guild_id, target_id).await? {
        Ok(prepared) => prepared,
        Err(response) => return Ok(response),
    };
    let error = |text: String| Notice {
        kind: "error",
        text,
    };

    let Some(stored) =
        config::get_verified_member(&state.db, access.guild_id, member.user.id).await?
    else {
        let notice = error("This member has no linked account, use Verify instead.".to_owned());
        return row_for(&state, access.guild_id, &member, &config, notice).await;
    };
    let notice = match verification::refresh_member(
        &state.services(),
        access.guild_id,
        &member,
        &config,
        &stored,
    )
    .await?
    {
        Ok((profile, outcome)) => describe(
            &state,
            access.guild_id,
            format!("Re-verified as {}.", profile.name),
            &outcome,
            "Everything was already up to date.",
        ),
        Err(RefreshFailure::NotSetUp) => error("Pick a verified role first.".to_owned()),
        Err(RefreshFailure::AccountGone) => error(format!(
            "The Minecraft account {} no longer exists.",
            stored.minecraft_name
        )),
    };
    state.guild_members.remove(&access.guild_id);
    row_for(&state, access.guild_id, &member, &config, notice).await
}

#[derive(Deserialize)]
pub struct VerifyForm {
    username: String,
}

fn refused(reason: impl Into<String>) -> Response {
    (StatusCode::UNPROCESSABLE_ENTITY, reason.into()).into_response()
}

/// Verifies a member on their behalf, with the same ownership check as `/verify`: the Minecraft
/// account must have the member's Discord linked on Hypixel. The member comes from the Discord
/// API, so an admin can't make the check pass for someone else.
pub async fn admin_verify(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path((guild_id, target_id)): Path<(NonZeroU64, NonZeroU64)>,
    Form(form): Form<VerifyForm>,
) -> Result<Response, AppError> {
    let (access, member, config) = match prepare(&state, user.id, guild_id, target_id).await? {
        Ok(prepared) => prepared,
        Err(response) => return Ok(response),
    };

    let username = form.username.trim();
    if username.is_empty() {
        return Ok(refused("Enter a Minecraft username."));
    }
    let Some(profile) = hypixel::fetch_mojang_profile(&state.http_client, username).await? else {
        return Ok(refused(format!(
            "No Minecraft account is named {username}."
        )));
    };
    let Some(player) =
        verification::player_for_proof(&state.hypixel, &profile.id, &member.user).await?
    else {
        return Ok(refused(format!(
            "{} has never joined Hypixel.",
            profile.name
        )));
    };
    match player.discord.as_deref() {
        None => {
            return Ok(refused(format!(
                "{} has no Discord linked on Hypixel. Ask {} to link @{} in the game's \
                 Social Media menu first.",
                profile.name, member.user.name, member.user.name
            )));
        }
        Some(linked) if !verification::is_same_user(&member.user, linked) => {
            return Ok(refused(format!(
                "{} is linked to the Discord {linked} on Hypixel, not to @{}.",
                profile.name, member.user.name
            )));
        }
        Some(_) => {}
    }
    match verification::check_unique(
        &state.services(),
        access.guild_id,
        member.user.id,
        &profile.id,
    )
    .await?
    {
        None => {}
        Some(Conflict::MemberLinked { minecraft_name }) => {
            return Ok(refused(format!(
                "@{} is already verified as {minecraft_name}. Remove their verification first.",
                member.user.name
            )));
        }
        Some(Conflict::AccountLinked { name }) => {
            return Ok(refused(format!(
                "{} is already linked to another member ({name}).",
                profile.name
            )));
        }
    }

    match run_sync(
        &state, &access, &member, &config, &profile, &player, user.id,
    )
    .await?
    {
        Ok(outcome) => {
            let notice = describe(
                &state,
                access.guild_id,
                format!("Verified as {} by you.", profile.name),
                &outcome,
                "Everything was already up to date.",
            );
            row_for(&state, access.guild_id, &member, &config, notice).await
        }
        Err(Refused(reason)) => Ok(refused(reason)),
    }
}

/// Removing roles is managing the member, so like on Discord it needs a higher role than theirs
fn check_hierarchy(
    state: &AppState,
    access: &Access,
    target: &serenity::Member,
    actor: serenity::UserId,
) -> Result<(), Refused> {
    let Some(ceiling) = access.role_ceiling else {
        return Ok(());
    };
    if target.user.id == actor {
        return Ok(());
    }
    let guild = state
        .cache
        .guild(access.guild_id)
        .ok_or("The bot couldn't load this server.")?;
    if guild.owner_id == target.user.id {
        return Err("Only the server owner can remove their own verification.".into());
    }
    let target_position = guild
        .member_highest_role(target)
        .map_or(0, |role| role.position);
    if target_position >= ceiling {
        return Err("This member's highest role is at or above yours.".into());
    }
    Ok(())
}

/// Removes a member's verification roles, resets their nickname and forgets their account
pub async fn unverify(
    State(state): State<Arc<AppState>>,
    LoggedIn(user): LoggedIn,
    Path((guild_id, target_id)): Path<(NonZeroU64, NonZeroU64)>,
) -> Result<Response, AppError> {
    let (access, member, config) = match prepare(&state, user.id, guild_id, target_id).await? {
        Ok(prepared) => prepared,
        Err(response) => return Ok(response),
    };
    let error = |text: String| Notice {
        kind: "error",
        text,
    };
    if let Err(Refused(reason)) = check_hierarchy(&state, &access, &member, user.id) {
        return row_for(&state, access.guild_id, &member, &config, error(reason)).await;
    }

    let outcome = verification::unverify_member(
        &state.services(),
        access.guild_id,
        &member,
        config.verified_role_id,
        config.unverified_role_id,
    )
    .await?;
    state.guild_members.remove(&access.guild_id);
    let notice = describe(
        &state,
        access.guild_id,
        "Verification removed.".to_owned(),
        &outcome,
        "They had no verification roles left.",
    );
    // The fresh member reflects the removed roles, so the row shows them as not verified
    let member = state
        .discord
        .get_member(access.guild_id, member.user.id)
        .await
        .unwrap_or(member);
    row_for(&state, access.guild_id, &member, &config, notice).await
}

#[cfg(test)]
pub(super) mod tests;
