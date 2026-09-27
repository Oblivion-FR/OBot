use poise::serenity_prelude as serenity;
use std::collections::BTreeSet;

use crate::config;
use crate::hypixel;
use crate::i18n::{Lang, tr};
use crate::verification::{self, Conflict, NicknameChange, Record, Services};
use crate::version;
use crate::{Context, Error};

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

fn mention_roles(roles: &BTreeSet<serenity::RoleId>) -> String {
    roles
        .iter()
        .map(|role| format!("<@&{role}>"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[poise::command(slash_command, guild_only, ephemeral)]
pub async fn verify(ctx: Context<'_>, username: String) -> Result<(), Error> {
    let lang = lang(ctx);
    // The API calls can exceed Discord's 3 second reply window
    ctx.defer_ephemeral().await?;

    let data = ctx.data();
    let guild_id = ctx.guild_id().ok_or("verify is guild only")?;
    let author = ctx.author();

    let config = config::get_config(&data.db, guild_id).await?;
    let Some(verified_role_id) = config.verified_role_id else {
        ctx.say(lang.t("verify-not-set-up")).await?;
        return Ok(());
    };

    let Some(profile) = hypixel::fetch_mojang_profile(&data.http, &username).await? else {
        ctx.say(tr!(
            lang,
            "verify-unknown-account",
            name = username.as_str()
        ))
        .await?;
        return Ok(());
    };

    let Some(player) = verification::player_for_proof(&data.hypixel, &profile.id, author).await?
    else {
        ctx.say(tr!(
            lang,
            "verify-never-joined",
            name = profile.name.as_str()
        ))
        .await?;
        return Ok(());
    };
    match &player.discord {
        None => {
            ctx.say(tr!(
                lang,
                "verify-no-discord-linked",
                name = profile.name.as_str(),
                discord = author.name.as_str()
            ))
            .await?;
            return Ok(());
        }
        Some(discord) if !verification::is_same_user(author, discord) => {
            ctx.say(tr!(
                lang,
                "verify-linked-to-someone-else",
                name = profile.name.as_str(),
                linked = discord.as_str(),
                discord = author.name.as_str()
            ))
            .await?;
            return Ok(());
        }
        Some(_) => {}
    }

    let member = match ctx.author_member().await {
        Some(member) => member.into_owned(),
        None => guild_id.member(ctx, author.id).await?,
    };
    let services = Services {
        db: &data.db,
        hypixel: &data.hypixel,
        mojang: &data.http,
        discord: ctx.http(),
        cache: ctx.cache(),
    };
    match verification::check_unique(&services, guild_id, author.id, &profile.id).await? {
        None => {}
        Some(Conflict::MemberLinked { minecraft_name }) => {
            ctx.say(tr!(
                lang,
                "verify-already-verified",
                name = minecraft_name.as_str()
            ))
            .await?;
            return Ok(());
        }
        Some(Conflict::AccountLinked { name }) => {
            ctx.say(tr!(
                lang,
                "verify-account-taken",
                name = profile.name.as_str(),
                member = name.as_str()
            ))
            .await?;
            return Ok(());
        }
    }
    let outcome = verification::sync_member(
        &services,
        guild_id,
        &member,
        verified_role_id,
        config.unverified_role_id,
        config.hypixel_guild.as_ref().map(|link| link.id.as_str()),
        &profile,
        &player,
        Record::Verified { by: None },
    )
    .await?;

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
    ctx.say(lines.join("\n")).await?;
    Ok(())
}
