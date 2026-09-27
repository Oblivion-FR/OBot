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
