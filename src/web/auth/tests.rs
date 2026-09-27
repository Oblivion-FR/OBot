use super::*;

fn user() -> User {
    User {
        id: serenity::UserId::new(42),
        name: "admin".to_owned(),
        avatar_url: "https://cdn.example/avatar.png".to_owned(),
        lang: Some(Lang::Fr),
    }
}

#[tokio::test]
async fn sessions_survive_in_the_database() {
    let db = crate::config::connect_in_memory().await;
    let token = Sessions(db.clone()).create(&user()).await.unwrap();

    // A new `Sessions`, like after a restart, still knows the login
    let sessions = Sessions(db.clone());
    let found = sessions.get(&token).await.expect("session is found");
    assert_eq!(found.id, serenity::UserId::new(42));
    assert_eq!(found.lang, Some(Lang::Fr));

    let stored: String = sqlx::query_scalar("SELECT token_hash FROM session")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_ne!(stored, token, "the token itself isn't stored");

    sessions.remove(&token).await.unwrap();
    assert!(sessions.get(&token).await.is_none());
}

#[tokio::test]
async fn expired_sessions_are_ignored_and_cleaned_up() {
    let db = crate::config::connect_in_memory().await;
    let sessions = Sessions(db.clone());
    let token = sessions.create(&user()).await.unwrap();
    sqlx::query("UPDATE session SET expires_at = 0")
        .execute(&db)
        .await
        .unwrap();
    assert!(sessions.get(&token).await.is_none());

    sessions.create(&user()).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM session")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(count, 1, "logging in drops expired sessions");
}

#[test]
fn unknown_tokens_hash_differently() {
    assert_ne!(token_hash("a"), token_hash("b"));
    assert_eq!(token_hash("a").len(), 64);
}
