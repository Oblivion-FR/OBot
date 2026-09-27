use super::*;

#[test]
fn message_mentions_its_button_in_every_language() {
    for lang in Lang::ALL {
        let text = lang.t("verify-message");
        assert!(
            text.contains(&format!("**{}**", lang.t("verify-button"))),
            "{text}"
        );
        assert!(!text.contains("verify-"), "every message exists: {text}");
    }
}

#[test]
fn message_ends_with_the_data_notice() {
    let message = serde_json::to_value(message(Lang::Fr)).expect("serializes");
    let content = message["content"].as_str().expect("has content");
    assert!(content.contains("\n-# Pour te donner tes rôles"));
    assert!(
        content.ends_with(
            "(<https://github.com/Oblivion-FR/OBot/blob/main/docs/fr/confidentialite.md>)"
        )
    );
}

#[test]
fn message_carries_the_button() {
    let message = serde_json::to_value(message(Lang::Fr)).expect("serializes");
    let button = &message["components"][0]["components"][0];
    assert_eq!(button["custom_id"], BUTTON_ID);
    assert_eq!(button["label"], "Vérifier");
}
