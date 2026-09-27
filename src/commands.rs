use poise::serenity_prelude as serenity;
use std::collections::BTreeSet;

use crate::config;
use crate::hypixel;
use crate::i18n::{Lang, tr};
use crate::server_log::{self, Event, Refusal};
use crate::verification::{self, Conflict, NicknameChange, Record, Services};
use crate::version;
use crate::{Context, Data, Error};

/// Replies in the language of the member's Discord client, English when OBot doesn't speak it
fn lang(ctx: Context<'_>) -> Lang {
    ctx.locale().and_then(Lang::from_tag).unwrap_or_default()
}

// Descriptions come from the `command-…` messages of locales/*/commands.ftl

#[poise::command(slash_command, ephemeral)]
pub async fn healthcheck(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say(lang(ctx).t("healthcheck-reply")).await?;
    Ok(())
}

#[poise::command(slash_command, ephemeral)]
pub async fn version(ctx: Context<'_>) -> Result<(), Error> {
    // `<…>` keeps Discord from adding link previews
    let commit = match version::commit_url() {
        Some(url) => format!("[{}](<{url}>)", version::COMMIT),
        None => version::COMMIT.to_owned(),
    };
    let reply = tr!(
        lang(ctx),
        "version-reply",
        version = version::NUMBER,
        commit = commit,
        repository = version::REPOSITORY
    );
    ctx.say(reply).await?;
    Ok(())
}

/// How a verified member is described, with when and by whom they were verified
fn describe_verified(
    lang: Lang,
    user_id: serenity::UserId,
    stored: &config::VerifiedMember,
) -> String {
    // Discord shows these timestamps in each reader's own time zone and language
    let since = format!("<t:{}:D>", stored.verified_at);
    let member = format!("<@{user_id}>");
    match stored.forced_by {
        Some(admin) => tr!(
            lang,
            "whois-verified-by",
            member = member,
            name = stored.minecraft_name.as_str(),
            uuid = stored.minecraft_uuid.as_str(),
            admin = format!("<@{admin}>"),
            since = since
        ),
        None => tr!(
            lang,
            "whois-verified",
            member = member,
            name = stored.minecraft_name.as_str(),
            uuid = stored.minecraft_uuid.as_str(),
            since = since
        ),
    }
}

/// Moderators only by default: server admins can open it to others in the integration settings
#[poise::command(
    slash_command,
    guild_only,
    ephemeral,
    default_member_permissions = "MANAGE_ROLES"
)]
pub async fn whois(
    ctx: Context<'_>,
    member: Option<serenity::User>,
    minecraft: Option<String>,
) -> Result<(), Error> {
    let lang = lang(ctx);
    let data = ctx.data();
    let guild_id = ctx.guild_id().ok_or("whois is guild only")?;
    if member.is_none() && minecraft.is_none() {
        ctx.say(lang.t("whois-missing")).await?;
        return Ok(());
    }

    let mut lines = Vec::new();
    if let Some(user) = member {
        lines.push(
            match config::get_verified_member(&data.db, guild_id, user.id).await? {
                Some(stored) => describe_verified(lang, user.id, &stored),
                None => tr!(
                    lang,
                    "whois-not-verified",
                    member = format!("<@{}>", user.id)
                ),
            },
        );
    }
    if let Some(username) = minecraft {
        let owner = match hypixel::fetch_mojang_profile(&data.http, username.trim()).await? {
            None => Err(tr!(lang, "whois-unknown-account", name = username.trim())),
            Some(profile) => {
                match config::find_account_owner(&data.db, guild_id, &profile.id).await? {
                    None => Err(tr!(
                        lang,
                        "whois-account-free",
                        name = profile.name.as_str()
                    )),
                    Some(owner) => Ok(owner),
                }
            }
        };
        lines.push(match owner {
            Err(line) => line,
            Ok(owner) => match config::get_verified_member(&data.db, guild_id, owner).await? {
                Some(stored) => describe_verified(lang, owner, &stored),
                None => tr!(lang, "whois-not-verified", member = format!("<@{owner}>")),
            },
        });
    }
    ctx.say(lines.join("\n")).await?;
    Ok(())
}

fn mention_roles(roles: &BTreeSet<serenity::RoleId>) -> String {
    roles
        .iter()
        .map(|role| format!("<@&{role}>"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[poise::command(slash_command, guild_only, ephemeral)]
pub async fn verify(ctx: Context<'_>, username: String) -> Result<(), Error> {
    // The API calls can exceed Discord's 3 second reply window
    ctx.defer_ephemeral().await?;
    let guild_id = ctx.guild_id().ok_or("verify is guild only")?;
    let member = match ctx.author_member().await {
        Some(member) => member.into_owned(),
        None => guild_id.member(ctx, ctx.author().id).await?,
    };
    let serenity_ctx = ctx.serenity_context();
    let reply = verify_member(
        &serenity_ctx.http,
        &serenity_ctx.cache,
        ctx.data(),
        guild_id,
        &member,
        &username,
        lang(ctx),
    )
    .await?;
    ctx.say(reply).await?;
    Ok(())
}

/// A member verifying themselves, from `/verify` or the verify button: checks the proof, applies
/// it and tells the log channel. Returns the reply to the member, in their language.
pub async fn verify_member(
    discord: &serenity::Http,
    cache: &serenity::Cache,
    data: &Data,
    guild_id: serenity::GuildId,
    member: &serenity::Member,
    username: &str,
    lang: Lang,
) -> Result<String, Error> {
    let author = &member.user;
    let config = config::get_config(&data.db, guild_id).await?;
    let Some(verified_role_id) = config.verified_role_id else {
        return Ok(lang.t("verify-not-set-up"));
    };

    let username = username.trim();
    let Some(profile) = hypixel::fetch_mojang_profile(&data.http, username).await? else {
        return Ok(tr!(lang, "verify-unknown-account", name = username));
    };
    let refused = |reason| {
        server_log::post(
            discord,
            &config,
            Event::Refused {
                member: author.id,
                name: &profile.name,
                reason,
            },
        )
    };

    let Some(player) = verification::player_for_proof(&data.hypixel, &profile.id, author).await?
    else {
        refused(Refusal::NeverJoined).await;
        return Ok(tr!(
            lang,
            "verify-never-joined",
            name = profile.name.as_str()
        ));
    };
    match &player.discord {
        None => {
            refused(Refusal::NoDiscordLinked).await;
            return Ok(tr!(
                lang,
                "verify-no-discord-linked",
                name = profile.name.as_str(),
                discord = author.name.as_str()
            ));
        }
        Some(linked) if !verification::is_same_user(author, linked) => {
            refused(Refusal::LinkedElsewhere { linked }).await;
            return Ok(tr!(
                lang,
                "verify-linked-to-someone-else",
                name = profile.name.as_str(),
                linked = linked.as_str(),
                discord = author.name.as_str()
            ));
        }
        Some(_) => {}
    }

    let services = Services {
        db: &data.db,
        hypixel: &data.hypixel,
        mojang: &data.http,
        discord,
        cache,
    };
    match verification::check_unique(&services, guild_id, author.id, &profile.id).await? {
        None => {}
        Some(Conflict::MemberLinked { minecraft_name }) => {
            refused(Refusal::AlreadyVerified {
                current: &minecraft_name,
            })
            .await;
            return Ok(tr!(
                lang,
                "verify-already-verified",
                name = minecraft_name.as_str()
            ));
        }
        Some(Conflict::AccountLinked { name }) => {
            refused(Refusal::AccountTaken { owner: &name }).await;
            return Ok(tr!(
                lang,
                "verify-account-taken",
                name = profile.name.as_str(),
                member = name.as_str()
            ));
        }
    }
    let outcome = verification::sync_member(
        &services,
        guild_id,
        member,
        verified_role_id,
        config.unverified_role_id,
        config.hypixel_guild.as_ref().map(|link| link.id.as_str()),
        &profile,
        &player,
        Record::Verified { by: None },
    )
    .await?;
    server_log::post(
        discord,
        &config,
        Event::Verified {
            member: author.id,
            name: &profile.name,
            by: None,
            outcome: &outcome,
        },
    )
    .await;

    let mut lines = vec![tr!(lang, "verify-done", name = profile.name.as_str())];
    if !outcome.added.is_empty() {
        lines.push(tr!(
            lang,
            "verify-roles-added",
            roles = mention_roles(&outcome.added)
        ));
    }
    if !outcome.removed.is_empty() {
        lines.push(tr!(
            lang,
            "verify-roles-removed",
            roles = mention_roles(&outcome.removed)
        ));
    }
    match outcome.nickname {
        // Only removing a verification resets nicknames
        NicknameChange::Unchanged | NicknameChange::Reset => {}
        NicknameChange::Set(nickname) => {
            lines.push(tr!(lang, "verify-nickname-set", nickname = nickname))
        }
        NicknameChange::Skipped { nickname, why } => lines.push(tr!(
            lang,
            "verify-nickname-skipped",
            nickname = nickname,
            reason = lang.t(why)
        )),
    }
    Ok(lines.join("\n"))
}
