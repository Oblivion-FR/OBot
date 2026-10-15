use axum::body::Body;
use axum::extract::{Path, Query};
use axum::http::{Request, StatusCode};
use axum::routing::get;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tower::ServiceExt;

use super::*;
use crate::{db, hypixel, mojang};

const UUID: &str = "069a79f444e94726a5befca90e38aaf5";

/// Stands in for Hypixel and Mojang: knows one player, Notch, and one guild
async fn fake_upstream() -> String {
    let app = Router::new()
        .route(
            "/hypixel/player",
            get(|Query(query): Query<HashMap<String, String>>| async move {
                let player = (query["uuid"] == UUID).then(|| {
                    json!({
                        "newPackageRank": "MVP_PLUS",
                        "socialMedia": {"links": {"DISCORD": "notch"}},
                    })
                });
                axum::Json(json!({"success": true, "player": player}))
            }),
        )
        .route(
            "/hypixel/guild",
            get(|Query(query): Query<HashMap<String, String>>| async move {
                let found = query.get("id").map(String::as_str) == Some("g1")
                    || query.get("name").map(String::as_str) == Some("Builders");
                let guild = found.then(|| {
                    json!({
                        "_id": "g1",
                        "name": "Builders",
                        "tag": "BLD",
                        "ranks": [{"name": "Member", "tag": "M", "priority": 1}],
                        "members": [{
                            "uuid": UUID,
                            "rank": "GUILDMASTER",
                            "joined": 1_600_000_000_000_i64,
                            "expHistory": {"2026-10-14": 100, "2026-10-15": 23},
                        }],
                    })
                });
                axum::Json(json!({"success": true, "guild": guild}))
            }),
        )
        .route(
            "/mojang/profiles/{name}",
            get(|Path(name): Path<String>| async move {
                if name.eq_ignore_ascii_case("notch") {
                    axum::Json(json!({"id": UUID, "name": "Notch"})).into_response()
                } else {
                    StatusCode::NOT_FOUND.into_response()
                }
            }),
        )
        .route(
            "/mojang/session/{uuid}",
            get(|Path(uuid): Path<String>| async move {
                if uuid == UUID {
                    axum::Json(json!({"id": UUID, "name": "Notch"})).into_response()
                } else {
                    StatusCode::NO_CONTENT.into_response()
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("port is free");
    let address = listener.local_addr().expect("bound");
    tokio::spawn(async move { axum::serve(listener, app).await });
    format!("http://{address}")
}

use axum::response::IntoResponse;

/// The API against the fake upstream, with a key for the `bot` service
async fn test_app() -> (Router, String) {
    let upstream = fake_upstream().await;
    let http = reqwest::Client::new();
    let db = db::connect_in_memory().await;
    let key = auth::create_key(&db, "bot").await.expect("key is created");
    let state = AppState {
        db,
        hypixel: Arc::new(hypixel::Hypixel::new(
            http.clone(),
            format!("{upstream}/hypixel"),
            "hypixel-key".to_owned(),
        )),
        mojang: Arc::new(mojang::Mojang::new(
            http,
            format!("{upstream}/mojang/profiles"),
            format!("{upstream}/mojang/session"),
        )),
    };
    (app(state), key)
}

async fn call(app: &Router, uri: &str, key: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::get(uri);
    if let Some(key) = key {
        request = request.header("authorization", format!("Bearer {key}"));
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).expect("valid request"))
        .await
        .expect("router answers");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body reads");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn health_needs_no_key() {
    let (app, _) = test_app().await;
    let (status, body) = call(&app, "/v1/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn other_routes_need_a_valid_key() {
    let (app, _) = test_app().await;
    for key in [None, Some("hyt_wrong"), Some("not-even-a-key")] {
        let (status, body) = call(&app, "/v1/minecraft/players/Notch", key).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "unauthorized");
    }
}

#[tokio::test]
async fn minecraft_names_resolve_to_accounts() {
    let (app, key) = test_app().await;
    let (status, body) = call(&app, "/v1/minecraft/players/notch", Some(&key)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"uuid": UUID, "name": "Notch"}));

    let (status, body) = call(&app, "/v1/minecraft/players/nobody", Some(&key)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "minecraft_player_not_found");

    let (status, body) = call(&app, "/v1/minecraft/players/no%20pe", Some(&key)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "invalid_minecraft_name");
}

#[tokio::test]
async fn minecraft_profiles_by_uuid_accept_dashes() {
    let (app, key) = test_app().await;
    let uri = "/v1/minecraft/profiles/069A79F4-44E9-4726-A5BE-FCA90E38AAF5";
    let (status, body) = call(&app, uri, Some(&key)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "Notch");

    let uri = "/v1/minecraft/profiles/00000000000000000000000000000000";
    let (status, _) = call(&app, uri, Some(&key)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn hypixel_players_have_a_rank_and_discord() {
    let (app, key) = test_app().await;
    let (status, body) = call(&app, &format!("/v1/hypixel/players/{UUID}"), Some(&key)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({"uuid": UUID, "rank": "MVP_PLUS", "discord": "notch"})
    );

    let uri = "/v1/hypixel/players/00000000000000000000000000000000?fresh=true";
    let (status, body) = call(&app, uri, Some(&key)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "hypixel_player_not_found");

    let (status, body) = call(&app, "/v1/hypixel/players/nope", Some(&key)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "invalid_uuid");
}

#[tokio::test]
async fn hypixel_guilds_by_id_and_name() {
    let (app, key) = test_app().await;
    let expected = json!({
        "id": "g1",
        "name": "Builders",
        "tag": "BLD",
        "ranks": [{"name": "Guild Master", "tag": "GM"}, {"name": "Member", "tag": "M"}],
        "members": [{
            "uuid": UUID,
            "rank": "Guild Master",
            "joined_at": 1_600_000_000_i64,
            "weekly_xp": 123,
        }],
    });
    let (status, body) = call(&app, "/v1/hypixel/guilds/g1", Some(&key)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, expected);

    let (status, body) = call(&app, "/v1/hypixel/guilds?name=Builders", Some(&key)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, expected);

    let (status, body) = call(&app, "/v1/hypixel/guilds?name=Nobody", Some(&key)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "hypixel_guild_not_found");
}

#[tokio::test]
async fn docs_list_every_route() {
    let (app, _) = test_app().await;
    let (status, spec) = call(&app, "/openapi.json", None).await;
    assert_eq!(status, StatusCode::OK);
    let paths: Vec<&str> = spec["paths"]
        .as_object()
        .expect("paths")
        .keys()
        .map(String::as_str)
        .collect();
    for path in [
        "/v1/health",
        "/v1/minecraft/players/{name}",
        "/v1/minecraft/profiles/{uuid}",
        "/v1/hypixel/players/{uuid}",
        "/v1/hypixel/guilds/{id}",
        "/v1/hypixel/guilds",
    ] {
        assert!(paths.contains(&path), "{path} missing from {paths:?}");
    }
    assert!(spec["components"]["securitySchemes"]["api_key"].is_object());
}
