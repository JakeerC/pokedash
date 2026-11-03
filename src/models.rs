use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PokemonStats {
    #[serde(rename = "HP")]
    pub hp: u32,
    #[serde(rename = "Atk")]
    pub atk: u32,
    #[serde(rename = "Def")]
    pub def: u32,
    #[serde(rename = "SpA")]
    pub spa: u32,
    #[serde(rename = "SpD")]
    pub spd: u32,
    #[serde(rename = "Spe")]
    pub spe: u32,
    #[serde(rename = "Total")]
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pokemon {
    #[serde(default)]
    pub id: u32,
    pub name: String,
    pub types: Vec<String>,
    pub abilities: Vec<String>,
    pub stats: PokemonStats,
    pub rarity: String,
    #[serde(default)]
    pub rarity_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    pub total_hunts: u32,
    pub successful_catches: u32,
    pub failed_catches: u32,
    pub first_time_catches: u32,
    pub xp_from_hunting: u32,
    pub xp_from_catching: u32,
    pub total_lifetime_caught: u32,
    pub lifetime_rarity: HashMap<String, u32>,
    pub lifetime_gens: HashMap<String, u32>,
    pub lifetime_unique_pids: Vec<u32>,
}

impl Default for PlayerStats {
    fn default() -> Self {
        let mut lifetime_rarity = HashMap::new();
        for r in &["common", "uncommon", "rare", "ultrarare", "epic", "legendary", "mythical"] {
            lifetime_rarity.insert(r.to_string(), 0);
        }
        let mut lifetime_gens = HashMap::new();
        for i in 1..=9 {
            lifetime_gens.insert(i.to_string(), 0);
        }

        Self {
            total_hunts: 0,
            successful_catches: 0,
            failed_catches: 0,
            first_time_catches: 0,
            xp_from_hunting: 0,
            xp_from_catching: 0,
            total_lifetime_caught: 0,
            lifetime_rarity,
            lifetime_gens,
            lifetime_unique_pids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerData {
    pub username: String,
    pub currency: u64,
    pub xp: u64,
    pub balls: HashMap<String, u32>,
    pub last_hunt_time: f64,
    pub stats: PlayerStats,
}

impl Default for PlayerData {
    fn default() -> Self {
        let mut balls = HashMap::new();
        balls.insert("poke_ball".to_string(), 50);
        balls.insert("great_ball".to_string(), 0);
        balls.insert("ultra_ball".to_string(), 0);
        balls.insert("master_ball".to_string(), 0);
        balls.insert("net_ball".to_string(), 0);
        balls.insert("dive_ball".to_string(), 0);
        balls.insert("fast_ball".to_string(), 0);
        balls.insert("dusk_ball".to_string(), 0);
        balls.insert("nest_ball".to_string(), 0);
        balls.insert("repeat_ball".to_string(), 0);
        balls.insert("quick_ball".to_string(), 0);

        Self {
            username: whoami(),
            currency: 0,
            xp: 0,
            balls,
            last_hunt_time: 0.0,
            stats: PlayerStats::default(),
        }
    }
}

fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "Trainer".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaughtPokemon {
    pub id: u32,
    pub name: String,
    pub types: Vec<String>,
    pub abilities: Vec<String>,
    pub stats: PokemonStats,
    pub rarity: String,
    pub rarity_score: f64,
    #[serde(default)]
    pub caught_at: f64,
    #[serde(default)]
    pub ball_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WildPokemonEncounter {
    pub pokemon: Pokemon,
    pub encountered_at: f64,
    pub cooldown_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatchAttemptResult {
    pub success: bool,
    pub pokemon: Pokemon,
    pub ball: String,
    pub catch_chance: f64,
    pub xp_gained: u32,
    pub is_new: bool,
    pub leveled_up: bool,
    pub new_level: u32,
    pub message: String,
    pub balls_remaining: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreItemInfo {
    pub id: String,
    pub name: String,
    pub price: u64,
    pub discounted_price: u64,
    pub category: String,
    pub unlock_level: u32,
    pub owned: u32,
    pub description: String,
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SellResult {
    pub sold_count: usize,
    pub total_earnings: u64,
    pub new_balance: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuyResult {
    pub item: String,
    pub amount: u32,
    pub total_cost: u64,
    pub new_balance: u64,
    pub new_owned_count: u32,
    pub message: String,
}
