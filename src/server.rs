use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, Path, Query, State},
    http::{header, request::Parts, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::game::{
    calculate_catch_chance, get_generation, get_level_discount, get_level_xp_range,
    get_player_level,
};
use crate::models::{AuthResponse, BuyResult, CatchAttemptResult, CaughtPokemon};
use crate::service::GameService;

#[derive(Clone)]
pub struct AppState {
    pub service: GameService,
}

/// Extractor to verify the Supabase Bearer token and extract the Trainer identity
pub struct AuthenticatedTrainer {
    pub username: String,
    pub user_id: String,
    pub email: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedTrainer
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|val| val.to_str().ok());

        let token = match auth_header {
            Some(h) if h.starts_with("Bearer ") => &h[7..],
            Some(h) if !h.trim().is_empty() => h.trim(),
            _ => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": "Authentication required. Please sign in to play PokéDash.",
                        "code": "unauthorized"
                    })),
                ));
            }
        };

        let app_state = AppState::from_ref(state);
        let user = if token.starts_with("guest_") {
            app_state.service.storage.verify_token(token).map_err(|e| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": format!("Invalid guest session: {}. Please sign in again.", e),
                        "code": "guest_session_invalid"
                    })),
                )
            })?
        } else if let Some(supabase) = &app_state.service.storage.supabase {
            supabase.verify_token(token).await.map_err(|e| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "error": format!("Invalid or expired session: {}. Please sign in again.", e),
                        "code": "session_expired"
                    })),
                )
            })?
        } else {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Supabase authentication is not configured on this server." })),
            ));
        };

        Ok(AuthenticatedTrainer {
            username: user.username,
            user_id: user.id,
            email: user.email,
        })
    }
}

// -------------------------------------------------------------
// Request Payloads
// -------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SignupPayload {
    pub email: String,
    pub password: String,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginPayload {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyPayload {
    pub email: String,
    pub token: String,
    pub r#type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResendPayload {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct CatchPayload {
    pub ball: String,
}

#[derive(Debug, Deserialize)]
pub struct AttackPayload {
    pub move_index: usize,
}

#[derive(Debug, Deserialize)]
pub struct PartySelectPayload {
    pub pokemon_id: u32,
}

#[derive(Debug, Deserialize)]
pub struct ArenaStartPayload {
    pub arena_id: String,
}

#[derive(Debug, Deserialize)]
pub struct BuyPayload {
    pub item: String,
    #[serde(default = "default_amount")]
    pub amount: u32,
}

fn default_amount() -> u32 {
    1
}

#[derive(Debug, Deserialize)]
pub struct SellPayload {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct SellAllPayload {
    pub rarity: String,
}

#[derive(Debug, Deserialize)]
pub struct PokedexQuery {
    pub search: Option<String>,
    pub gen: Option<u32>,
    pub r#type: Option<String>,
    pub rarity: Option<String>,
    pub sort: Option<String>,
    pub owned: Option<bool>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        // Public endpoints
        .route("/status", get(get_status))
        .route("/leaderboard", get(get_leaderboard))
        // Authentication endpoints
        .route("/auth/signup", post(post_auth_signup))
        .route("/auth/login", post(post_auth_login))
        .route("/auth/verify", post(post_auth_verify))
        .route("/auth/resend", post(post_auth_resend))
        .route("/auth/guest", post(post_auth_guest))
        .route("/auth/guest-characters", get(get_guest_characters))
        .route("/auth/me", get(get_auth_me))
        // Protected Game Endpoints (Strictly require AuthenticatedTrainer)
        .route("/player", get(get_player))
        .route("/player/reset", post(reset_player))
        .route("/hunt", post(post_hunt))
        .route("/wild", get(get_wild))
        .route("/catch", post(post_catch))
        .route("/flee", post(post_flee))
        .route("/battle/wild-attack", post(post_wild_attack))
        .route("/battle/heal", post(post_heal))
        .route("/party/select", post(post_party_select))
        .route("/arena", get(get_arena))
        .route("/arena/start", post(post_arena_start))
        .route("/arena/attack", post(post_arena_attack))
        .route("/pokedex", get(get_pokedex))
        .route("/catalog", get(get_catalog))
        .route("/pokemon/:id", get(get_pokemon_by_id))
        .route("/store", get(get_store))
        .route("/store/buy", post(post_store_buy))
        .route("/store/sell", post(post_store_sell))
        .route("/store/sell-all", post(post_store_sell_all))
        .route("/store/sell-dupes", post(post_store_sell_dupes));

    Router::new()
        .nest("/api", api_routes)
        .route("/keyboard", get(serve_keyboard))
        .nest_service("/sprites", ServeDir::new("static/sprites"))
        .nest_service("/fonts", ServeDir::new("static/fonts"))
        .nest_service("/backgrounds", ServeDir::new("static/backgrounds"))
        .nest_service("/pokeballs", ServeDir::new("static/pokeballs"))
        .nest_service("/static", ServeDir::new("static"))
        .fallback(serve_frontend)
        .layer(cors)
        .with_state(state)
}

async fn serve_keyboard() -> impl IntoResponse {
    let html_path = std::path::Path::new("static/keyboard.html");
    if html_path.exists() {
        match std::fs::read_to_string(html_path) {
            Ok(content) => (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                content,
            ),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain")],
                "Could not read keyboard.html".to_string(),
            ),
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain")],
            "keyboard.html not found".to_string(),
        )
    }
}

async fn serve_frontend() -> impl IntoResponse {
    let html_path = std::path::Path::new("static/index.html");
    if html_path.exists() {
        match std::fs::read_to_string(html_path) {
            Ok(content) => (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                content,
            ),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain")],
                "Could not read index.html".to_string(),
            ),
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain")],
            "Web interface not found in static/index.html".to_string(),
        )
    }
}

// -------------------------------------------------------------
// Auth Handlers
// -------------------------------------------------------------

async fn post_auth_signup(
    State(state): State<AppState>,
    Json(payload): Json<SignupPayload>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    let email = payload.email.trim();
    let username = payload.username.trim();
    let password = payload.password.trim();

    if email.is_empty() || !email.contains('@') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Valid email address is required" })),
        ));
    }
    if username.len() < 3 || username.len() > 20 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Trainer username must be between 3 and 20 characters" })),
        ));
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Username can only contain letters, numbers, hyphens, and underscores" })),
        ));
    }
    if password.len() < 6 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Password must be at least 6 characters long" })),
        ));
    }

    let supabase = state
        .service
        .storage
        .supabase
        .as_ref()
        .ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Supabase authentication is not configured" })),
            )
        })?;

    // Check if username is already taken
    match supabase.is_username_taken(username).await {
        Ok(taken) if taken => {
            return Err((
                StatusCode::CONFLICT,
                Json(json!({ "error": format!("Trainer username '{}' is already taken. Please choose another.", username) })),
            ));
        }
        _ => {}
    }

    match supabase.signup(email, password, username).await {
        Ok(auth_res) => {
            // Also ensure default player profile is initialized in DB
            let mut def_player = crate::models::PlayerData::default();
            def_player.username = username.to_string();
            let _ = supabase.save_player(&def_player).await;
            Ok(Json(auth_res))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

async fn post_auth_login(
    State(state): State<AppState>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    let email = payload.email.trim();
    let password = payload.password.trim();

    if email.is_empty() || password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Email and password are required" })),
        ));
    }

    let supabase = state
        .service
        .storage
        .supabase
        .as_ref()
        .ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Supabase authentication is not configured" })),
            )
        })?;

    match supabase.login(email, password).await {
        Ok(auth_res) => {
            // Ensure player record exists in DB
            let existing = supabase.load_player(&auth_res.user.username).await.unwrap_or(None);
            if existing.is_none() {
                let mut def_player = crate::models::PlayerData::default();
                def_player.username = auth_res.user.username.clone();
                let _ = supabase.save_player(&def_player).await;
            }
            Ok(Json(auth_res))
        }
        Err(e) => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

async fn post_auth_verify(
    State(state): State<AppState>,
    Json(payload): Json<VerifyPayload>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    let email = payload.email.trim();
    let token = payload.token.trim();

    let supabase = state
        .service
        .storage
        .supabase
        .as_ref()
        .ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Supabase authentication is not configured" })),
            )
        })?;

    match supabase.verify_code(email, token, payload.r#type.as_deref()).await {
        Ok(auth_res) => {
            let mut def_player = crate::models::PlayerData::default();
            def_player.username = auth_res.user.username.clone();
            let _ = supabase.save_player(&def_player).await;
            Ok(Json(auth_res))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

async fn post_auth_resend(
    State(state): State<AppState>,
    Json(payload): Json<ResendPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let email = payload.email.trim();
    if email.is_empty() || !email.contains('@') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Valid email address is required" })),
        ));
    }

    let supabase = state
        .service
        .storage
        .supabase
        .as_ref()
        .ok_or_else(|| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "Supabase authentication is not configured" })),
            )
        })?;

    match supabase.resend_code(email).await {
        Ok(_) => Ok(Json(json!({ "message": "Confirmation code resent to your email!" }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

async fn post_auth_guest(
    State(state): State<AppState>,
    Json(payload): Json<crate::models::GuestLoginPayload>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    let req_name = payload.character.as_deref().or(payload.username.as_deref());
    match state.service.storage.create_guest_session(req_name).await {
        Ok(res) => Ok(Json(res)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Could not issue guest passport: {}", e) })),
        )),
    }
}

async fn get_guest_characters() -> Json<serde_json::Value> {
    let characters = crate::models::get_pokemon_characters();
    Json(json!({
        "characters": characters
    }))
}

async fn get_auth_me(
    trainer: AuthenticatedTrainer,
) -> Json<serde_json::Value> {
    Json(json!({
        "authenticated": true,
        "user": {
            "id": trainer.user_id,
            "username": trainer.username,
            "email": trainer.email,
        }
    }))
}

// -------------------------------------------------------------
// Public Endpoints
// -------------------------------------------------------------

async fn get_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "online",
        "version": "0.3.0",
        "auth": {
            "primary": "supabase_email",
            "guest": true,
            "enabled": state.service.storage.supabase.is_some(),
        },
        "total_pokemon": 1025,
    }))
}

async fn get_leaderboard(State(state): State<AppState>) -> Json<serde_json::Value> {
    let leaderboard = state.service.get_leaderboard();
    Json(json!({
        "leaderboard": leaderboard,
        "total_trainers": leaderboard.len(),
    }))
}

// -------------------------------------------------------------
// Protected Game Endpoints
// -------------------------------------------------------------

async fn get_player(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let player = state.service.get_player(&trainer.username);
    let level = get_player_level(player.xp);
    let (prev_xp, next_xp) = get_level_xp_range(level);
    let discount = get_level_discount(level);
    let pokedex = state.service.get_pokedex(&trainer.username);

    let unique_count = player.stats.lifetime_unique_pids.len();
    let total_caught = player.stats.total_lifetime_caught;

    Json(json!({
        "player": player,
        "level": level,
        "prev_xp": prev_xp,
        "next_xp": next_xp,
        "discount_pct": (discount * 100.0).round() as u32,
        "unique_count": unique_count,
        "total_caught": total_caught,
        "dex_count": pokedex.len(),
    }))
}

async fn reset_player(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.service.reset_player(&trainer.username).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        )
    })?;
    Ok(Json(json!({ "message": format!("Player {} reset to fresh state", trainer.username) })))
}

async fn post_hunt(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.hunt(&trainer.username) {
        Ok(encounter) => {
            let player = state.service.get_player(&trainer.username);
            let level = get_player_level(player.xp);
            let dex = state.service.get_pokedex(&trainer.username);
            let already_caught = dex.iter().any(|p| p.id == encounter.pokemon.id);

            let mut odds = HashMap::new();
            for ball in crate::game::STORE_BALLS {
                let chance = calculate_catch_chance(&encounter.pokemon, ball.id, level, already_caught, encounter.current_hp, encounter.max_hp);
                odds.insert(ball.id, (chance * 100.0).round() as u32);
            }

            let cry_url = format!("https://raw.githubusercontent.com/PokeAPI/cries/main/cries/pokemon/latest/{}.ogg", encounter.pokemon.id);
            let art_url = format!("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/{}.png", encounter.pokemon.id);
            let showdown_url = format!("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/showdown/{}.gif", encounter.pokemon.id);

            Ok(Json(json!({
                "encounter": encounter,
                "already_caught": already_caught,
                "ball_odds": odds,
                "cry_url": cry_url,
                "art_url": art_url,
                "showdown_url": showdown_url,
                "player_lead": player.party.first(),
            })))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn get_wild(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    if let Some(wild) = state.service.get_wild(&trainer.username) {
        let player = state.service.get_player(&trainer.username);
        let level = get_player_level(player.xp);
        let dex = state.service.get_pokedex(&trainer.username);
        let already_caught = dex.iter().any(|p| p.id == wild.pokemon.id);

        let mut odds = HashMap::new();
        for ball in crate::game::STORE_BALLS {
            let chance = calculate_catch_chance(&wild.pokemon, ball.id, level, already_caught, wild.current_hp, wild.max_hp);
            odds.insert(ball.id, (chance * 100.0).round() as u32);
        }

        let cry_url = format!("https://raw.githubusercontent.com/PokeAPI/cries/main/cries/pokemon/latest/{}.ogg", wild.pokemon.id);
        let art_url = format!("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/{}.png", wild.pokemon.id);
        let showdown_url = format!("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/showdown/{}.gif", wild.pokemon.id);

        Json(json!({
            "has_wild": true,
            "encounter": wild,
            "already_caught": already_caught,
            "ball_odds": odds,
            "cry_url": cry_url,
            "art_url": art_url,
            "showdown_url": showdown_url,
            "player_lead": player.party.first(),
        }))
    } else {
        Json(json!({
            "has_wild": false,
            "encounter": null,
        }))
    }
}

async fn post_catch(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
    Json(payload): Json<CatchPayload>,
) -> Result<Json<CatchAttemptResult>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.catch(&trainer.username, &payload.ball) {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn post_wild_attack(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
    Json(payload): Json<AttackPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.wild_attack(&trainer.username, payload.move_index) {
        Ok(res) => Ok(Json(json!(res))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn post_flee(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.flee(&trainer.username) {
        Ok(msg) => Ok(Json(json!({ "message": msg }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn post_heal(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.heal_party(&trainer.username) {
        Ok(msg) => {
            let player = state.service.get_player(&trainer.username);
            Ok(Json(json!({
                "message": msg,
                "party": player.party,
            })))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn post_party_select(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
    Json(payload): Json<PartySelectPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.set_active_pokemon(&trainer.username, payload.pokemon_id) {
        Ok(msg) => {
            let player = state.service.get_player(&trainer.username);
            Ok(Json(json!({
                "message": msg,
                "party": player.party,
            })))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn get_arena(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let divisions = state.service.get_arena_list(&trainer.username);
    let active_battle = state.service.get_arena_battle(&trainer.username);
    let player = state.service.get_player(&trainer.username);
    Json(json!({
        "divisions": divisions,
        "active_battle": active_battle,
        "player_lead": player.party.first(),
        "badges_owned": player.arena_badges,
    }))
}

async fn post_arena_start(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
    Json(payload): Json<ArenaStartPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.start_arena_battle(&trainer.username, &payload.arena_id) {
        Ok(battle) => Ok(Json(json!(battle))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn post_arena_attack(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
    Json(payload): Json<AttackPayload>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.arena_attack(&trainer.username, payload.move_index) {
        Ok(res) => Ok(Json(json!(res))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn get_pokedex(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
    Query(query): Query<PokedexQuery>,
) -> Json<serde_json::Value> {
    let mut dex = state.service.get_pokedex(&trainer.username);

    if let Some(ref search) = query.search {
        let s = search.trim().to_lowercase();
        dex.retain(|p| p.name.to_lowercase().contains(&s) || p.id.to_string() == s);
    }
    if let Some(gen) = query.gen {
        dex.retain(|p| get_generation(p.id) == gen);
    }
    if let Some(ref t) = query.r#type {
        let target = t.trim().to_lowercase();
        dex.retain(|p| p.types.iter().any(|item| item.to_lowercase() == target));
    }
    if let Some(ref r) = query.rarity {
        let target = r.trim().to_lowercase();
        dex.retain(|p| p.rarity.to_lowercase() == target);
    }

    let sort_mode = query.sort.as_deref().unwrap_or("id");
    match sort_mode {
        "id_desc" => dex.sort_by(|a, b| b.id.cmp(&a.id)),
        "rarity" => {
            let rarity_rank = |r: &str| match r.to_lowercase().as_str() {
                "mythical" => 7,
                "legendary" => 6,
                "epic" => 5,
                "ultrarare" => 4,
                "rare" => 3,
                "uncommon" => 2,
                _ => 1,
            };
            dex.sort_by(|a, b| rarity_rank(&b.rarity).cmp(&rarity_rank(&a.rarity)));
        }
        "bst" => dex.sort_by(|a, b| b.stats.total.cmp(&a.stats.total)),
        "recent" => dex.sort_by(|a, b| b.caught_at.partial_cmp(&a.caught_at).unwrap_or(std::cmp::Ordering::Equal)),
        _ => dex.sort_by_key(|p| p.id),
    }

    let total = dex.len();
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(50).max(1);
    let total_pages = (total + limit - 1) / limit;

    let start = (page - 1) * limit;
    let paginated: Vec<CaughtPokemon> = dex.into_iter().skip(start).take(limit).collect();

    Json(json!({
        "total": total,
        "page": page,
        "limit": limit,
        "total_pages": total_pages,
        "pokemon": paginated,
    }))
}

async fn get_catalog(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
    Query(query): Query<PokedexQuery>,
) -> Json<serde_json::Value> {
    let mut catalog = state.service.storage.pokemon_list.clone();
    let dex = state.service.get_pokedex(&trainer.username);

    let mut owned_map: HashMap<u32, usize> = HashMap::new();
    for p in dex {
        *owned_map.entry(p.id).or_insert(0) += 1;
    }

    if let Some(ref search) = query.search {
        let s = search.trim().to_lowercase();
        catalog.retain(|p| p.name.to_lowercase().contains(&s) || p.id.to_string() == s);
    }
    if let Some(gen) = query.gen {
        catalog.retain(|p| get_generation(p.id) == gen);
    }
    if let Some(ref t) = query.r#type {
        let target = t.trim().to_lowercase();
        catalog.retain(|p| p.types.iter().any(|item| item.to_lowercase() == target));
    }
    if let Some(ref r) = query.rarity {
        let target = r.trim().to_lowercase();
        catalog.retain(|p| p.rarity.to_lowercase() == target);
    }
    if let Some(owned_only) = query.owned {
        if owned_only {
            catalog.retain(|p| owned_map.get(&p.id).copied().unwrap_or(0) > 0);
        } else {
            catalog.retain(|p| owned_map.get(&p.id).copied().unwrap_or(0) == 0);
        }
    }

    let sort_mode = query.sort.as_deref().unwrap_or("id");
    match sort_mode {
        "id_desc" => catalog.sort_by(|a, b| b.id.cmp(&a.id)),
        "rarity" => {
            let rarity_rank = |r: &str| match r.to_lowercase().as_str() {
                "mythical" => 7,
                "legendary" => 6,
                "epic" => 5,
                "ultrarare" => 4,
                "rare" => 3,
                "uncommon" => 2,
                _ => 1,
            };
            catalog.sort_by(|a, b| rarity_rank(&b.rarity).cmp(&rarity_rank(&a.rarity)));
        }
        "bst" => catalog.sort_by(|a, b| b.stats.total.cmp(&a.stats.total)),
        "atk" => catalog.sort_by(|a, b| b.stats.atk.cmp(&a.stats.atk)),
        "def" => catalog.sort_by(|a, b| b.stats.def.cmp(&a.stats.def)),
        "spe" => catalog.sort_by(|a, b| b.stats.spe.cmp(&a.stats.spe)),
        "hp" => catalog.sort_by(|a, b| b.stats.hp.cmp(&a.stats.hp)),
        "name" => catalog.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        _ => catalog.sort_by_key(|p| p.id),
    }

    let total = catalog.len();
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(50).max(1);
    let total_pages = (total + limit - 1) / limit;

    let start = (page - 1) * limit;
    let paginated: Vec<serde_json::Value> = catalog
        .into_iter()
        .skip(start)
        .take(limit)
        .map(|p| {
            let owned_count = owned_map.get(&p.id).copied().unwrap_or(0);
            let gen = get_generation(p.id);
            json!({
                "pokemon": p,
                "generation": gen,
                "owned": owned_count > 0,
                "count": owned_count,
            })
        })
        .collect();

    Json(json!({
        "total": total,
        "page": page,
        "limit": limit,
        "total_pages": total_pages,
        "items": paginated,
    }))
}

async fn get_pokemon_by_id(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(pokemon) = state.service.storage.pokemon_catalog.get(&id) {
        let dex = state.service.get_pokedex(&trainer.username);
        let owned_instances: Vec<&CaughtPokemon> = dex.iter().filter(|p| p.id == id).collect();
        let cry_url = format!("https://raw.githubusercontent.com/PokeAPI/cries/main/cries/pokemon/latest/{}.ogg", id);
        let art_url = format!("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/{}.png", id);
        let showdown_url = format!("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/showdown/{}.gif", id);
        let moves = crate::game::generate_moves_for_pokemon(pokemon);

        Ok(Json(json!({
            "pokemon": pokemon,
            "generation": get_generation(id),
            "owned_count": owned_instances.len(),
            "caught_history": owned_instances,
            "moves": moves,
            "cry_url": cry_url,
            "art_url": art_url,
            "showdown_url": showdown_url,
        })))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Pokemon #{} not found", id) })),
        ))
    }
}

async fn get_store(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let items = state.service.get_store_catalog(&trainer.username);
    let player = state.service.get_player(&trainer.username);
    let level = get_player_level(player.xp);
    let discount = get_level_discount(level);

    Json(json!({
        "currency": player.currency,
        "level": level,
        "discount_pct": (discount * 100.0).round() as u32,
        "items": items,
        "sell_prices": {
            "Common": 150,
            "Uncommon": 300,
            "Rare": 800,
            "UltraRare": 1500,
            "Epic": 3000,
            "Legendary": 10000,
            "Mythical": 25000,
        }
    }))
}

async fn post_store_buy(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
    Json(payload): Json<BuyPayload>,
) -> Result<Json<BuyResult>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.buy(&trainer.username, &payload.item, payload.amount) {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn post_store_sell(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
    Json(payload): Json<SellPayload>,
) -> Result<Json<crate::models::SellResult>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.sell_one(&trainer.username, &payload.name) {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn post_store_sell_all(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
    Json(payload): Json<SellAllPayload>,
) -> Result<Json<crate::models::SellResult>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.sell_all_by_rarity(&trainer.username, &payload.rarity) {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn post_store_sell_dupes(
    trainer: AuthenticatedTrainer,
    State(state): State<AppState>,
) -> Result<Json<crate::models::SellResult>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.sell_duplicates(&trainer.username) {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}
