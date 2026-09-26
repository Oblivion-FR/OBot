use poise::serenity_prelude as serenity;
use std::collections::BTreeSet;

use crate::config::{self, RuleKind};
use crate::hypixel::{self, GuildQuery};
use crate::nickname;
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

/// Discord never lets bots rename the server owner, nor members whose highest role is at or
/// above the bot's. The error is shown to the member.
fn can_rename(ctx: Context<'_>, member: &serenity::Member) -> Result<(), &'static str> {
    let Some(guild) = ctx.guild() else {
        return Err("the bot couldn't load this server");
    };
    if guild.owner_id == member.user.id {
        return Err("bots can't rename the server owner");
    }
    let bot_id = ctx.cache().current_user().id;
    // Without the bot's member in cache, let Discord decide
    let Some(bot) = guild.members.get(&bot_id) else {
        return Ok(());
    };
    let top_position = |member| {
        guild
            .member_highest_role(member)
            .map_or(0, |role| role.position)
    };
    if top_position(bot) > top_position(member) {
        Ok(())
    } else {
        Err("your highest role is at or above the bot's")
    }
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

    let Some(player) =
        hypixel::fetch_player(&data.http, &data.hypixel_api_key, &profile.id).await?
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
        Some(discord) if !is_same_user(author, discord) => {
            ctx.say(format!(
                "`{}` is linked to the Discord `{discord}`, not to you (`{}`).",
                profile.name, author.name
            ))
            .await?;
            return Ok(());
        }
        Some(_) => {}
    }

    let rules = config::list_rules(&data.db, guild_id).await?;
    let nickname_format = config::get_nickname_format(&data.db, guild_id).await?;
    let needs_guild = rules.iter().any(|rule| rule.kind != RuleKind::HypixelRank)
        || (nickname_format.enabled && nickname_format.needs_guild());
    // Only the linked Hypixel guild counts, being in another one is the same as being in none
    let player_guild = match &config.hypixel_guild {
        Some(linked) if needs_guild => {
            let query = GuildQuery::Player(&profile.id);
            hypixel::fetch_guild(&data.http, &data.hypixel_api_key, query)
                .await?
                .filter(|guild| guild.id == linked.id)
        }
        _ => None,
    };
    let guild_rank = player_guild
        .as_ref()
        .and_then(|guild| guild.member_rank(&profile.id));

    // Roles referenced by rules are kept in sync: granted when the rule matches, removed otherwise
    let mut wanted = BTreeSet::from([verified_role_id]);
    let mut unwanted = BTreeSet::new();
    for rule in &rules {
        let matches = match rule.kind {
            RuleKind::HypixelRank => player.rank.as_deref() == Some(rule.value.as_str()),
            RuleKind::GuildMember => guild_rank.is_some(),
            RuleKind::GuildRank => {
                guild_rank.is_some_and(|rank| rank.eq_ignore_ascii_case(&rule.value))
            }
        };
        if matches {
            wanted.insert(rule.role_id);
        } else {
            unwanted.insert(rule.role_id);
        }
    }
    unwanted.extend(config.unverified_role_id);
    unwanted.retain(|role| !wanted.contains(role));

    let (current_roles, current_nickname, renamable) = match ctx.author_member().await {
        Some(member) => (
            member.roles.clone(),
            member.nick.clone(),
            can_rename(ctx, &member),
        ),
        None => (
            Vec::new(),
            None,
            Err("the bot couldn't load your server profile"),
        ),
    };
    wanted.retain(|role| !current_roles.contains(role));
    unwanted.retain(|role| current_roles.contains(role));

    let reason = format!("Verified as {}", profile.name);
    for &role in &wanted {
        ctx.http()
            .add_member_role(guild_id, author.id, role, Some(&reason))
            .await?;
    }
    for &role in &unwanted {
        ctx.http()
            .remove_member_role(guild_id, author.id, role, Some(&reason))
            .await?;
    }

    let mut message = format!("You are verified as `{}`!", profile.name);
    if !wanted.is_empty() {
        message += &format!("\nRoles added: {}", mention_roles(&wanted));
    }
    if !unwanted.is_empty() {
        message += &format!("\nRoles removed: {}", mention_roles(&unwanted));
    }

    if nickname_format.enabled {
        let nickname = nickname_format.render(&nickname::Values {
            hypixel_rank: player.rank.as_deref().map(hypixel::rank_label),
            ign: &profile.name,
            guild_rank_tag: player_guild
                .as_ref()
                .zip(guild_rank)
                .and_then(|(guild, rank)| guild.rank_tag(rank)),
            guild_tag: player_guild.as_ref().and_then(|guild| guild.tag.as_deref()),
        });
        // An empty nickname would reset it to the Discord name instead
        if !nickname.is_empty() && current_nickname.as_deref() != Some(nickname.as_str()) {
            // A skipped or failed rename is only a warning, the roles are already updated
            let skipped = match renamable {
                Err(why) => Some(why),
                Ok(()) => {
                    let edit = serenity::EditMember::new()
                        .nickname(&nickname)
                        .audit_log_reason(&reason);
                    match guild_id.edit_member(ctx, author.id, edit).await {
                        Ok(_) => None,
                        Err(error) => {
                            eprintln!("Could not rename {} in {guild_id}: {error}", author.id);
                            Some("the bot may be missing the Manage Nicknames permission")
                        }
                    }
                }
            };
            message += &match skipped {
                None => format!("\nNickname set to `{nickname}`"),
                Some(why) => format!("\n⚠️ Nickname not changed to `{nickname}`: {why}."),
            };
        }
    }
    ctx.say(message).await?;
    Ok(())
}
