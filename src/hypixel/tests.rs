use super::*;

impl Guild {
    /// A guild whose members are `(uuid, joined at in seconds, XP this week)`
    pub(crate) fn for_tests(members: &[(&str, i64, u64)]) -> Self {
        Self {
            id: "guild".to_owned(),
            name: "Guild".to_owned(),
            tag: None,
            members: members
                .iter()
                .map(|&(uuid, joined, xp)| GuildMember {
                    uuid: uuid.to_owned(),
                    rank: "Member".to_owned(),
                    joined: Some(joined * 1000),
                    exp_history: [("2026-09-20".to_owned(), xp)].into_iter().collect(),
                })
                .collect(),
            ranks: Vec::new(),
        }
    }
}

fn rank(name: &str, tag: Option<&str>, priority: i64) -> GuildRank {
    GuildRank {
        name: name.to_owned(),
        tag: tag.map(str::to_owned),
        priority,
    }
}

#[test]
fn guild_ranks_are_listed_highest_first() {
    let guild = Guild {
        id: "id".to_owned(),
        name: "Guild".to_owned(),
        tag: None,
        members: vec![GuildMember {
            uuid: "owner".to_owned(),
            rank: "GUILDMASTER".to_owned(),
            joined: Some(1_600_000_000_123),
            exp_history: [
                ("2026-09-20".to_owned(), 1200),
                ("2026-09-21".to_owned(), 34),
            ]
            .into_iter()
            .collect(),
        }],
        ranks: vec![rank("Member", None, 1), rank("Officer", Some("OFC"), 3)],
    };

    assert_eq!(guild.rank_names(), ["Guild Master", "Officer", "Member"]);
    assert_eq!(guild.member_rank("owner"), Some(GUILD_MASTER));
    let stats = guild.member_stats("owner").expect("owner is a member");
    assert_eq!(stats.joined_at, Some(1_600_000_000));
    assert_eq!(stats.weekly_xp, 1234);
    assert!(guild.member_stats("stranger").is_none());
    assert_eq!(guild.rank_tag("Guild Master"), Some("GM"));
    assert_eq!(guild.rank_tag("officer"), Some("OFC"));
    assert_eq!(guild.rank_tag("Member"), None);
}
