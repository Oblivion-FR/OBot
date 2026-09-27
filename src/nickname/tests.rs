use super::*;

/// A value whose key is its label, enough for tests without custom texts
fn shown(label: &str) -> Keyed<'_> {
    Keyed {
        key: label,
        label: Some(label),
    }
}

fn values<'a>(rank: Option<&'a str>, ign: &'a str, guild_rank_tag: Option<&'a str>) -> Values<'a> {
    Values {
        hypixel_rank: rank.map(shown),
        ign,
        guild_rank: guild_rank_tag.map(shown),
        guild_tag: Some("OBOT"),
    }
}

fn custom(
    field: Field,
    value: &str,
    prefix: Option<&str>,
    label: Option<&str>,
    suffix: Option<&str>,
) -> CustomText {
    CustomText {
        field,
        value: value.to_owned(),
        text: ValueText {
            prefix: prefix.map(str::to_owned),
            label: label.map(str::to_owned),
            suffix: suffix.map(str::to_owned),
        },
    }
}

#[test]
fn renders_default_format() {
    let format = NicknameFormat::default();
    let nickname = format.render(&values(Some("MVP+"), "Notch", Some("OFC")));
    assert_eq!(nickname, "[MVP+] Notch [OFC]");
}

#[test]
fn skips_missing_values_with_their_wrapping() {
    let format = NicknameFormat::default();
    assert_eq!(format.render(&values(None, "Notch", None)), "Notch");
    assert_eq!(
        format.render(&values(None, "Notch", Some("MBR"))),
        "Notch [MBR]"
    );
}

#[test]
fn drops_least_important_part_when_too_long() {
    let mut format = NicknameFormat::default();
    format.segments[3].enabled = true;
    // 36 characters with everything, the guild tag (importance 4) is dropped first
    let nickname = format.render(&values(Some("MVP++"), "Sixteen_Chars_Ok", Some("GM")));
    assert_eq!(nickname, "[MVP++] Sixteen_Chars_Ok [GM]");
}

#[test]
fn follows_segment_order_and_custom_wrapping() {
    let mut format = NicknameFormat::default();
    format.segments.swap(0, 1);
    format.segments[1].prefix = "(".to_owned();
    format.segments[1].suffix = ")".to_owned();
    format.segments[3].enabled = true;
    format.separator = " | ".to_owned();
    let nickname = format.render(&values(Some("VIP"), "Notch", None));
    assert_eq!(nickname, "Notch | (VIP) | [OBOT]");
}

#[test]
fn truncates_when_a_single_part_is_still_too_long() {
    let mut format = NicknameFormat::default();
    format.segments[1].prefix = "#".repeat(30);
    let nickname = format.render(&values(None, "Notch", None));
    assert_eq!(nickname.chars().count(), MAX_LEN);
}

#[test]
fn custom_texts_replace_only_the_parts_they_set() {
    let format = NicknameFormat {
        custom_texts: vec![
            // Own brackets, usual label
            custom(Field::HypixelRank, "SUPERSTAR", Some("★"), None, Some("★")),
            // Own label, usual brackets
            custom(Field::GuildRankTag, "Officer", None, Some("O"), None),
            // No brackets at all
            custom(
                Field::GuildRankTag,
                "Guild Master",
                Some(""),
                Some("👑"),
                Some(""),
            ),
        ],
        ..NicknameFormat::default()
    };
    let superstar = Keyed {
        key: "SUPERSTAR",
        label: Some("MVP++"),
    };
    let rank = |key, tag| Keyed {
        key,
        label: Some(tag),
    };

    let officer = Values {
        hypixel_rank: Some(superstar),
        ign: "Notch",
        guild_rank: Some(rank("officer", "OFC")),
        guild_tag: None,
    };
    assert_eq!(
        format.render(&officer),
        "★MVP++★ Notch [O]",
        "guild ranks match any case"
    );

    let master = Values {
        guild_rank: Some(rank("Guild Master", "GM")),
        ..officer
    };
    assert_eq!(format.render(&master), "★MVP++★ Notch 👑");
}

#[test]
fn a_custom_label_shows_values_that_have_none() {
    let no_rank = Keyed {
        key: "NO_RANK",
        label: None,
    };
    let player = Values {
        hypixel_rank: Some(no_rank),
        ign: "Notch",
        guild_rank: None,
        guild_tag: None,
    };
    let mut format = NicknameFormat::default();
    assert_eq!(
        format.render(&player),
        "Notch",
        "no rank shows nothing by default"
    );

    format.custom_texts = vec![custom(
        Field::HypixelRank,
        "NO_RANK",
        None,
        Some("Guest"),
        None,
    )];
    assert_eq!(format.render(&player), "[Guest] Notch");
}
