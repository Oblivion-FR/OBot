use poise::serenity_prelude as serenity;

use crate::hypixel::{self, LinkedDiscord};
use crate::{Context, Error};

/// Check that the bot is alive
#[poise::command(slash_command, ephemeral)]
pub async fn healthcheck(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Hi!").await?;
    Ok(())
}

/// Hypixel stores whatever the player typed, which may be a legacy `name#1234` tag
fn is_same_user(user: &serenity::User, linked: &str) -> bool {
    let linked = linked.trim();
    linked.eq_ignore_ascii_case(&user.name) || linked.eq_ignore_ascii_case(&user.tag())
}

/// Get the verified role by proving you own a Minecraft account
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

    let Some(profile) = hypixel::fetch_mojang_profile(&data.http, &pseudo).await? else {
        ctx.say(format!("No Minecraft account is named `{pseudo}`."))
            .await?;
        return Ok(());
    };

    let linked =
        hypixel::fetch_linked_discord(&data.http, &data.hypixel_api_key, &profile.id).await?;
    let message = match linked {
        LinkedDiscord::UnknownPlayer => {
            format!("`{}` has never joined Hypixel.", profile.name)
        }
        LinkedDiscord::NotLinked => format!(
            "`{}` has no Discord linked on Hypixel. In game, open your profile \
             → Social Media → Discord and enter `{}`, then try again.",
            profile.name, author.name
        ),
        LinkedDiscord::Linked(discord) if !is_same_user(author, &discord) => format!(
            "`{}` is linked to the Discord `{discord}`, not to you (`{}`).",
            profile.name, author.name
        ),
        LinkedDiscord::Linked(_) => {
            let reason = format!("Verified as {}", profile.name);
            ctx.http()
                .add_member_role(guild_id, author.id, data.verified_role_id, Some(&reason))
                .await?;
            ctx.http()
                .remove_member_role(guild_id, author.id, data.unverified_role_id, Some(&reason))
                .await?;
            format!("You are now verified as `{}`!", profile.name)
        }
    };

    ctx.say(message).await?;
    Ok(())
}
