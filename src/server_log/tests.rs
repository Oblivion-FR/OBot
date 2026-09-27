use super::*;

fn user(id: u64) -> serenity::UserId {
    serenity::UserId::new(id)
}

fn outcome() -> Outcome {
    Outcome {
        added: BTreeSet::from([serenity::RoleId::new(5), serenity::RoleId::new(6)]),
        removed: BTreeSet::from([serenity::RoleId::new(7)]),
        nickname: NicknameChange::Set("[MVP+] Notch".to_owned()),
    }
}

#[test]
fn verification_lists_what_changed_in_small_print() {
    let outcome = outcome();
    let text = Event::Verified {
        member: user(1),
        name: "Notch",
        by: None,
        outcome: &outcome,
    }
    .text(Lang::En);
    assert_eq!(
        text,
        "✅ <@1> verified as `Notch`.\n\
         -# Added <@&5>, <@&6> · Removed <@&7> · nickname `[MVP+] Notch`"
    );
}

#[test]
fn nothing_changed_leaves_only_the_headline() {
    let outcome = Outcome {
        added: BTreeSet::new(),
        removed: BTreeSet::new(),
        nickname: NicknameChange::Unchanged,
    };
    let text = Event::Reverified {
        member: user(1),
        name: "Notch",
        by: user(2),
        outcome: &outcome,
    }
    .text(Lang::En);
    assert!(text.contains("<@2>"));
    assert!(!text.contains('\n'));
}

#[test]
fn refusals_and_skipped_nicknames_are_translated() {
    let text = Event::Refused {
        member: user(1),
        name: "Notch",
        reason: Refusal::LinkedElsewhere { linked: "someone" },
    }
    .text(Lang::Fr);
    assert!(text.starts_with("❌ <@1> n'a pas pu se vérifier"));
    assert!(text.contains("`someone`"));

    let outcome = Outcome {
        added: BTreeSet::new(),
        removed: BTreeSet::new(),
        nickname: NicknameChange::Skipped {
            nickname: "Notch".to_owned(),
            why: "rename-owner",
        },
    };
    let text = Event::Refreshed {
        member: user(1),
        name: "Notch",
        outcome: &outcome,
    }
    .text(Lang::En);
    assert!(text.ends_with("bots can't rename the server owner"));
}
