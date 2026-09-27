use super::*;

fn stored(minecraft_uuid: &str, minecraft_name: &str) -> VerifiedMember {
    VerifiedMember {
        minecraft_uuid: minecraft_uuid.to_owned(),
        minecraft_name: minecraft_name.to_owned(),
        verified_at: 1_750_000_000,
        forced_by: None,
    }
}

fn info(id: u64, name: &str, joined_at: Option<i64>) -> MemberInfo {
    MemberInfo {
        id: serenity::UserId::new(id),
        name: name.to_owned(),
        username: name.to_lowercase(),
        avatar_url: format!("https://cdn.example/{id}.png"),
        joined_at,
        roles: Vec::new(),
    }
}

/// A table with one verified and one unverified member, for template tests
pub(in crate::web) fn sample_table() -> MembersTable {
    let guild = hypixel::Guild::for_tests(&[("uuid", 1_720_000_000, 1_234_567)]);
    let mut veteran = stored("uuid", "Notch");
    veteran.forced_by = Some(serenity::UserId::new(1));
    let mut unverified = member_row(&info(21, "Newcomer", Some(1_700_000_000)), None, None, None);
    unverified.notice = Some(Notice {
        kind: "warn",
        text: "Nickname not changed.".to_owned(),
    });
    let hidden = hidden_columns(None);
    MembersTable {
        rows: vec![
            member_row(
                &info(20, "Veteran", Some(1_700_000_000)),
                Some(&veteran),
                None,
                Some(&guild),
            ),
            unverified,
        ],
        error: None,
        guild_note: None,
        total: 2,
        verified_count: 1,
        matching: 2,
        headers: headers(
            serenity::GuildId::new(2),
            Some(("name", Direction::Ascending)),
            "",
            &hidden,
        ),
        hidden_classes: "hide-xp".to_owned(),
        sort: Some(("name", "asc")),
        search: String::new(),
        page: 1,
        pages: 2,
        prev_url: None,
        next_url: Some(page_url(
            serenity::GuildId::new(2),
            Some(("name", "asc")),
            "",
            2,
        )),
        can_act: true,
    }
}

#[test]
fn page_urls_keep_sort_and_encode_search() {
    let guild_id = serenity::GuildId::new(5);
    assert_eq!(
        page_url(guild_id, Some(("name", "desc")), "a b&c", 2),
        "/guilds/5/verification?sort=name&dir=desc&q=a%20b%26c&page=2#members"
    );
    assert_eq!(
        page_url(guild_id, None, "", 1),
        "/guilds/5/verification#members"
    );
}

#[test]
fn headers_cycle_ascending_descending_unsorted() {
    let guild_id = serenity::GuildId::new(5);
    let arrows = |sort| {
        headers(guild_id, sort, "", &[])
            .into_iter()
            .map(|header| (header.arrow, header.url))
            .collect::<Vec<_>>()
    };

    let unsorted = arrows(None);
    assert_eq!(unsorted[0].0, "↕");
    assert!(unsorted[0].1.contains("sort=name&dir=asc"));

    let ascending = arrows(Some(("name", Direction::Ascending)));
    assert_eq!(ascending[0].0, "↓");
    assert!(ascending[0].1.contains("sort=name&dir=desc"));
    assert_eq!(ascending[1].0, "↕", "other columns stay unsorted");

    let descending = arrows(Some(("name", Direction::Descending)));
    assert_eq!(descending[0].0, "↑");
    assert_eq!(descending[0].1, "/guilds/5/verification#members");
}

#[test]
fn hidden_columns_default_to_xp_and_ignore_unknown_keys() {
    assert_eq!(hidden_columns(None), ["xp"]);
    assert!(
        hidden_columns(Some("")).is_empty(),
        "the viewer showed everything"
    );
    assert_eq!(
        hidden_columns(Some("verified,name,bogus,minecraft")),
        ["minecraft", "verified"],
        "the member column can't be hidden"
    );
}

#[test]
fn missing_values_stay_last_in_both_directions() {
    let guild = hypixel::Guild::for_tests(&[("zed", 300, 50), ("alex", 100, 900)]);
    let row = |name: &str, account: Option<(&str, &str)>, joined| {
        let stored = account.map(|(uuid, minecraft)| stored(uuid, minecraft));
        member_row(&info(1, name, joined), stored.as_ref(), None, Some(&guild))
    };
    let mut rows = vec![
        row("b", None, None),
        row("a", Some(("zed", "Zed")), Some(2)),
        row("c", Some(("alex", "Alex")), Some(1)),
        row("d", Some(("other", "Outsider")), Some(3)),
    ];
    let names = |rows: &[MemberRow]| rows.iter().map(|row| row.name.as_str()).collect::<String>();

    sort_rows(&mut rows, "minecraft", Direction::Ascending);
    assert_eq!(names(&rows), "cdab");
    sort_rows(&mut rows, "minecraft", Direction::Descending);
    assert_eq!(names(&rows), "adcb");
    sort_rows(&mut rows, "joined", Direction::Descending);
    assert_eq!(names(&rows), "dacb");
    sort_rows(&mut rows, "guild_joined", Direction::Ascending);
    assert_eq!(names(&rows), "cabd", "members outside the guild go last");
    sort_rows(&mut rows, "xp", Direction::Descending);
    assert_eq!(names(&rows), "cabd");
    sort_rows(&mut rows, "status", Direction::Ascending);
    assert_eq!(names(&rows), "acdb", "verified first, then by name");
}

#[test]
fn row_shows_verification_and_guild_values() {
    let mut member = info(1, "Name", Some(1_700_000_000));
    member.roles = vec![serenity::RoleId::new(9)];
    let role = Some(serenity::RoleId::new(9));
    assert_eq!(member_row(&member, None, role, None).status, "role");
    assert_eq!(member_row(&member, None, None, None).status, "none");

    let guild = hypixel::Guild::for_tests(&[("uuid", 1_720_000_000, 1_234_567)]);
    let row = member_row(&member, Some(&stored("uuid", "Notch")), role, Some(&guild));
    assert_eq!(row.status, "verified");
    assert_eq!(row.joined, "2023-11-14");
    assert_eq!(row.verified, "2025-06-15");
    assert_eq!(row.guild_joined, "2024-07-03");
    assert_eq!(row.xp, "1,234,567");

    let outside = member_row(&member, Some(&stored("other", "Notch")), role, Some(&guild));
    assert_eq!(
        (outside.guild_joined.as_str(), outside.xp.as_str()),
        ("", "")
    );
}

#[test]
fn xp_uses_thousands_separators() {
    assert_eq!(xp_label(0), "0");
    assert_eq!(xp_label(999), "999");
    assert_eq!(xp_label(1000), "1,000");
    assert_eq!(xp_label(12_345_678), "12,345,678");
}
