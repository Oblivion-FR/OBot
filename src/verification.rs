use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use std::collections::BTreeSet;

use crate::Error;
use crate::config::{self, RuleKind};
use crate::hypixel::{self, GuildQuery, MojangProfile, Player};
use crate::nickname;

/// What syncing a member needs, shared by `/verify` and the panel
pub struct Services<'a> {
    pub db: &'a SqlitePool,
    pub http: &'a reqwest::Client,
    pub hypixel_api_key: &'a str,
    pub discord: &'a serenity::Http,
    pub cache: &'a serenity::Cache,
}

/// Whether the Discord account linked on Hypixel is this user, which proves they own the
/// Minecraft account. Hypixel stores whatever the player typed, which may be a legacy `name#1234` tag.
pub fn is_same_user(user: &serenity::User, linked: &str) -> bool {
    let linked = linked.trim();
    linked.eq_ignore_ascii_case(&user.name) || linked.eq_ignore_ascii_case(&user.tag())
}

pub enum NicknameChange {
    /// Renaming is off, or the nickname is already right
    Unchanged,
    Set(String),
    /// Back to the member's Discord name
    Reset,
    Skipped {
        nickname: String,
        why: &'static str,
    },
}

pub struct Outcome {
    pub added: BTreeSet<serenity::RoleId>,
    pub removed: BTreeSet<serenity::RoleId>,
    pub nickname: NicknameChange,
}

/// Discord never lets bots rename the server owner, nor members whose highest role is at or
/// above the bot's. The error is shown to whoever triggered the verification.
fn can_rename(
    cache: &serenity::Cache,
    guild_id: serenity::GuildId,
    member: &serenity::Member,
) -> Result<(), &'static str> {
    let Some(guild) = cache.guild(guild_id) else {
        return Err("the bot couldn't load this server");
    };
    if guild.owner_id == member.user.id {
        return Err("bots can't rename the server owner");
    }
    let bot_id = cache.current_user().id;
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
        Err("their highest role is at or above the bot's")
    }
}

/// Applies a proven Minecraft account to a member: roles from the rules, nickname, and the
/// stored link used to re-verify them later
#[allow(clippy::too_many_arguments)]
pub async fn sync_member(
    services: &Services<'_>,
    guild_id: serenity::GuildId,
    member: &serenity::Member,
    verified_role_id: serenity::RoleId,
    unverified_role_id: Option<serenity::RoleId>,
    hypixel_guild_id: Option<&str>,
    profile: &MojangProfile,
    player: &Player,
    forced_by: Option<serenity::UserId>,
) -> Result<Outcome, Error> {
    let rules = config::list_rules(services.db, guild_id).await?;
    let nickname_format = config::get_nickname_format(services.db, guild_id).await?;
    let needs_guild = rules.iter().any(|rule| rule.kind != RuleKind::HypixelRank)
        || (nickname_format.enabled && nickname_format.needs_guild());
    // Only the linked Hypixel guild counts, being in another one is the same as being in none
    let player_guild = match hypixel_guild_id {
        Some(linked_id) if needs_guild => {
            let query = GuildQuery::Player(&profile.id);
            hypixel::fetch_guild(services.http, services.hypixel_api_key, query)
                .await?
                .filter(|guild| guild.id == linked_id)
        }
        _ => None,
    };
    let guild_rank = player_guild
        .as_ref()
        .and_then(|guild| guild.member_rank(&profile.id));

    // Roles referenced by rules are kept in sync: granted when the rule matches, removed otherwise
    let mut added = BTreeSet::from([verified_role_id]);
    let mut removed = BTreeSet::new();
    for rule in &rules {
        let matches = match rule.kind {
            RuleKind::HypixelRank => player.rank.as_deref() == Some(rule.value.as_str()),
            RuleKind::GuildMember => guild_rank.is_some(),
            RuleKind::GuildRank => {
                guild_rank.is_some_and(|rank| rank.eq_ignore_ascii_case(&rule.value))
            }
        };
        if matches {
            added.insert(rule.role_id);
        } else {
            removed.insert(rule.role_id);
        }
    }
    removed.extend(unverified_role_id);
    removed.retain(|role| !added.contains(role));
    added.retain(|role| !member.roles.contains(role));
    removed.retain(|role| member.roles.contains(role));

    let reason = match forced_by {
        Some(_) => format!("Verified as {} by an admin", profile.name),
        None => format!("Verified as {}", profile.name),
    };
    let user_id = member.user.id;
    for &role in &added {
        services
            .discord
            .add_member_role(guild_id, user_id, role, Some(&reason))
            .await?;
    }
    for &role in &removed {
        services
            .discord
            .remove_member_role(guild_id, user_id, role, Some(&reason))
            .await?;
    }
    config::record_verification(
        services.db,
        guild_id,
        user_id,
        &profile.id,
        &profile.name,
        forced_by,
    )
    .await?;

    let mut nickname_change = NicknameChange::Unchanged;
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
        if !nickname.is_empty() && member.nick.as_deref() != Some(nickname.as_str()) {
            // A skipped or failed rename is only a warning, the roles are already updated
            nickname_change = match can_rename(services.cache, guild_id, member) {
                Err(why) => NicknameChange::Skipped { nickname, why },
                Ok(()) => {
                    let edit = serenity::EditMember::new()
                        .nickname(&nickname)
                        .audit_log_reason(&reason);
                    match guild_id.edit_member(services.discord, user_id, edit).await {
                        Ok(_) => NicknameChange::Set(nickname),
                        Err(error) => {
                            eprintln!("Could not rename {user_id} in {guild_id}: {error}");
                            NicknameChange::Skipped {
                                nickname,
                                why: "the bot may be missing the Manage Nicknames permission",
                            }
                        }
                    }
                }
            };
        }
    }

    Ok(Outcome {
        added,
        removed,
        nickname: nickname_change,
    })
}

/// Why a Minecraft account can't be linked to a member
pub enum Conflict {
    /// The member is already linked to another account
    MemberLinked { minecraft_name: String },
    /// The account is linked to another member of the server
    AccountLinked { name: String },
}

/// One Discord account per Minecraft account in each server. Re-linking the same account is
/// fine, and the link of a member who left the server is dropped instead of blocking.
pub async fn check_unique(
    services: &Services<'_>,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    minecraft_uuid: &str,
) -> Result<Option<Conflict>, Error> {
    if let Some(existing) = config::get_verified_member(services.db, guild_id, user_id).await?
        && existing.minecraft_uuid != minecraft_uuid
    {
        return Ok(Some(Conflict::MemberLinked {
            minecraft_name: existing.minecraft_name,
        }));
    }
    let Some(owner) = config::find_account_owner(services.db, guild_id, minecraft_uuid).await?
    else {
        return Ok(None);
    };
    if owner == user_id {
        return Ok(None);
    }
    match services.discord.get_member(guild_id, owner).await {
        Ok(member) => Ok(Some(Conflict::AccountLinked {
            name: member.display_name().to_owned(),
        })),
        Err(serenity::Error::Http(error))
            if error
                .status_code()
                .is_some_and(|status| status.as_u16() == 404) =>
        {
            config::delete_verification(services.db, guild_id, owner).await?;
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

/// Undoes a verification: removes the verified role and every rule role, gives the unverified
/// role back, resets the nickname and forgets the linked account
pub async fn unverify_member(
    services: &Services<'_>,
    guild_id: serenity::GuildId,
    member: &serenity::Member,
    verified_role_id: Option<serenity::RoleId>,
    unverified_role_id: Option<serenity::RoleId>,
) -> Result<Outcome, Error> {
    let rules = config::list_rules(services.db, guild_id).await?;
    let mut removed: BTreeSet<_> = verified_role_id
        .into_iter()
        .chain(rules.iter().map(|rule| rule.role_id))
        .filter(|role| member.roles.contains(role))
        .collect();
    let added: BTreeSet<_> = unverified_role_id
        .filter(|role| !member.roles.contains(role))
        .into_iter()
        .collect();
    removed.retain(|role| !added.contains(role));

    let reason = "Verification removed by an admin";
    let user_id = member.user.id;
    for &role in &removed {
        services
            .discord
            .remove_member_role(guild_id, user_id, role, Some(reason))
            .await?;
    }
    for &role in &added {
        services
            .discord
            .add_member_role(guild_id, user_id, role, Some(reason))
            .await?;
    }
    config::delete_verification(services.db, guild_id, user_id).await?;

    let nickname = match &member.nick {
        None => NicknameChange::Unchanged,
        Some(nickname) => match can_rename(services.cache, guild_id, member) {
            Err(why) => NicknameChange::Skipped {
                nickname: nickname.clone(),
                why,
            },
            Ok(()) => {
                // An empty nickname makes Discord show the member's own name again
                let edit = serenity::EditMember::new()
                    .nickname("")
                    .audit_log_reason(reason);
                match guild_id.edit_member(services.discord, user_id, edit).await {
                    Ok(_) => NicknameChange::Reset,
                    Err(error) => {
                        eprintln!(
                            "Could not reset the nickname of {user_id} in {guild_id}: {error}"
                        );
                        NicknameChange::Skipped {
                            nickname: nickname.clone(),
                            why: "the bot may be missing the Manage Nicknames permission",
                        }
                    }
                }
            }
        },
    };

    Ok(Outcome {
        added,
        removed,
        nickname,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linked_discord_matches_username_or_legacy_tag() {
        let mut user = serenity::User::default();
        user.name = "notch".to_owned();
        assert!(is_same_user(&user, " Notch "));
        assert!(!is_same_user(&user, "jeb_"));

        user.discriminator = std::num::NonZeroU16::new(1234);
        assert!(is_same_user(&user, "notch#1234"));
    }
}
