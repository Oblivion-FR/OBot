use super::*;

fn shell() -> Shell {
    Shell {
        user: User {
            id: serenity::UserId::new(1),
            name: "admin".to_owned(),
            avatar_url: "https://cdn.example/avatar.png".to_owned(),
            lang: None,
        },
        guilds: vec![
            guild(2, "Test <server>", None),
            guild(3, "With Icon", Some("https://cdn.example/icon.png")),
        ],
        current: Some(serenity::GuildId::new(2)),
        invite_url: "https://discord.com/oauth2/authorize?client_id=9".to_owned(),
        privacy_admin: false,
    }
}

fn guild(id: u64, name: &str, icon_url: Option<&str>) -> GuildSummary {
    GuildSummary {
        id: serenity::GuildId::new(id),
        name: name.to_owned(),
        icon_url: icon_url.map(str::to_owned),
        initials: super::super::initials(name),
    }
}

fn role(id: u64, name: &str, assignable: bool) -> RoleOption {
    RoleOption {
        id: serenity::RoleId::new(id),
        name: name.to_owned(),
        assignable,
    }
}

fn chip(name: &str) -> RoleChip {
    RoleChip {
        name: name.to_owned(),
        color: "#3ba55d".to_owned(),
    }
}

#[test]
fn frame_renders_rail_and_sidebar() {
    let html = OverviewPage {
        lang: Lang::En,
        shell: shell(),
        guild: guild(2, "Test <server>", None),
        section: "overview",
        section_title: "nav-overview",
        error: None,
        verified_role: Some(chip("Verified")),
        hypixel_guild: None,
        rule_count: 1,
        nickname_enabled: true,
        nickname_example: "[MVP+] Notch [OFC]".to_owned(),
    }
    .render()
    .expect("template renders");

    assert!(
        html.contains(r#"data-tooltip="Test &#60;server&#62;""#),
        "names are escaped"
    );
    assert!(html.contains(r#"class="rail-item active" data-tooltip="Test &#60;server&#62;""#));
    assert!(html.contains(r#"<img src="https://cdn.example/icon.png" alt="">"#));
    assert!(
        html.contains("\n      Ts\n    </a>"),
        "initials without an icon"
    );
    assert!(html.contains(r#"class="active" aria-current="page""#));
    assert!(html.contains("https://cdn.example/avatar.png"));
    assert!(html.contains("1 rule<"));
    assert!(html.contains("[MVP+] Notch [OFC]"));

    // Footer: the version, its commit and the repository
    assert!(html.contains(&format!("OBot v{}", crate::version::NUMBER)));
    assert!(html.contains(r#"<a href="https://github.com/Oblivion-FR/OBot" target="_blank""#));
    assert!(
        html.contains(r#"href="https://github.com/Oblivion-FR/OBot/blob/main/docs/privacy.md""#)
    );
    if let Some(url) = crate::version::commit_url() {
        assert!(html.contains(&format!(r#"<a href="{url}""#)));
    }
}

#[test]
fn verification_page_renders_roles_and_error() {
    let html = VerificationPage {
        lang: Lang::En,
        shell: shell(),
        guild: guild(2, "Test", None),
        section: "verification",
        section_title: "nav-verification",
        error: error_message("role_not_allowed"),
        roles: vec![role(10, "Admin", false), role(11, "Verified", true)],
        verified_role_id: Some(serenity::RoleId::new(11)),
        unverified_role_id: None,
        hypixel_guild: Some("My Guild".to_owned()),
        guild_id: serenity::GuildId::new(2),
        members: members::tests::sample_table(),
    }
    .render()
    .expect("template renders");

    assert!(html.contains(r#"<option value="11" selected>@Verified</option>"#));
    assert!(html.contains(r#"<option value="10" disabled>@Admin</option>"#));
    assert!(html.contains(r#"value="My Guild""#));
    assert!(html.contains("You can only pick roles below your highest role."));

    assert!(html.contains("1 of 2 verified"));
    assert!(html.contains(r#"<th class="col-name" aria-sort="ascending">"#));
    assert!(html.contains(r#"<input type="hidden" name="dir" value="asc">"#));
    assert!(html.contains(r#"hx-post="/guilds/2/members/20/reverify""#));
    assert!(html.contains("Verified by admin"));
    assert!(html.contains(r#"data-verify-url="/guilds/2/members/21/verify""#));
    assert!(html.contains(r#"class="notice-row notice-warn" data-for="member-21""#));
    assert!(
        html.contains(r#"href="/guilds/2/verification?sort=name&#38;dir=asc&#38;page=2#members""#)
    );
    assert!(html.contains(r#"id="verify-dialog""#));
    assert!(html.contains(r#"<table class="members hide-xp">"#));
    assert!(html.contains(r#"<th class="col-xp" aria-sort="none">"#));
    assert!(
        html.contains(r#"data-column-toggle="xp">"#),
        "xp starts unchecked"
    );
    assert!(html.contains(r#"data-column-toggle="verified" checked>"#));
    assert!(html.contains(r#"<td class="col-xp number">1,234,567</td>"#));
    assert!(html.contains(r#"<td class="col-guild_joined hint">2024-07-03</td>"#));
    assert!(html.contains(r#"hx-post="/guilds/2/members/20/unverify""#));
    assert!(
        !html.contains(r#"hx-post="/guilds/2/members/21/unverify""#),
        "nothing to remove from unverified members"
    );
}

#[test]
fn verification_page_renders_in_french() {
    let html = VerificationPage {
        lang: Lang::Fr,
        shell: shell(),
        guild: guild(2, "Test", None),
        section: "verification",
        section_title: "nav-verification",
        error: error_message("role_not_allowed"),
        roles: vec![role(11, "Verified", true)],
        verified_role_id: Some(serenity::RoleId::new(11)),
        unverified_role_id: None,
        hypixel_guild: None,
        guild_id: serenity::GuildId::new(2),
        members: members::tests::sample_table(),
    }
    .render()
    .expect("template renders");

    assert!(html.contains(r#"<html lang="fr">"#));
    assert!(html.contains("Vérification"));
    assert!(html.contains("Tu ne peux choisir que des rôles en dessous de ton rôle le plus haut."));
    assert!(html.contains("1 sur 2 vérifiés"));
    assert!(html.contains("Vérifié par un admin"));
    assert!(html.contains("/blob/main/docs/fr/confidentialite.md"));
    assert!(html.contains(
        r#"<button type="submit" name="lang" value="fr" title="Français" class="active""#
    ));
}

#[test]
fn rules_page_renders_groups_rules_and_rank_dropdown() {
    let rule = |id, condition: &str, role: &str| RuleRow {
        id,
        condition: condition.to_owned(),
        role: chip(role),
    };
    let html = RulesPage {
        lang: Lang::En,
        shell: shell(),
        guild: guild(2, "Test", None),
        section: "rules",
        section_title: "nav-rules",
        error: None,
        groups: vec![GroupRow {
            id: 7,
            name: "Ranks".to_owned(),
            separator: chip("━━ Ranks ━━"),
            rules: vec![rule(5, "Hypixel rank is MVP+", "MVP+")],
        }],
        ungrouped: vec![rule(6, "No Hypixel rank", "Default")],
        rule_count: 2,
        roles: vec![role(11, "Verified", true)],
        ranks: hypixel::RANKS,
        hypixel_guild: Some("My Guild".to_owned()),
        guild_ranks: vec!["Guild Master".to_owned(), "Officer".to_owned()],
        guild_ranks_error: false,
    }
    .render()
    .expect("template renders");

    assert!(html.contains("2 rules in 1 group<"));
    assert!(html.contains("<strong>Ranks</strong>"));
    assert!(html.contains("━━ Ranks ━━"));
    assert!(html.contains("/guilds/2/groups/7/delete"));
    assert!(html.contains("/guilds/2/rules/5/delete"));
    assert!(html.contains("/guilds/2/rules/6/delete"));
    assert!(html.contains("<strong>No group</strong>"));
    assert!(html.contains("--role-color: #3ba55d"));
    assert!(html.contains(r#"<option value="7">Ranks</option>"#));
    assert!(html.contains(r#"<option value="NO_RANK">No rank</option>"#));
    assert!(html.contains(r#"<option value="Officer">Officer</option>"#));
}

#[test]
fn nickname_page_renders_fields_texts_and_preview() {
    let format = NicknameFormat {
        custom_texts: vec![nickname::CustomText {
            field: Field::HypixelRank,
            value: "SUPERSTAR".to_owned(),
            text: nickname::ValueText {
                prefix: Some("★".to_owned()),
                label: None,
                suffix: Some("★".to_owned()),
            },
        }],
        ..NicknameFormat::default()
    };
    let guild_ranks = Ok(vec![
        ("Guild Master".to_owned(), "GM".to_owned()),
        ("Officer".to_owned(), "OFC".to_owned()),
    ]);
    let html = NicknamePage {
        lang: Lang::En,
        shell: shell(),
        guild: guild(2, "Test", None),
        section: "nickname",
        section_title: "nav-nickname",
        error: None,
        nickname_enabled: true,
        nickname_separator: format.separator.clone(),
        nickname_rows: NicknameRow::from_format(&format),
        value_tables: value_tables(Lang::En, &format, guild_ranks),
        previews: previews(&format),
    }
    .render()
    .expect("template renders");

    assert!(html.contains(r#"name="hypixel_rank_prefix" value="[""#));
    assert!(html.contains("[MVP+] Notch [OFC]"));
    assert!(
        html.contains("★MVP++★ Sixteen_Chars_Ok"),
        "custom texts show in the preview"
    );
    assert!(html.contains(r#"name="nickname_enabled" checked"#));

    // Hypixel ranks: NO_RANK first, with no default label; SUPERSTAR is row 5
    assert!(html.contains(r#"name="text_hypixel_rank_0_value" value="NO_RANK""#));
    assert!(html.contains(r#"name="text_hypixel_rank_0_default_label" value="""#));
    assert!(html.contains(r#"name="text_hypixel_rank_5_prefix" value="★""#));
    assert!(html.contains(r#"name="text_hypixel_rank_5_default_prefix" value="[""#));
    assert!(html.contains(r#"name="text_hypixel_rank_5_label" value="MVP++""#));
    assert!(html.contains("Custom</span>"));
    // Guild ranks show their tag as the default label
    assert!(html.contains(r#"name="text_guild_rank_tag_1_value" value="Officer""#));
    assert!(html.contains(r#"name="text_guild_rank_tag_1_label" value="OFC""#));
}

#[test]
fn value_tables_explain_a_missing_guild() {
    let tables = value_tables(
        Lang::En,
        &NicknameFormat::default(),
        Err("texts-link-guild"),
    );
    assert_eq!(tables[1].note, Some("texts-link-guild"));
    assert!(tables[1].rows.is_empty());
}

#[test]
fn only_changed_value_texts_become_custom() {
    let row = |part: &str, value: &str| (format!("text_hypixel_rank_0_{part}"), value.to_owned());
    let form: HashMap<String, String> = [
        row("value", "SUPERSTAR"),
        row("default_prefix", "["),
        row("default_label", "MVP++"),
        row("default_suffix", "]"),
        // Prefix emptied, label changed, suffix left as shown
        row("prefix", ""),
        row("label", "Star"),
        row("suffix", "]"),
        (
            "text_guild_rank_tag_0_value".to_owned(),
            "Officer".to_owned(),
        ),
        (
            "text_guild_rank_tag_0_default_label".to_owned(),
            "OFC".to_owned(),
        ),
        ("text_guild_rank_tag_0_label".to_owned(), "OFC".to_owned()),
    ]
    .into_iter()
    .collect();
    let format = parse_nickname_form(&form);

    assert_eq!(
        format.custom_texts.len(),
        1,
        "the untouched guild rank isn't custom"
    );
    let custom = &format.custom_texts[0];
    assert_eq!(
        (custom.field, custom.value.as_str()),
        (Field::HypixelRank, "SUPERSTAR")
    );
    assert_eq!(
        custom.text,
        nickname::ValueText {
            prefix: Some(String::new()),
            label: Some("Star".to_owned()),
            suffix: None,
        }
    );
}

#[test]
fn nickname_form_reorders_and_limits() {
    let form: HashMap<String, String> = [
        ("nickname_enabled", "on"),
        ("separator", " "),
        ("ign_enabled", "on"),
        ("ign_position", "1"),
        ("ign_importance", "1"),
        ("hypixel_rank_enabled", "on"),
        ("hypixel_rank_position", "2"),
        ("hypixel_rank_prefix", "(((((((((((("),
        ("hypixel_rank_suffix", ")"),
        ("hypixel_rank_importance", "not a number"),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value.to_owned()))
    .collect();
    let format = parse_nickname_form(&form);

    assert!(format.enabled);
    let order: Vec<_> = format
        .segments
        .iter()
        .map(|segment| segment.field)
        .collect();
    assert!(order[..2] == [Field::Ign, Field::HypixelRank]);
    assert_eq!(format.segments[1].prefix.len(), MAX_WRAPPING_LEN);
    assert_eq!(format.segments[1].importance, 9);
    assert!(!format.segments[2].enabled, "unchecked fields are disabled");
}

#[test]
fn messages_page_renders_language_and_channels() {
    let channel = |id, name: &str, sendable| ChannelOption {
        id: serenity::ChannelId::new(id),
        name: name.to_owned(),
        sendable,
    };
    let html = MessagesPage {
        lang: Lang::En,
        shell: shell(),
        guild: guild(2, "Test", None),
        section: "messages",
        section_title: "nav-messages",
        error: error_message("channel_not_sendable"),
        server_language: Lang::Fr,
        channels: vec![
            channel(30, "logs", true),
            channel(31, "announcements", false),
        ],
        log_channel_id: Some(serenity::ChannelId::new(30)),
        posted: true,
    }
    .render()
    .expect("template renders");

    assert!(html.contains(r#"<option value="fr" selected>Français</option>"#));
    assert!(html.contains(r#"<option value="30" selected>#logs</option>"#));
    assert!(html.contains(r#"<option value="31" disabled>#announcements</option>"#));
    assert!(html.contains("send messages in this channel."));
    assert!(html.contains(r#"href="/guilds/2/messages" class="active""#));
    assert!(html.contains("The verify message was posted."));
    assert!(html.contains(r#"action="/guilds/2/verify-message""#));
}
