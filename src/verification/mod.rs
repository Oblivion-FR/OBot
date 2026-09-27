use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use std::collections::BTreeSet;

use crate::Error;
use crate::config::{self, GuildConfig, Rule, RuleGroup, RuleKind, VerifiedMember};
use crate::hypixel::{self, Hypixel, MojangProfile, Player};
use crate::nickname;

/// What syncing a member needs, shared by `/verify` and the panel
pub struct Services<'a> {
    pub db: &'a SqlitePool,
    pub hypixel: &'a Hypixel,
    /// For Mojang, which has no API key and no rate limit headers to follow
    pub mojang: &'a reqwest::Client,
    pub discord: &'a serenity::Http,
    pub cache: &'a serenity::Cache,
}

/// Whether the Discord account linked on Hypixel is this user, which proves they own the
/// Minecraft account. Hypixel stores whatever the player typed, which may be a legacy `name#1234` tag.
pub fn is_same_user(user: &serenity::User, linked: &str) -> bool {
    let linked = linked.trim();
    linked.eq_ignore_ascii_case(&user.name) || linked.eq_ignore_ascii_case(&user.tag())
}

/// The player's profile, as long as it shows the ownership proof. A cached profile is only used
/// when it proves the link, otherwise it's fetched again: after a failed attempt the player may
/// have just linked their Discord in game. `None` if they never joined Hypixel.
pub async fn player_for_proof(
    hypixel: &Hypixel,
    uuid: &str,
    user: &serenity::User,
) -> Result<Option<Player>, Error> {
    if let Some(Some(player)) = hypixel.cached_player(uuid)
        && player
            .discord
            .as_deref()
            .is_some_and(|linked| is_same_user(user, linked))
    {
        return Ok(Some(player));
    }
    hypixel.player(uuid).await
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

impl Outcome {
    /// Whether the member's roles or nickname were changed
    pub fn changed(&self) -> bool {
        !self.added.is_empty()
            || !self.removed.is_empty()
            || matches!(
                self.nickname,
                NicknameChange::Set(_) | NicknameChange::Reset
            )
    }
}

/// How a sync is recorded
pub enum Record {
    /// A new proof of ownership, by the member or by an admin on their behalf
    Verified { by: Option<serenity::UserId> },
    /// A refresh of an earlier proof: keeps when and by whom it was made
    Refreshed,
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

pub enum RefreshFailure {
    /// The server has no verified role anymore
    NotSetUp,
    /// The Minecraft account was deleted
    AccountGone,
}

/// Re-applies the account a member proved before, with current Mojang and Hypixel data: picks
/// up name, rank and guild changes. The proof isn't checked again, it was when it was made.
pub async fn refresh_member(
    services: &Services<'_>,
    guild_id: serenity::GuildId,
    member: &serenity::Member,
    config: &GuildConfig,
    stored: &VerifiedMember,
) -> Result<Result<(MojangProfile, Outcome), RefreshFailure>, Error> {
    let Some(verified_role_id) = config.verified_role_id else {
        return Ok(Err(RefreshFailure::NotSetUp));
    };
    let Some(profile) =
        hypixel::fetch_mojang_profile_by_uuid(services.mojang, &stored.minecraft_uuid).await?
    else {
        return Ok(Err(RefreshFailure::AccountGone));
    };
    let player = services
        .hypixel
        .player(&profile.id)
        .await?
        .unwrap_or(Player {
            discord: None,
            rank: None,
        });
    let outcome = sync_member(
        services,
        guild_id,
        member,
        verified_role_id,
        config.unverified_role_id,
        config.hypixel_guild.as_ref().map(|link| link.id.as_str()),
        &profile,
        &player,
        Record::Refreshed,
    )
    .await?;
    Ok(Ok((profile, outcome)))
}

/// Whether a rule applies to a player, from their Hypixel rank (`None` without one) and their
/// rank in the linked guild (`None` when not in it)
pub fn rule_matches(rule: &Rule, rank: Option<&str>, guild_rank: Option<&str>) -> bool {
    match rule.kind {
        RuleKind::HypixelRank => rank.unwrap_or(hypixel::NO_RANK) == rule.value,
        RuleKind::GuildMember => guild_rank.is_some(),
        RuleKind::GuildRank => {
            guild_rank.is_some_and(|rank| rank.eq_ignore_ascii_case(&rule.value))
        }
    }
}

/// Roles a member should and shouldn't have, before looking at what they have now
#[derive(Debug, PartialEq)]
pub struct RolePlan {
    pub wanted: BTreeSet<serenity::RoleId>,
    pub unwanted: BTreeSet<serenity::RoleId>,
}

/// Every role the verification manages is kept in sync: a rule's role while the rule matches,
/// a group's separator while any of its rules matches, and the verified and unverified roles.
/// A role wanted for one reason is never removed for another.
pub fn plan_roles(
    verified_role_id: serenity::RoleId,
    unverified_role_id: Option<serenity::RoleId>,
    rules: &[Rule],
    groups: &[RuleGroup],
    rank: Option<&str>,
    guild_rank: Option<&str>,
) -> RolePlan {
    let mut wanted = BTreeSet::from([verified_role_id]);
    let mut unwanted = BTreeSet::new();
    let mut matched_groups = BTreeSet::new();
    for rule in rules {
        if rule_matches(rule, rank, guild_rank) {
            wanted.insert(rule.role_id);
            matched_groups.extend(rule.group_id);
        } else {
            unwanted.insert(rule.role_id);
        }
    }
    for group in groups {
        if matched_groups.contains(&group.id) {
            wanted.insert(group.separator_role_id);
        } else {
            unwanted.insert(group.separator_role_id);
        }
    }
    unwanted.extend(unverified_role_id);
    unwanted.retain(|role| !wanted.contains(role));
    RolePlan { wanted, unwanted }
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
    record: Record,
) -> Result<Outcome, Error> {
    let rules = config::list_rules(services.db, guild_id).await?;
    let groups = config::list_groups(services.db, guild_id).await?;
    let nickname_format = config::get_nickname_format(services.db, guild_id).await?;
    let needs_guild = rules.iter().any(|rule| rule.kind != RuleKind::HypixelRank)
        || (nickname_format.enabled && nickname_format.needs_guild());
    // Only the linked Hypixel guild counts, being in another one is the same as being in none
    let player_guild = match hypixel_guild_id {
        Some(linked_id) if needs_guild => {
            services
                .hypixel
                .guild_with_member(linked_id, &profile.id)
                .await?
        }
        _ => None,
    };
    let guild_rank = player_guild
        .as_ref()
        .and_then(|guild| guild.member_rank(&profile.id));

    let RolePlan {
        wanted: mut added,
        unwanted: mut removed,
    } = plan_roles(
        verified_role_id,
        unverified_role_id,
        &rules,
        &groups,
        player.rank.as_deref(),
        guild_rank,
    );
    added.retain(|role| !member.roles.contains(role));
    removed.retain(|role| member.roles.contains(role));

    let reason = match record {
        Record::Verified { by: Some(_) } => format!("Verified as {} by an admin", profile.name),
        Record::Verified { by: None } => format!("Verified as {}", profile.name),
        Record::Refreshed => format!("Refreshed from {}'s Hypixel profile", profile.name),
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
    match record {
        Record::Verified { by } => {
            config::record_verification(
                services.db,
                guild_id,
                user_id,
                &profile.id,
                &profile.name,
                by,
            )
            .await?
        }
        Record::Refreshed => {
            config::update_minecraft_name(services.db, guild_id, user_id, &profile.name).await?
        }
    }

    let mut nickname_change = NicknameChange::Unchanged;
    if nickname_format.enabled {
        let nickname = nickname_format.render(&nickname::Values {
            hypixel_rank: Some(nickname::Keyed {
                key: player.rank.as_deref().unwrap_or(hypixel::NO_RANK),
                label: player.rank.as_deref().map(hypixel::rank_label),
            }),
            ign: &profile.name,
            guild_rank: player_guild.as_ref().zip(guild_rank).map(|(guild, rank)| {
                nickname::Keyed {
                    key: rank,
                    label: guild.rank_tag(rank),
                }
            }),
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
    let groups = config::list_groups(services.db, guild_id).await?;
    let mut removed: BTreeSet<_> = verified_role_id
        .into_iter()
        .chain(rules.iter().map(|rule| rule.role_id))
        .chain(groups.iter().map(|group| group.separator_role_id))
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
mod tests;
