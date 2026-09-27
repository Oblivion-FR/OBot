use super::*;
use std::collections::BTreeSet;

/// Message ids of a language's files, read from the lines starting with `id =`
fn message_ids(lang: Lang) -> BTreeSet<String> {
    let (_, sources) = FILES
        .iter()
        .find(|(file_lang, _)| *file_lang == lang)
        .expect("every language has files");
    sources
        .iter()
        .flat_map(|source| source.lines())
        .filter_map(|line| {
            let (id, _) = line.split_once(" =")?;
            let is_id = id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            (is_id && !id.is_empty()).then(|| id.to_owned())
        })
        .collect()
}

#[test]
fn translation_files_are_valid() {
    check().expect("every .ftl file parses");
}

#[test]
fn every_language_has_every_message() {
    let english = message_ids(Lang::En);
    assert!(!english.is_empty());
    for lang in Lang::ALL {
        let ids = message_ids(lang);
        let missing: Vec<_> = english.difference(&ids).collect();
        let extra: Vec<_> = ids.difference(&english).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "`{}`: missing {missing:?}, not in English {extra:?}",
            lang.code()
        );
    }
}

#[test]
fn languages_come_from_discord_locales_and_accept_language() {
    assert_eq!(Lang::from_tag("fr"), Some(Lang::Fr));
    assert_eq!(Lang::from_tag("en-US"), Some(Lang::En));
    assert_eq!(Lang::from_tag("FR-ca;q=0.8"), Some(Lang::Fr));
    assert_eq!(Lang::from_tag("de"), None);
}

#[test]
fn messages_are_translated_with_their_variables() {
    let english = tr!(Lang::En, "verify-done", name = "Notch");
    let french = tr!(Lang::Fr, "verify-done", name = "Notch");
    assert_eq!(english, "You are verified as `Notch`!");
    assert_eq!(french, "Tu es vérifié en tant que `Notch` !");
    assert!(
        !english.contains('\u{2068}'),
        "no invisible direction marks around variables"
    );
}

#[test]
fn multiline_messages_keep_their_line_breaks() {
    let reply = tr!(
        Lang::En,
        "version-reply",
        version = "0.3.0",
        commit = "abc1234",
        repository = "https://example.com"
    );
    assert_eq!(
        reply,
        "OBot v0.3.0 (abc1234)\nSource: <https://example.com>"
    );
}

#[test]
fn unknown_messages_show_their_id() {
    assert_eq!(Lang::Fr.t("no-such-message"), "no-such-message");
}

#[test]
fn commands_get_their_texts_in_every_language() {
    let mut commands = vec![crate::commands::verify()];
    localize_commands(&mut commands);
    let verify = &commands[0];
    assert_eq!(
        verify.description.as_deref(),
        Some("Get your roles by proving you own a Minecraft account")
    );
    let french = |list: &[(Cow<'static, str>, Cow<'static, str>)]| {
        list.iter()
            .find(|(locale, _)| locale == "fr")
            .map(|(_, text)| text.to_string())
    };
    assert_eq!(
        french(&verify.description_localizations).as_deref(),
        Some("Obtiens tes rôles en prouvant que tu possèdes un compte Minecraft")
    );
    let username = &verify.parameters[0];
    assert_eq!(
        username.description.as_deref(),
        Some("Your Minecraft username")
    );
    assert_eq!(
        french(&username.name_localizations).as_deref(),
        Some("pseudo")
    );
}
