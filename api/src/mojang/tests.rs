use super::*;

#[test]
fn names_are_letters_digits_and_underscores() {
    assert!(valid_name("Notch"));
    assert!(valid_name("a_b_1"));
    assert!(valid_name("x"));
    assert!(!valid_name(""));
    assert!(!valid_name("seventeen_chars_x"));
    assert!(!valid_name("../player"));
    assert!(!valid_name("na me"));
    assert!(!valid_name("é"));
}

#[test]
fn uuids_lose_their_dashes_and_capitals() {
    let plain = "069a79f444e94726a5befca90e38aaf5";
    assert_eq!(normalize_uuid(plain).as_deref(), Some(plain));
    assert_eq!(
        normalize_uuid("069A79F4-44E9-4726-A5BE-FCA90E38AAF5").as_deref(),
        Some(plain)
    );
    assert_eq!(normalize_uuid("069a79f4"), None);
    assert_eq!(normalize_uuid("069a79f444e94726a5befca90e38aazz"), None);
    assert_eq!(normalize_uuid("069a79f4-44e94726a5befca90e38aaf5"), None);
}
