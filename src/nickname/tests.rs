use super::*;

fn values<'a>(rank: Option<&'a str>, ign: &'a str, guild_rank_tag: Option<&'a str>) -> Values<'a> {
    Values {
        hypixel_rank: rank,
        ign,
        guild_rank_tag,
        guild_tag: Some("OBOT"),
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
