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

fn role(id: u64) -> serenity::RoleId {
    serenity::RoleId::new(id)
}

fn rule(id: i64, kind: RuleKind, value: &str, role_id: u64, group_id: Option<i64>) -> Rule {
    Rule {
        id,
        kind,
        value: value.to_owned(),
        role_id: role(role_id),
        group_id,
    }
}

fn roles(ids: &[u64]) -> BTreeSet<serenity::RoleId> {
    ids.iter().copied().map(role).collect()
}

#[test]
fn no_rank_rule_matches_players_without_a_rank_only() {
    let no_rank = rule(1, RuleKind::HypixelRank, hypixel::NO_RANK, 10, None);
    assert!(rule_matches(&no_rank, None, None));
    assert!(!rule_matches(&no_rank, Some("VIP"), None));

    let vip = rule(2, RuleKind::HypixelRank, "VIP", 11, None);
    assert!(rule_matches(&vip, Some("VIP"), None));
    assert!(!rule_matches(&vip, None, None));
}

#[test]
fn guild_rules_need_the_linked_guild() {
    let member = rule(1, RuleKind::GuildMember, "", 10, None);
    let officer = rule(2, RuleKind::GuildRank, "Officer", 11, None);
    assert!(rule_matches(&member, None, Some("Member")));
    assert!(!rule_matches(&member, None, None));
    assert!(rule_matches(&officer, None, Some("officer")));
    assert!(!rule_matches(&officer, None, Some("Member")));
}

#[test]
fn separators_follow_their_group_rules() {
    // Group 1 (separator 100): VIP → 10, MVP → 11. Group 2 (separator 200): guild member → 20.
    let rules = [
        rule(1, RuleKind::HypixelRank, "VIP", 10, Some(1)),
        rule(2, RuleKind::HypixelRank, "MVP", 11, Some(1)),
        rule(3, RuleKind::GuildMember, "", 20, Some(2)),
        rule(4, RuleKind::HypixelRank, hypixel::NO_RANK, 30, None),
    ];
    let groups = [
        RuleGroup {
            id: 1,
            name: "Ranks".to_owned(),
            separator_role_id: role(100),
        },
        RuleGroup {
            id: 2,
            name: "Guild".to_owned(),
            separator_role_id: role(200),
        },
    ];
    let verified = role(1);
    let unverified = Some(role(2));

    let vip_outside_guild = plan_roles(verified, unverified, &rules, &groups, Some("VIP"), None);
    assert_eq!(vip_outside_guild.wanted, roles(&[1, 10, 100]));
    assert_eq!(vip_outside_guild.unwanted, roles(&[2, 11, 20, 30, 200]));

    let no_rank_in_guild = plan_roles(verified, unverified, &rules, &groups, None, Some("Member"));
    assert_eq!(no_rank_in_guild.wanted, roles(&[1, 20, 30, 200]));
    assert_eq!(no_rank_in_guild.unwanted, roles(&[2, 10, 11, 100]));
}

#[test]
fn a_role_wanted_for_one_reason_is_never_removed_for_another() {
    // The same role is both a rule role and a separator, and matches through the rule only
    let rules = [
        rule(1, RuleKind::HypixelRank, "VIP", 50, None),
        rule(2, RuleKind::HypixelRank, "MVP", 11, Some(1)),
    ];
    let groups = [RuleGroup {
        id: 1,
        name: "Ranks".to_owned(),
        separator_role_id: role(50),
    }];
    let plan = plan_roles(role(1), None, &rules, &groups, Some("VIP"), None);
    assert!(plan.wanted.contains(&role(50)));
    assert!(!plan.unwanted.contains(&role(50)));
}
