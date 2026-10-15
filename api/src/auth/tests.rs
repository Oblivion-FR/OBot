use super::*;

#[tokio::test]
async fn created_keys_find_their_service_until_revoked() {
    let db = crate::db::connect_in_memory().await;
    let key = create_key(&db, "bot").await.expect("key is created");
    assert!(key.starts_with(KEY_PREFIX));

    let client = find_client(&db, &key).await.expect("query works");
    assert_eq!(client.map(|client| client.name).as_deref(), Some("bot"));
    assert!(
        find_client(&db, "hyt_unknown")
            .await
            .expect("query works")
            .is_none()
    );

    sqlx::query("UPDATE api_key SET revoked_at = 1")
        .execute(&db)
        .await
        .expect("revoked");
    assert!(find_client(&db, &key).await.expect("query works").is_none());
}

#[tokio::test]
async fn only_the_hash_is_stored() {
    let db = crate::db::connect_in_memory().await;
    let key = create_key(&db, "dashboard").await.expect("key is created");
    let (stored,): (String,) = sqlx::query_as("SELECT key_hash FROM api_key")
        .fetch_one(&db)
        .await
        .expect("one key");
    assert_ne!(stored, key);
    assert_eq!(stored, hash(&key));
}
