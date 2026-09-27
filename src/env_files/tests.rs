use super::*;

#[test]
fn obot_env_picks_the_mode() {
    assert_eq!(mode(Some("production".to_owned())).unwrap(), "production");
    assert_eq!(mode(Some("staging-2".to_owned())).unwrap(), "staging-2");
}

#[test]
fn build_profile_picks_the_mode_when_obot_env_is_unset() {
    let default = if cfg!(debug_assertions) {
        "development"
    } else {
        "production"
    };
    assert_eq!(mode(None).unwrap(), default);
    assert_eq!(mode(Some(String::new())).unwrap(), default);
}

#[test]
fn modes_that_could_escape_the_file_name_are_refused() {
    assert!(mode(Some("../secrets".to_owned())).is_err());
    assert!(mode(Some("prod uction".to_owned())).is_err());
}

#[test]
fn files_go_from_most_to_least_specific() {
    assert_eq!(
        files("production"),
        [
            ".env.production.local",
            ".env.production",
            ".env.local",
            ".env"
        ]
    );
}

#[test]
fn files_are_only_read_from_the_directory_itself() {
    let parent = std::env::temp_dir().join(format!("obot-env-{}", std::process::id()));
    let directory = parent.join("project");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        parent.join(".env"),
        "OBOT_TEST_FROM_PARENT=1
",
    )
    .unwrap();
    std::fs::write(
        directory.join(".env.test"),
        "OBOT_TEST_FROM_MODE=1
",
    )
    .unwrap();

    let loaded = load_from(&directory, "test").unwrap();
    std::fs::remove_dir_all(&parent).unwrap();

    assert_eq!(loaded, [".env.test"]);
    assert!(
        std::env::var("OBOT_TEST_FROM_PARENT").is_err(),
        "a parent's .env is ignored"
    );
    assert_eq!(std::env::var("OBOT_TEST_FROM_MODE").unwrap(), "1");
}
