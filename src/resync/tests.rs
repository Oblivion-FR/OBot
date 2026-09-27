use super::*;

#[test]
fn interval_defaults_to_three_hours() {
    assert_eq!(interval(None).unwrap(), Some(Duration::from_secs(3 * 3600)));
}

#[test]
fn interval_is_set_in_hours_and_zero_turns_it_off() {
    assert_eq!(
        interval(Some("6".to_owned())).unwrap(),
        Some(Duration::from_secs(6 * 3600))
    );
    assert_eq!(
        interval(Some(" 1 ".to_owned())).unwrap(),
        Some(Duration::from_secs(3600))
    );
    assert_eq!(interval(Some("0".to_owned())).unwrap(), None);
}

#[test]
fn interval_refuses_anything_but_whole_hours() {
    assert!(interval(Some("1.5".to_owned())).is_err());
    assert!(interval(Some("-1".to_owned())).is_err());
    assert!(interval(Some("often".to_owned())).is_err());
}
