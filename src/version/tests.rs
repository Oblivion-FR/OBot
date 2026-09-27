use super::*;

#[test]
fn commit_links_to_its_page_in_the_repository() {
    assert_eq!(
        commit_url_of("20c7fcf").as_deref(),
        Some("https://github.com/Oblivion-FR/OBot/commit/20c7fcf")
    );
    assert_eq!(commit_url_of("unknown"), None);
}

#[test]
fn label_shows_version_and_commit() {
    assert_eq!(label(), format!("{NUMBER} ({COMMIT})"));
}
