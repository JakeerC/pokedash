use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use serde::Deserialize;

use crate::models::{ActiveArenaBattle, CaughtPokemon, PlayerData, Pokemon, PokemonStats, WildPokemonEncounter};
use crate::game::{create_battle_pokemon, get_player_level};

#[derive(Debug, Deserialize)]
struct PokemonJsonEntry {
    pub name: String,
    pub types: Vec<String>,
    pub abilities: Vec<String>,
    pub stats: PokemonStats,
    pub rarity: String,
    #[serde(default)]
    pub rarity_score: f64,
}

pub struct Storage {
    pub save_dir: PathBuf,
    pub player_file: PathBuf,
    pub pokedex_file: PathBuf,
    pub wild_file: PathBuf,
    pub arena_file: PathBuf,
    pub pokemon_catalog: HashMap<u32, Pokemon>,
    pub pokemon_list: Vec<Pokemon>,
    pub rarity_pools: HashMap<String, Vec<u32>>,
}

impl Storage {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let save_dir = home.join(".pokedash");
        fs::create_dir_all(&save_dir)?;

        let player_file = save_dir.join("player.json");
        let pokedex_file = save_dir.join("pokedex.json");
        let wild_file = save_dir.join("wild_pokemon.json");
        let arena_file = save_dir.join("arena_battle.json");

        // Load pokemon_data.json
        let candidates = [
            PathBuf::from("data/pokemon_data.json"),
            PathBuf::from("/Users/jakeerchilakala/orca/projects/pokedash/data/pokemon_data.json"),
            PathBuf::from("/Users/jakeerchilakala/Projects/pokecatch_cli_game-main/pokecatch/data/pokemon_data.json"),
        ];

        let mut data_path = None;
        for c in &candidates {
            if c.exists() {
                data_path = Some(c.clone());
                break;
            }
        }

        let data_path = data_path.ok_or("Cannot find data/pokemon_data.json")?;
        let content = fs::read_to_string(&data_path)?;
        let raw_map: HashMap<String, PokemonJsonEntry> = serde_json::from_str(&content)?;

        let mut pokemon_catalog = HashMap::new();
        let mut pokemon_list = Vec::new();
        let mut rarity_pools: HashMap<String, Vec<u32>> = HashMap::new();

        for (id_str, raw) in raw_map {
            if let Ok(id) = id_str.parse::<u32>() {
                let p = Pokemon {
                    id,
                    name: raw.name,
                    types: raw.types,
                    abilities: raw.abilities,
                    stats: raw.stats,
                    rarity: raw.rarity,
                    rarity_score: raw.rarity_score,
                };
                rarity_pools
                    .entry(p.rarity.to_lowercase())
                    .or_default()
                    .push(id);
                pokemon_catalog.insert(id, p.clone());
                pokemon_list.push(p);
            }
        }

        pokemon_list.sort_by_key(|p| p.id);

        Ok(Self {
            save_dir,
            player_file,
            pokedex_file,
            wild_file,
            arena_file,
            pokemon_catalog,
            pokemon_list,
            rarity_pools,
        })
    }

    pub fn load_player(&self) -> PlayerData {
        let mut player = if self.player_file.exists() {
            if let Ok(content) = fs::read_to_string(&self.player_file) {
                if let Ok(data) = serde_json::from_str::<PlayerData>(&content) {
                    data
                } else {
                    PlayerData::default()
                }
            } else {
                PlayerData::default()
            }
        } else {
            PlayerData::default()
        };

        // Ensure player has a party if empty
        if player.party.is_empty() {
            let level = get_player_level(player.xp).max(5);
            let dex = self.load_pokedex();
            if let Some(first_caught) = dex.first() {
                if let Some(proto) = self.pokemon_catalog.get(&first_caught.id) {
                    player.party.push(create_battle_pokemon(proto, level));
                    player.active_pokemon_id = Some(first_caught.id);
                }
            } else if let Some(starter) = self.pokemon_catalog.get(&25) { // Pikachu starter
                player.party.push(create_battle_pokemon(starter, level));
                player.active_pokemon_id = Some(25);
            }
            let _ = self.save_player(&player);
        }

        player
    }

    pub fn save_player(&self, player: &PlayerData) -> std::io::Result<()> {
        let temp_file = self.save_dir.join("player.json.tmp");
        let json = serde_json::to_string_pretty(player)?;
        fs::write(&temp_file, json)?;
        fs::rename(temp_file, &self.player_file)?;
        Ok(())
    }

    pub fn load_pokedex(&self) -> Vec<CaughtPokemon> {
        if self.pokedex_file.exists() {
            if let Ok(content) = fs::read_to_string(&self.pokedex_file) {
                if let Ok(dex) = serde_json::from_str::<Vec<CaughtPokemon>>(&content) {
                    return dex;
                }
            }
        }
        Vec::new()
    }

    pub fn save_pokedex(&self, dex: &[CaughtPokemon]) -> std::io::Result<()> {
        let temp_file = self.save_dir.join("pokedex.json.tmp");
        let json = serde_json::to_string_pretty(dex)?;
        fs::write(&temp_file, json)?;
        fs::rename(temp_file, &self.pokedex_file)?;
        Ok(())
    }

    pub fn load_wild_encounter(&self) -> Option<WildPokemonEncounter> {
        if self.wild_file.exists() {
            if let Ok(content) = fs::read_to_string(&self.wild_file) {
                if let Ok(wild) = serde_json::from_str::<WildPokemonEncounter>(&content) {
                    return Some(wild);
                }
            }
        }
        None
    }

    pub fn save_wild_encounter(&self, wild: &WildPokemonEncounter) -> std::io::Result<()> {
        let temp_file = self.save_dir.join("wild_pokemon.json.tmp");
        let json = serde_json::to_string_pretty(wild)?;
        fs::write(&temp_file, json)?;
        fs::rename(temp_file, &self.wild_file)?;
        Ok(())
    }

    pub fn clear_wild_encounter(&self) {
        let _ = fs::remove_file(&self.wild_file);
    }

    pub fn load_arena_battle(&self) -> Option<ActiveArenaBattle> {
        if self.arena_file.exists() {
            if let Ok(content) = fs::read_to_string(&self.arena_file) {
                if let Ok(battle) = serde_json::from_str::<ActiveArenaBattle>(&content) {
                    return Some(battle);
                }
            }
        }
        None
    }

    pub fn save_arena_battle(&self, battle: &ActiveArenaBattle) -> std::io::Result<()> {
        let temp_file = self.save_dir.join("arena_battle.json.tmp");
        let json = serde_json::to_string_pretty(battle)?;
        fs::write(&temp_file, json)?;
        fs::rename(temp_file, &self.arena_file)?;
        Ok(())
    }

    pub fn clear_arena_battle(&self) {
        let _ = fs::remove_file(&self.arena_file);
    }

    pub fn reset_game(&self) -> std::io::Result<()> {
        let default_player = PlayerData::default();
        self.save_player(&default_player)?;
        self.save_pokedex(&[])?;
        self.clear_wild_encounter();
        self.clear_arena_battle();
        Ok(())
    }

    pub fn now_sec() -> f64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }
}
