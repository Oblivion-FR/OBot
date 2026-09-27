use poise::serenity_prelude as serenity;
use std::collections::BTreeSet;

use crate::config;
use crate::hypixel;
use crate::verification::{self, Conflict, NicknameChange, Services};
use crate::version;
use crate::{Context, Error};

/// Check that the bot is alive
#[poise::command(slash_command, ephemeral)]
pub async fn healthcheck(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Hi!").await?;
    Ok(())
}

/// Which version of OBot runs, with a link to its source
#[poise::command(slash_command, ephemeral)]
pub async fn version(ctx: Context<'_>) -> Result<(), Error> {
    // `<…>` keeps Discord from adding link previews
    let commit = match version::commit_url() {
        Some(url) => format!("[{}](<{url}>)", version::COMMIT),
        None => version::COMMIT.to_owned(),
    };
    ctx.say(format!(
        "OBot v{} ({commit})\nSource: <{}>",
        version::NUMBER,
        version::REPOSITORY
    ))
    .await?;
    Ok(())
}

fn mention_roles(roles: &BTreeSet<serenity::RoleId>) -> String {
    roles
        .iter()
        .map(|role| format!("<@&{role}>"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Get your roles by proving you own a Minecraft account
#[poise::command(slash_command, guild_only, ephemeral)]
pub async fn verify(
    ctx: Context<'_>,
    #[description = "Your Minecraft username"] pseudo: String,
) -> Result<(), Error> {
    // The API calls can exceed Discord's 3 second reply window
    ctx.defer_ephemeral().await?;

    let data = ctx.data();
    let guild_id = ctx.guild_id().ok_or("verify is guild only")?;
    let author = ctx.author();

    let config = config::get_config(&data.db, guild_id).await?;
    let Some(verified_role_id) = config.verified_role_id else {
        ctx.say("Verification is not set up on this server yet, ask an admin to configure it in the panel.")
            .await?;
        return Ok(());
    };

    let Some(profile) = hypixel::fetch_mojang_profile(&data.http, &pseudo).await? else {
        ctx.say(format!("No Minecraft account is named `{pseudo}`."))
            .await?;
        return Ok(());
    };

    let Some(player) = verification::player_for_proof(&data.hypixel, &profile.id, author).await?
    else {
        ctx.say(format!("`{}` has never joined Hypixel.", profile.name))
            .await?;
        return Ok(());
    };
    match &player.discord {
        None => {
            ctx.say(format!(
                "`{}` has no Discord linked on Hypixel. In game, open your profile \
                 → Social Media → Discord and enter `{}`, then try again.",
                profile.name, author.name
            ))
            .await?;
            return Ok(());
        }
        Some(discord) if !verification::is_same_user(author, discord) => {
            ctx.say(format!(
                "`{}` is linked to the Discord `{discord}`, not to you (`{}`).",
                profile.name, author.name
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
        discord: ctx.http(),
        cache: ctx.cache(),
    };
    match verification::check_unique(&services, guild_id, author.id, &profile.id).await? {
        None => {}
        Some(Conflict::MemberLinked { minecraft_name }) => {
            ctx.say(format!(
                "You are already verified as `{minecraft_name}`. Ask an admin to remove your \
                 verification to link another account."
            ))
            .await?;
            return Ok(());
        }
        Some(Conflict::AccountLinked { name }) => {
            ctx.say(format!(
                "`{}` is already linked to another member of this server ({name}).",
                profile.name
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
        None,
    )
    .await?;

    let mut message = format!("You are verified as `{}`!", profile.name);
    if !outcome.added.is_empty() {
        message += &format!("\nRoles added: {}", mention_roles(&outcome.added));
    }
    if !outcome.removed.is_empty() {
        message += &format!("\nRoles removed: {}", mention_roles(&outcome.removed));
    }
    match outcome.nickname {
        // Only removing a verification resets nicknames
        NicknameChange::Unchanged | NicknameChange::Reset => {}
        NicknameChange::Set(nickname) => message += &format!("\nNickname set to `{nickname}`"),
        NicknameChange::Skipped { nickname, why } => {
            message += &format!("\n⚠️ Nickname not changed to `{nickname}`: {why}.")
        }
    }
    ctx.say(message).await?;
    Ok(())
}
