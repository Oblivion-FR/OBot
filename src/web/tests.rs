use super::*;

#[test]
fn initials_take_first_letters() {
    assert_eq!(initials("Hypixel Guild Hub Network"), "HGH");
    assert_eq!(initials("obot"), "o");
    assert_eq!(initials("Test <server>"), "Ts");
}
