//! Posts what OBot did to members in the server's log channel, in the server's language

use poise::serenity_prelude as serenity;
use std::collections::BTreeSet;

use crate::config::GuildConfig;
use crate::i18n::{Lang, tr};
use crate::verification::{NicknameChange, Outcome};

/// Why a member's own verification was refused. Typos in the username aren't logged.
pub enum Refusal<'a> {
    NeverJoined,
    NoDiscordLinked,
    LinkedElsewhere { linked: &'a str },
    AlreadyVerified { current: &'a str },
    AccountTaken { owner: &'a str },
}

pub enum Event<'a> {
    /// An admin picked this channel in the panel
    ChannelSet { admin: serenity::UserId },
    /// By the member themselves, or by an admin from the panel
    Verified {
        member: serenity::UserId,
        name: &'a str,
        by: Option<serenity::UserId>,
        outcome: &'a Outcome,
    },
    Reverified {
        member: serenity::UserId,
        name: &'a str,
        by: serenity::UserId,
        outcome: &'a Outcome,
    },
    /// By the scheduled refresh, only logged when something changed
    Refreshed {
        member: serenity::UserId,
        name: &'a str,
        outcome: &'a Outcome,
    },
    Removed {
        member: serenity::UserId,
        by: serenity::UserId,
        outcome: &'a Outcome,
    },
    Refused {
        member: serenity::UserId,
        name: &'a str,
        reason: Refusal<'a>,
    },
}

fn mention(user: serenity::UserId) -> String {
    format!("<@{user}>")
}

fn mention_roles(roles: &BTreeSet<serenity::RoleId>) -> String {
    roles
        .iter()
        .map(|role| format!("<@&{role}>"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn details(lang: Lang, outcome: &Outcome) -> Vec<String> {
    let mut details = Vec::new();
    if !outcome.added.is_empty() {
        details.push(tr!(
            lang,
            "log-added",
            roles = mention_roles(&outcome.added)
        ));
    }
    if !outcome.removed.is_empty() {
        details.push(tr!(
            lang,
            "log-removed",
            roles = mention_roles(&outcome.removed)
        ));
    }
    match &outcome.nickname {
        NicknameChange::Unchanged => {}
        NicknameChange::Set(nickname) => {
            details.push(tr!(lang, "log-nickname-set", nickname = nickname.as_str()))
        }
        NicknameChange::Reset => details.push(lang.t("log-nickname-reset")),
        NicknameChange::Skipped { nickname, why } => details.push(tr!(
            lang,
            "log-nickname-skipped",
            nickname = nickname.as_str(),
            reason = lang.t(why)
        )),
    }
    details
}

impl Refusal<'_> {
    fn text(&self, lang: Lang) -> String {
        match self {
            Self::NeverJoined => lang.t("log-reason-never-joined"),
            Self::NoDiscordLinked => lang.t("log-reason-no-discord"),
            Self::LinkedElsewhere { linked } => {
                tr!(lang, "log-reason-linked-elsewhere", linked = *linked)
            }
            Self::AlreadyVerified { current } => {
                tr!(lang, "log-reason-already-verified", current = *current)
            }
            Self::AccountTaken { owner } => tr!(lang, "log-reason-account-taken", owner = *owner),
        }
    }
}

impl Event<'_> {
    /// The headline, then what changed in small text below it
    pub fn text(&self, lang: Lang) -> String {
        let (headline, outcome) = match self {
            Self::ChannelSet { admin } => {
                (tr!(lang, "log-channel-set", admin = mention(*admin)), None)
            }
            Self::Verified {
                member,
                name,
                by: None,
                outcome,
            } => (
                tr!(
                    lang,
                    "log-verified",
                    member = mention(*member),
                    name = *name
                ),
                Some(outcome),
            ),
            Self::Verified {
                member,
                name,
                by: Some(admin),
                outcome,
            } => (
                tr!(
                    lang,
                    "log-verified-by",
                    member = mention(*member),
                    name = *name,
                    admin = mention(*admin)
                ),
                Some(outcome),
            ),
            Self::Reverified {
                member,
                name,
                by,
                outcome,
            } => (
                tr!(
                    lang,
                    "log-reverified-by",
                    member = mention(*member),
                    name = *name,
                    admin = mention(*by)
                ),
                Some(outcome),
            ),
            Self::Refreshed {
                member,
                name,
                outcome,
            } => (
                tr!(
                    lang,
                    "log-refreshed",
                    member = mention(*member),
                    name = *name
                ),
                Some(outcome),
            ),
            Self::Removed {
                member,
                by,
                outcome,
            } => (
                tr!(
                    lang,
                    "log-removed-by",
                    member = mention(*member),
                    admin = mention(*by)
                ),
                Some(outcome),
            ),
            Self::Refused {
                member,
                name,
                reason,
            } => (
                tr!(
                    lang,
                    "log-refused",
                    member = mention(*member),
                    name = *name,
                    reason = reason.text(lang)
                ),
                None,
            ),
        };
        let details = outcome.map_or_else(Vec::new, |outcome| details(lang, outcome));
        if details.is_empty() {
            headline
        } else {
            // `-#` is Discord's small print
            format!("{headline}\n-# {}", details.join(" · "))
        }
    }
}

/// Posts the event when the server has a log channel. Mentions don't ping anyone, and a failure
/// is only printed: the log never stops what it reports on.
pub async fn post(discord: &serenity::Http, config: &GuildConfig, event: Event<'_>) {
    let Some(channel_id) = config.log_channel_id else {
        return;
    };
    let message = serenity::CreateMessage::new()
        .content(event.text(config.language))
        .allowed_mentions(serenity::CreateAllowedMentions::new());
    if let Err(error) = channel_id.send_message(discord, message).await {
        eprintln!("Could not post in the log channel {channel_id}: {error}");
    }
}

#[cfg(test)]
mod tests;
