use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use pokedash::models::get_pokemon_characters;
use pokedash::server::{create_router, AppState};
use pokedash::service::GameService;
use pokedash::storage::Storage;
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_pokemon_characters_list() {
    let characters = get_pokemon_characters();
    assert!(characters.len() >= 30, "Should have a rich list of Pokémon characters");

    let ash = characters.iter().find(|c| c.username == "Ash");
    assert!(ash.is_some(), "Ash Ketchum should be in characters list");
    assert_eq!(ash.unwrap().name, "Ash Ketchum");

    let red = characters.iter().find(|c| c.username == "Red");
    assert!(red.is_some(), "Red should be in characters list");

    let cynthia = characters.iter().find(|c| c.username == "Cynthia");
    assert!(cynthia.is_some(), "Cynthia should be in characters list");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_create_guest_session_random() {
    let storage = Storage::new().await.expect("Failed to initialize Storage");

    let auth_res = storage
        .create_guest_session(None)
        .await
        .expect("Failed to create random guest session");

    assert!(auth_res.token.starts_with("guest_"));
    assert!(!auth_res.requires_verification);
    assert!(auth_res.user.email.ends_with("@guest.pokedash.local"));

    // Verify token can be verified
    let verified = storage
        .verify_token(&auth_res.token)
        .expect("Guest token should be verified");
    assert_eq!(verified.username, auth_res.user.username);
    assert_eq!(verified.id, auth_res.user.id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_create_guest_session_with_pokemon_character() {
    let storage = Storage::new().await.expect("Failed to initialize Storage");

    let auth_res = storage
        .create_guest_session(Some("Cynthia"))
        .await
        .expect("Failed to create guest session for Cynthia");

    assert!(auth_res.token.starts_with("guest_cynthia_"));
    assert!(auth_res.user.username.starts_with("Cynthia"));
    assert!(!auth_res.requires_verification);

    // Verify player profile was initialized with Cynthia's signature starter (Garchomp #445)
    let player = storage.load_player(&auth_res.user.username);
    assert_eq!(player.username, auth_res.user.username);
    assert_eq!(player.party.len(), 1);
    assert_eq!(player.party[0].id, 445, "Cynthia's signature starter should be Garchomp");
    assert_eq!(player.party[0].name, "Garchomp");

    let dex = storage.load_pokedex(&auth_res.user.username);
    assert_eq!(dex.len(), 1);
    assert_eq!(dex[0].id, 445);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_guest_auth_http_endpoints() {
    let storage = Arc::new(Storage::new().await.expect("Failed to initialize Storage"));
    let service = GameService::new(storage.clone());
    let app = create_router(AppState { service });

    // 1. GET /api/auth/guest-characters
    let req = Request::builder()
        .uri("/api/auth/guest-characters")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let characters = val.get("characters").and_then(|v| v.as_array()).unwrap();
    assert!(characters.iter().any(|c| c["username"] == "Ash"));
    assert!(characters.iter().any(|c| c["username"] == "Red"));
    assert!(characters.iter().any(|c| c["username"] == "Misty"));

    // 2. Unauthenticated request to /api/player must be 401
    let req_unauth = Request::builder()
        .uri("/api/player")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res_unauth = app.clone().oneshot(req_unauth).await.unwrap();
    assert_eq!(res_unauth.status(), StatusCode::UNAUTHORIZED);

    // 3. POST /api/auth/guest to login as "Misty"
    let req_guest = Request::builder()
        .uri("/api/auth/guest")
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"character": "Misty"}"#))
        .unwrap();

    let res_guest = app.clone().oneshot(req_guest).await.unwrap();
    assert_eq!(res_guest.status(), StatusCode::OK);

    let guest_bytes = axum::body::to_bytes(res_guest.into_body(), 1024 * 1024).await.unwrap();
    let guest_val: serde_json::Value = serde_json::from_slice(&guest_bytes).unwrap();
    let token = guest_val["token"].as_str().expect("Token should be present");
    let username = guest_val["user"]["username"].as_str().expect("Username should be present");
    assert!(token.starts_with("guest_misty_"));
    assert!(username.starts_with("Misty"));

    // 4. Authenticated request to /api/auth/me with guest token
    let req_me = Request::builder()
        .uri("/api/auth/me")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res_me = app.clone().oneshot(req_me).await.unwrap();
    assert_eq!(res_me.status(), StatusCode::OK);

    let me_bytes = axum::body::to_bytes(res_me.into_body(), 1024 * 1024).await.unwrap();
    let me_val: serde_json::Value = serde_json::from_slice(&me_bytes).unwrap();
    assert_eq!(me_val["authenticated"], true);
    assert_eq!(me_val["user"]["username"], username);

    // 5. Authenticated request to /api/player with guest token
    let req_player = Request::builder()
        .uri("/api/player")
        .method("GET")
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let res_player = app.clone().oneshot(req_player).await.unwrap();
    assert_eq!(res_player.status(), StatusCode::OK);

    let player_bytes = axum::body::to_bytes(res_player.into_body(), 1024 * 1024).await.unwrap();
    let player_val: serde_json::Value = serde_json::from_slice(&player_bytes).unwrap();
    assert_eq!(player_val["player"]["username"], username);
    assert_eq!(player_val["player"]["party"][0]["name"], "Starmie");
}
