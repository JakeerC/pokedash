use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
pub struct BattleMove {
    pub name: String,
    pub r#type: String,
    pub power: u32,
    pub accuracy: u32,
    pub category: String, // "physical" or "special"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattlePokemon {
    pub id: u32,
    pub name: String,
    pub level: u32,
    pub current_hp: u32,
    pub max_hp: u32,
    pub stats: PokemonStats,
    pub types: Vec<String>,
    pub abilities: Vec<String>,
    pub moves: Vec<BattleMove>,
    pub rarity: String,
    #[serde(default)]
    pub sprite_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    #[serde(default)]
    pub total_hunts: u32,
    #[serde(default)]
    pub successful_catches: u32,
    #[serde(default)]
    pub failed_catches: u32,
    #[serde(default)]
    pub first_time_catches: u32,
    #[serde(default)]
    pub xp_from_hunting: u32,
    #[serde(default)]
    pub xp_from_catching: u32,
    #[serde(default)]
    pub total_lifetime_caught: u32,
    #[serde(default)]
    pub lifetime_rarity: HashMap<String, u32>,
    #[serde(default)]
    pub lifetime_gens: HashMap<String, u32>,
    #[serde(default)]
    pub lifetime_unique_pids: Vec<u32>,
    #[serde(default)]
    pub arena_battles: u32,
    #[serde(default)]
    pub arena_victories: u32,
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
            arena_battles: 0,
            arena_victories: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerData {
    pub username: String,
    #[serde(default)]
    pub currency: u64,
    #[serde(default)]
    pub xp: u64,
    #[serde(default)]
    pub balls: HashMap<String, u32>,
    #[serde(default)]
    pub last_hunt_time: f64,
    #[serde(default)]
    pub stats: PlayerStats,
    #[serde(default)]
    pub active_pokemon_id: Option<u32>,
    #[serde(default)]
    pub party: Vec<BattlePokemon>,
    #[serde(default)]
    pub arena_badges: Vec<String>,
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
            active_pokemon_id: Some(25),
            party: vec![
                BattlePokemon {
                    id: 25,
                    name: "Pikachu".to_string(),
                    level: 5,
                    current_hp: 20,
                    max_hp: 20,
                    stats: PokemonStats {
                        hp: 35,
                        atk: 55,
                        def: 40,
                        spa: 50,
                        spd: 50,
                        spe: 90,
                        total: 320,
                    },
                    types: vec!["Electric".to_string()],
                    abilities: vec!["Static".to_string(), "Lightning Rod".to_string()],
                    moves: vec![
                        BattleMove { name: "Tackle".to_string(), r#type: "Normal".to_string(), power: 40, accuracy: 100, category: "physical".to_string() },
                        BattleMove { name: "Thunderbolt".to_string(), r#type: "Electric".to_string(), power: 80, accuracy: 100, category: "special".to_string() },
                        BattleMove { name: "Slash".to_string(), r#type: "Normal".to_string(), power: 70, accuracy: 100, category: "physical".to_string() },
                        BattleMove { name: "Giga Impact".to_string(), r#type: "Normal".to_string(), power: 110, accuracy: 90, category: "physical".to_string() },
                    ],
                    rarity: "Common".to_string(),
                    sprite_url: "/sprites/25.png".to_string(),
                }
            ],
            arena_badges: Vec::new(),
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
    #[serde(default)]
    pub current_hp: u32,
    #[serde(default)]
    pub max_hp: u32,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub moves: Vec<BattleMove>,
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

// Arena Models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArenaDivision {
    pub id: String,
    pub name: String,
    pub leader_name: String,
    pub title: String,
    pub badge: String,
    pub badge_icon: String,
    pub recommended_level: u32,
    pub reward_money: u64,
    pub reward_xp: u32,
    pub team_preview: Vec<u32>,
    pub completed: bool,
    pub dialogue_intro: String,
    pub dialogue_defeat: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveArenaBattle {
    pub arena_id: String,
    pub leader_name: String,
    pub leader_title: String,
    pub current_enemy_index: usize,
    pub enemy_team: Vec<BattlePokemon>,
    pub player_active: BattlePokemon,
    pub log: Vec<String>,
    pub is_over: bool,
    pub player_won: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BattleActionResult {
    pub player_move_name: String,
    #[serde(default)]
    pub player_move_type: Option<String>,
    pub player_damage: u32,
    pub player_crit: bool,
    pub player_effectiveness: f64,
    pub enemy_move_name: Option<String>,
    #[serde(default)]
    pub enemy_move_type: Option<String>,
    pub enemy_damage: u32,
    pub enemy_crit: bool,
    pub enemy_effectiveness: f64,
    pub enemy_current_hp: u32,
    pub enemy_max_hp: u32,
    pub player_current_hp: u32,
    pub player_max_hp: u32,
    pub enemy_fainted: bool,
    pub player_fainted: bool,
    pub battle_ended: bool,
    pub player_won: bool,
    pub reward_money: u64,
    pub reward_xp: u32,
    pub badge_earned: Option<String>,
    pub message: String,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: AuthUser,
    #[serde(default)]
    pub requires_verification: bool,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: usize,
    pub username: String,
    pub level: u32,
    pub xp: u64,
    pub currency: u64,
    pub badges_count: usize,
    pub unique_caught: usize,
    #[serde(default)]
    pub lead_pokemon_id: Option<u32>,
    #[serde(default)]
    pub lead_pokemon_name: Option<String>,
    #[serde(default)]
    pub lead_pokemon_level: Option<u32>,
    #[serde(default)]
    pub lead_pokemon_sprite: Option<String>,
}
