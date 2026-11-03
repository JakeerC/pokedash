use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::models::{BuyResult, CatchAttemptResult, CaughtPokemon};
use crate::game::{
    calculate_catch_chance, get_generation, get_level_discount, get_level_xp_range,
    get_player_level,
};
use crate::service::GameService;

#[derive(Clone)]
pub struct AppState {
    pub service: GameService,
}

#[derive(Debug, Deserialize)]
pub struct CatchPayload {
    pub ball: String,
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
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        .route("/status", get(get_status))
        .route("/player", get(get_player))
        .route("/player/reset", post(reset_player))
        .route("/hunt", post(post_hunt))
        .route("/wild", get(get_wild))
        .route("/catch", post(post_catch))
        .route("/flee", post(post_flee))
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
        .nest_service("/sprites", ServeDir::new("static/sprites"))
        .fallback(serve_frontend)
        .layer(cors)
        .with_state(state)
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

async fn get_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let player = state.service.get_player();
    let level = get_player_level(player.xp);
    Json(json!({
        "status": "online",
        "version": "0.1.0",
        "username": player.username,
        "level": level,
        "currency": player.currency,
        "total_pokemon": 1025,
    }))
}

async fn get_player(State(state): State<AppState>) -> Json<serde_json::Value> {
    let player = state.service.get_player();
    let level = get_player_level(player.xp);
    let (prev_xp, next_xp) = get_level_xp_range(level);
    let discount = get_level_discount(level);
    let pokedex = state.service.get_pokedex();

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

async fn reset_player(State(state): State<AppState>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    state.service.storage.reset_game().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;
    Ok(Json(json!({ "message": "Game data reset to fresh state" })))
}

async fn post_hunt(State(state): State<AppState>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.hunt() {
        Ok(encounter) => {
            let player = state.service.get_player();
            let level = get_player_level(player.xp);
            let dex = state.service.get_pokedex();
            let already_caught = dex.iter().any(|p| p.id == encounter.pokemon.id);

            // Calculate odds for all 11 balls
            let mut odds = HashMap::new();
            for ball in crate::game::STORE_BALLS {
                let chance = calculate_catch_chance(&encounter.pokemon, ball.id, level, already_caught);
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
            })))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn get_wild(State(state): State<AppState>) -> Json<serde_json::Value> {
    if let Some(wild) = state.service.get_wild() {
        let player = state.service.get_player();
        let level = get_player_level(player.xp);
        let dex = state.service.get_pokedex();
        let already_caught = dex.iter().any(|p| p.id == wild.pokemon.id);

        let mut odds = HashMap::new();
        for ball in crate::game::STORE_BALLS {
            let chance = calculate_catch_chance(&wild.pokemon, ball.id, level, already_caught);
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
        }))
    } else {
        Json(json!({
            "has_wild": false,
            "encounter": null,
        }))
    }
}

async fn post_catch(
    State(state): State<AppState>,
    Json(payload): Json<CatchPayload>,
) -> Result<Json<CatchAttemptResult>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.catch(&payload.ball) {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn post_flee(State(state): State<AppState>) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.flee() {
        Ok(msg) => Ok(Json(json!({ "message": msg }))),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn get_pokedex(
    State(state): State<AppState>,
    Query(query): Query<PokedexQuery>,
) -> Json<serde_json::Value> {
    let mut dex = state.service.get_pokedex();

    // Filtering
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

    // Sorting
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
    State(state): State<AppState>,
    Query(query): Query<PokedexQuery>,
) -> Json<serde_json::Value> {
    let mut catalog = state.service.storage.pokemon_list.clone();
    let dex = state.service.get_pokedex();

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
            json!({
                "pokemon": p,
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
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(pokemon) = state.service.storage.pokemon_catalog.get(&id) {
        let dex = state.service.get_pokedex();
        let owned_instances: Vec<&CaughtPokemon> = dex.iter().filter(|p| p.id == id).collect();
        let cry_url = format!("https://raw.githubusercontent.com/PokeAPI/cries/main/cries/pokemon/latest/{}.ogg", id);
        let art_url = format!("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/{}.png", id);
        let showdown_url = format!("https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/showdown/{}.gif", id);

        Ok(Json(json!({
            "pokemon": pokemon,
            "generation": get_generation(id),
            "owned_count": owned_instances.len(),
            "caught_history": owned_instances,
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

async fn get_store(State(state): State<AppState>) -> Json<serde_json::Value> {
    let items = state.service.get_store_catalog();
    let player = state.service.get_player();
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
    State(state): State<AppState>,
    Json(payload): Json<BuyPayload>,
) -> Result<Json<BuyResult>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.buy(&payload.item, payload.amount) {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn post_store_sell(
    State(state): State<AppState>,
    Json(payload): Json<SellPayload>,
) -> Result<Json<crate::models::SellResult>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.sell_one(&payload.name) {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn post_store_sell_all(
    State(state): State<AppState>,
    Json(payload): Json<SellAllPayload>,
) -> Result<Json<crate::models::SellResult>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.sell_all_by_rarity(&payload.rarity) {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}

async fn post_store_sell_dupes(
    State(state): State<AppState>,
) -> Result<Json<crate::models::SellResult>, (StatusCode, Json<serde_json::Value>)> {
    match state.service.sell_duplicates() {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e })),
        )),
    }
}
