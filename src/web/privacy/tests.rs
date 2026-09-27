use super::*;
use crate::config;

fn user(id: u64) -> serenity::UserId {
    serenity::UserId::new(id)
}

async fn database() -> SqlitePool {
    let db = config::connect_in_memory().await;
    let guild = serenity::GuildId::new(100);
    config::record_verification(
        &db,
        guild,
        user(1),
        "069a79f444e94726a5befca90e38aaf5",
        "Notch",
        None,
    )
    .await
    .unwrap();
    // Verified by admin 3, who is also verified themselves in another server
    config::record_verification(
        &db,
        guild,
        user(2),
        "853c80ef3c3749fdaa49938b674adae6",
        "jeb_",
        Some(user(3)),
    )
    .await
    .unwrap();
    config::record_verification(
        &db,
        serenity::GuildId::new(200),
        user(3),
        "61699b2ed3274a019f1e0ea8c3f06bc6",
        "Dinnerbone",
        None,
    )
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO session (token_hash, user_id, name, avatar_url, expires_at)
         VALUES ('a', 3, 'Admin Three', '', 9999999999)",
    )
    .execute(&db)
    .await
    .unwrap();
    db
}

#[test]
fn admin_ids_are_a_list() {
    assert_eq!(
        parse_admin_ids(Some("111111111111111111, 222222222222222222")).unwrap(),
        [user(111111111111111111), user(222222222222222222)]
    );
    assert!(parse_admin_ids(None).unwrap().is_empty());
    assert!(parse_admin_ids(Some("")).unwrap().is_empty());
    assert!(parse_admin_ids(Some("12,me")).is_err());
}

#[tokio::test]
async fn search_matches_ids_names_and_uuids() {
    let db = database().await;
    let found = |query: &'static str| {
        let db = db.clone();
        async move {
            let mut ids = matching_people(&db, query).await.unwrap();
            ids.sort();
            ids
        }
    };
    assert_eq!(found("1").await, [user(1)]);
    assert_eq!(found("notch").await, [user(1)], "names match without case");
    assert_eq!(found("dinner").await, [user(3)], "names match partially");
    assert_eq!(
        found("853c80ef-3c37-49fd-aa49-938b674adae6").await,
        [user(2)]
    );
    assert_eq!(found("3").await, [user(3)], "admin and panel user");
    assert_eq!(found("admin three").await, [user(3)], "panel names");
    assert!(found("nobody").await.is_empty());
}

#[tokio::test]
async fn erasing_removes_everything_about_a_person() {
    let db = database().await;
    let erased = erase(&db, user(3)).await.unwrap();
    assert_eq!(
        erased,
        Erased {
            verifications: 1,
            sessions: 1,
            admin_mentions: 1,
            guilds: vec![serenity::GuildId::new(200)],
        }
    );
    assert!(matching_people(&db, "3").await.unwrap().is_empty());

    // The member they verified keeps their verification, without the admin's ID
    let other = config::get_verified_member(&db, serenity::GuildId::new(100), user(2))
        .await
        .unwrap()
        .expect("still verified");
    assert_eq!(other.forced_by, None);
    assert!(
        config::get_verified_member(&db, serenity::GuildId::new(100), user(1))
            .await
            .unwrap()
            .is_some(),
        "other people are untouched"
    );
}

#[test]
fn page_lists_each_person_with_an_erase_button() {
    let shell = Shell {
        user: super::super::auth::User {
            id: user(9),
            name: "privacy admin".to_owned(),
            avatar_url: String::new(),
            lang: None,
        },
        guilds: Vec::new(),
        current: None,
        invite_url: String::new(),
        privacy_admin: true,
    };
    let html = DataRequestsPage {
        lang: Lang::En,
        shell,
        query: "notch".to_owned(),
        people: Some(vec![Person {
            id: user(1),
            name: Some("Notch".to_owned()),
            verifications: vec![VerificationRow {
                guild: "Test <server>".to_owned(),
                minecraft_name: "Notch".to_owned(),
                minecraft_uuid: "069a79f444e94726a5befca90e38aaf5".to_owned(),
                verified_on: "2026-09-27".to_owned(),
                by: Some(user(3)),
            }],
            verified_by_them: 0,
            sessions: 2,
        }]),
        erased: Some("Erased.".to_owned()),
    }
    .render()
    .expect("template renders");

    assert!(
        html.contains(r#"href="/data" title="Data requests""#),
        "top bar link"
    );
    assert!(html.contains("Test &#60;server&#62;"), "names are escaped");
    assert!(html.contains("Panel sessions: 2"));
    assert!(
        html.contains("<code>3</code>"),
        "the admin who verified them"
    );
    assert!(html.contains(r#"<input type="hidden" name="user_id" value="1">"#));
    assert!(html.contains(r#"<input type="hidden" name="q" value="notch">"#));
    assert!(html.contains(r#"<div class="alert ok" role="status">Erased.</div>"#));
}
