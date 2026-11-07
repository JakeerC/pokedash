use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use serde::Deserialize;

use crate::models::{ActiveArenaBattle, CaughtPokemon, PlayerData, Pokemon, PokemonStats, WildPokemonEncounter};
use crate::game::{create_battle_pokemon, get_player_level};
use crate::supabase::{SupabaseClient, SupabaseConfig};

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
    pub supabase: Option<SupabaseClient>,
}

impl Storage {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let save_dir = home.join(".pokedash");
        let _ = fs::create_dir_all(&save_dir);

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

        // Initialize Supabase client
        let supabase_config = SupabaseConfig::from_env_or_default();
        let supabase = match SupabaseClient::new(supabase_config) {
            Ok(client) => {
                println!("🔌 Connected to Supabase DB at {}", client.config.url);
                Some(client)
            }
            Err(e) => {
                eprintln!("⚠️ Could not initialize Supabase client: {}. Falling back to local file storage.", e);
                None
            }
        };

        let storage = Self {
            save_dir,
            player_file,
            pokedex_file,
            wild_file,
            arena_file,
            pokemon_catalog,
            pokemon_list,
            rarity_pools,
            supabase,
        };

        // Bootstrap or verify initial player setup
        let _ = storage.load_player();

        Ok(storage)
    }

    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| handle.block_on(f)),
            Err(_) => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build tokio runtime");
                rt.block_on(f)
            }
        }
    }

    pub fn load_player(&self) -> PlayerData {
        let mut player = if let Some(supabase) = &self.supabase {
            match Self::block_on(supabase.load_player()) {
                Ok(Some(player)) => player,
                Ok(None) => {
                    // Try to migrate from local file if available
                    let local = self.load_player_from_file();
                    if local.currency > 0 || local.xp > 0 || !local.party.is_empty() {
                        println!("⚡ Migrating local player data to Supabase...");
                        let _ = Self::block_on(supabase.save_player(&local));
                        local
                    } else {
                        let default_player = PlayerData::default();
                        let _ = Self::block_on(supabase.save_player(&default_player));
                        default_player
                    }
                }
                Err(e) => {
                    eprintln!("[Supabase] Failed to load player: {}. Falling back to local file.", e);
                    self.load_player_from_file()
                }
            }
        } else {
            self.load_player_from_file()
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
        if let Some(supabase) = &self.supabase {
            if let Err(e) = Self::block_on(supabase.save_player(player)) {
                eprintln!("[Supabase] Error saving player: {}", e);
            }
        }
        self.save_player_to_file(player)?;
        Ok(())
    }

    fn load_player_from_file(&self) -> PlayerData {
        if self.player_file.exists() {
            if let Ok(content) = fs::read_to_string(&self.player_file) {
                if let Ok(data) = serde_json::from_str::<PlayerData>(&content) {
                    return data;
                }
            }
        }
        PlayerData::default()
    }

    fn save_player_to_file(&self, player: &PlayerData) -> std::io::Result<()> {
        let temp_file = self.save_dir.join("player.json.tmp");
        let json = serde_json::to_string_pretty(player)?;
        fs::write(&temp_file, json)?;
        fs::rename(temp_file, &self.player_file)?;
        Ok(())
    }

    pub fn load_pokedex(&self) -> Vec<CaughtPokemon> {
        if let Some(supabase) = &self.supabase {
            match Self::block_on(supabase.load_pokedex()) {
                Ok(dex) if !dex.is_empty() => return dex,
                Ok(_) => {
                    let local = self.load_pokedex_from_file();
                    if !local.is_empty() {
                        println!("⚡ Migrating local Pokédex to Supabase ({} entries)...", local.len());
                        let _ = Self::block_on(supabase.save_pokedex(&local));
                        return local;
                    }
                    return Vec::new();
                }
                Err(e) => {
                    eprintln!("[Supabase] Error loading pokedex: {}. Falling back to local file.", e);
                    return self.load_pokedex_from_file();
                }
            }
        }
        self.load_pokedex_from_file()
    }

    pub fn save_pokedex(&self, dex: &[CaughtPokemon]) -> std::io::Result<()> {
        if let Some(supabase) = &self.supabase {
            if let Err(e) = Self::block_on(supabase.save_pokedex(dex)) {
                eprintln!("[Supabase] Error saving pokedex: {}", e);
            }
        }
        self.save_pokedex_to_file(dex)?;
        Ok(())
    }

    fn load_pokedex_from_file(&self) -> Vec<CaughtPokemon> {
        if self.pokedex_file.exists() {
            if let Ok(content) = fs::read_to_string(&self.pokedex_file) {
                if let Ok(dex) = serde_json::from_str::<Vec<CaughtPokemon>>(&content) {
                    return dex;
                }
            }
        }
        Vec::new()
    }

    fn save_pokedex_to_file(&self, dex: &[CaughtPokemon]) -> std::io::Result<()> {
        let temp_file = self.save_dir.join("pokedex.json.tmp");
        let json = serde_json::to_string_pretty(dex)?;
        fs::write(&temp_file, json)?;
        fs::rename(temp_file, &self.pokedex_file)?;
        Ok(())
    }

    pub fn load_wild_encounter(&self) -> Option<WildPokemonEncounter> {
        if let Some(supabase) = &self.supabase {
            match Self::block_on(supabase.load_wild_encounter()) {
                Ok(wild) => return wild,
                Err(e) => {
                    eprintln!("[Supabase] Error loading wild encounter: {}. Falling back to local file.", e);
                }
            }
        }
        self.load_wild_encounter_from_file()
    }

    pub fn save_wild_encounter(&self, wild: &WildPokemonEncounter) -> std::io::Result<()> {
        if let Some(supabase) = &self.supabase {
            if let Err(e) = Self::block_on(supabase.save_wild_encounter(wild)) {
                eprintln!("[Supabase] Error saving wild encounter: {}", e);
            }
        }
        self.save_wild_encounter_to_file(wild)?;
        Ok(())
    }

    fn load_wild_encounter_from_file(&self) -> Option<WildPokemonEncounter> {
        if self.wild_file.exists() {
            if let Ok(content) = fs::read_to_string(&self.wild_file) {
                if let Ok(wild) = serde_json::from_str::<WildPokemonEncounter>(&content) {
                    return Some(wild);
                }
            }
        }
        None
    }

    fn save_wild_encounter_to_file(&self, wild: &WildPokemonEncounter) -> std::io::Result<()> {
        let temp_file = self.save_dir.join("wild_pokemon.json.tmp");
        let json = serde_json::to_string_pretty(wild)?;
        fs::write(&temp_file, json)?;
        fs::rename(temp_file, &self.wild_file)?;
        Ok(())
    }

    pub fn clear_wild_encounter(&self) {
        if let Some(supabase) = &self.supabase {
            if let Err(e) = Self::block_on(supabase.clear_wild_encounter()) {
                eprintln!("[Supabase] Error clearing wild encounter: {}", e);
            }
        }
        let _ = fs::remove_file(&self.wild_file);
    }

    pub fn load_arena_battle(&self) -> Option<ActiveArenaBattle> {
        if let Some(supabase) = &self.supabase {
            match Self::block_on(supabase.load_arena_battle()) {
                Ok(battle) => return battle,
                Err(e) => {
                    eprintln!("[Supabase] Error loading arena battle: {}. Falling back to local file.", e);
                }
            }
        }
        self.load_arena_battle_from_file()
    }

    pub fn save_arena_battle(&self, battle: &ActiveArenaBattle) -> std::io::Result<()> {
        if let Some(supabase) = &self.supabase {
            if let Err(e) = Self::block_on(supabase.save_arena_battle(battle)) {
                eprintln!("[Supabase] Error saving arena battle: {}", e);
            }
        }
        self.save_arena_battle_to_file(battle)?;
        Ok(())
    }

    fn load_arena_battle_from_file(&self) -> Option<ActiveArenaBattle> {
        if self.arena_file.exists() {
            if let Ok(content) = fs::read_to_string(&self.arena_file) {
                if let Ok(battle) = serde_json::from_str::<ActiveArenaBattle>(&content) {
                    return Some(battle);
                }
            }
        }
        None
    }

    fn save_arena_battle_to_file(&self, battle: &ActiveArenaBattle) -> std::io::Result<()> {
        let temp_file = self.save_dir.join("arena_battle.json.tmp");
        let json = serde_json::to_string_pretty(battle)?;
        fs::write(&temp_file, json)?;
        fs::rename(temp_file, &self.arena_file)?;
        Ok(())
    }

    pub fn clear_arena_battle(&self) {
        if let Some(supabase) = &self.supabase {
            if let Err(e) = Self::block_on(supabase.clear_arena_battle()) {
                eprintln!("[Supabase] Error clearing arena battle: {}", e);
            }
        }
        let _ = fs::remove_file(&self.arena_file);
    }

    pub fn reset_game(&self) -> std::io::Result<()> {
        if let Some(supabase) = &self.supabase {
            if let Err(e) = Self::block_on(supabase.reset_game()) {
                eprintln!("[Supabase] Error resetting game: {}", e);
            }
        }
        let default_player = PlayerData::default();
        self.save_player_to_file(&default_player)?;
        self.save_pokedex_to_file(&[])?;
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
