use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use serde::Deserialize;

use crate::game::{create_battle_pokemon, get_player_level};
use crate::models::{
    ActiveArenaBattle, AuthResponse, AuthUser, CaughtPokemon, GuestSession, LeaderboardEntry, PlayerData, Pokemon,
    PokemonStats, WildPokemonEncounter,
};
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
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub weight: Option<u32>,
}

pub struct Storage {
    pub save_dir: PathBuf,
    pub pokemon_catalog: HashMap<u32, Pokemon>,
    pub pokemon_list: Vec<Pokemon>,
    pub rarity_pools: HashMap<String, Vec<u32>>,
    pub supabase: Option<SupabaseClient>,
    pub guest_sessions: Arc<RwLock<HashMap<String, GuestSession>>>,
}

impl Storage {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let save_dir = home.join(".pokedash");
        let _ = fs::create_dir_all(&save_dir);

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
                    sprite_url: Some(format!("/sprites/{}.png", id)),
                    height: raw.height,
                    weight: raw.weight,
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

        // Load guest sessions
        let guest_sessions_map = Self::load_guest_sessions(&save_dir);
        let guest_sessions = Arc::new(RwLock::new(guest_sessions_map));

        // Initialize Supabase client
        let supabase_config = SupabaseConfig::from_env_or_default();
        let supabase = match SupabaseClient::new(supabase_config, guest_sessions.clone()) {
            Ok(client) => {
                println!("🔌 Connected to Supabase DB at {}", client.config.url);
                Some(client)
            }
            Err(e) => {
                eprintln!("⚠️ Could not initialize Supabase client: {}. Falling back to local file storage.", e);
                None
            }
        };

        println!("📦 Loaded {} Pokémon from local static data file", pokemon_list.len());

        Ok(Self {
            save_dir,
            pokemon_catalog,
            pokemon_list,
            rarity_pools,
            supabase,
            guest_sessions,
        })
    }

    fn safe_username(username: &str) -> String {
        let safe: String = username
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if safe.is_empty() {
            "Trainer".to_string()
        } else {
            safe
        }
    }

    fn player_file(&self, username: &str) -> PathBuf {
        let safe = Self::safe_username(username);
        let named = self.save_dir.join(format!("player_{}.json", safe));
        if !named.exists() {
            let legacy = self.save_dir.join("player.json");
            if legacy.exists() {
                return legacy;
            }
        }
        named
    }

    fn pokedex_file(&self, username: &str) -> PathBuf {
        let safe = Self::safe_username(username);
        let named = self.save_dir.join(format!("pokedex_{}.json", safe));
        if !named.exists() {
            let legacy = self.save_dir.join("pokedex.json");
            if legacy.exists() {
                return legacy;
            }
        }
        named
    }

    fn wild_file(&self, username: &str) -> PathBuf {
        let safe = Self::safe_username(username);
        let named = self.save_dir.join(format!("wild_{}.json", safe));
        if !named.exists() {
            let legacy = self.save_dir.join("wild_pokemon.json");
            if legacy.exists() {
                return legacy;
            }
        }
        named
    }

    fn arena_file(&self, username: &str) -> PathBuf {
        let safe = Self::safe_username(username);
        let named = self.save_dir.join(format!("arena_{}.json", safe));
        if !named.exists() {
            let legacy = self.save_dir.join("arena_battle.json");
            if legacy.exists() {
                return legacy;
            }
        }
        named
    }

    fn block_on<F: std::future::Future>(f: F) -> F::Output
    where
        F: Send,
        F::Output: Send,
    {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => match handle.runtime_flavor() {
                tokio::runtime::RuntimeFlavor::MultiThread => {
                    tokio::task::block_in_place(|| handle.block_on(f))
                }
                _ => {
                    std::thread::scope(|s| {
                        s.spawn(|| {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .expect("Failed to build tokio runtime");
                            rt.block_on(f)
                        })
                        .join()
                        .expect("Blocking scoped helper thread panicked")
                    })
                }
            },
            Err(_) => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to build tokio runtime");
                rt.block_on(f)
            }
        }
    }

    fn load_guest_sessions(save_dir: &std::path::Path) -> HashMap<String, GuestSession> {
        let file_path = save_dir.join("guest_sessions.json");
        if file_path.exists() {
            if let Ok(content) = fs::read_to_string(&file_path) {
                if let Ok(sessions) = serde_json::from_str::<HashMap<String, GuestSession>>(&content) {
                    return sessions;
                }
            }
        }
        HashMap::new()
    }

    pub fn save_guest_sessions_to_file(&self) {
        let file_path = self.save_dir.join("guest_sessions.json");
        let sessions = self.guest_sessions.read().unwrap();
        if let Ok(json) = serde_json::to_string_pretty(&*sessions) {
            let _ = fs::write(file_path, json);
        }
    }

    pub fn verify_token(&self, token: &str) -> Result<AuthUser, Box<dyn std::error::Error + Send + Sync>> {
        if token.starts_with("guest_") {
            let sessions = self.guest_sessions.read().unwrap();
            if let Some(session) = sessions.get(token) {
                return Ok(session.user.clone());
            }
            return Err("Guest session expired or not found. Please log in as a guest again.".into());
        }

        let supabase = self
            .supabase
            .as_ref()
            .ok_or("Supabase client is not configured")?;
        Self::block_on(supabase.verify_token(token))
    }

    pub async fn create_guest_session(
        &self,
        character_req: Option<&str>,
    ) -> Result<AuthResponse, Box<dyn std::error::Error + Send + Sync>> {
        use rand::Rng;

        let char_list = crate::models::get_pokemon_characters();

        // 1. Pick character base name
        let base_name = if let Some(req) = character_req {
            let sanitized: String = req
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if sanitized.len() >= 2 && sanitized.len() <= 20 {
                sanitized
            } else {
                let idx = rand::thread_rng().gen_range(0..char_list.len());
                char_list[idx].username.clone()
            }
        } else {
            let idx = rand::thread_rng().gen_range(0..char_list.len());
            char_list[idx].username.clone()
        };

        // 2. Determine unique username:
        // Try candidate base_name. If taken by an existing account, append random 3 digits (e.g. Ash_482).
        let mut final_username = base_name.clone();
        let mut attempts = 0;

        loop {
            let taken = if let Some(supabase) = &self.supabase {
                supabase.is_username_taken(&final_username).await.unwrap_or(false)
            } else {
                let safe = Self::safe_username(&final_username);
                self.save_dir.join(format!("player_{}.json", safe)).exists()
            };

            if !taken || attempts >= 10 {
                break;
            }
            attempts += 1;
            let rand_num = rand::thread_rng().gen_range(100..999);
            final_username = format!("{}_{}", base_name, rand_num);
        }

        // 3. Generate secure random token
        let token_rand: String = (0..24)
            .map(|_| {
                let bytes = b"abcdefghijklmnopqrstuvwxyz0123456789";
                let idx = rand::thread_rng().gen_range(0..bytes.len());
                bytes[idx] as char
            })
            .collect();
        let token = format!("guest_{}_{}", final_username.to_lowercase(), token_rand);

        let user_id = format!("guest_{}", &token_rand[..8]);
        let user_email = format!("{}@guest.pokedash.local", final_username.to_lowercase());

        let auth_user = AuthUser {
            id: user_id,
            email: user_email,
            username: final_username.clone(),
        };

        let now_sec = chrono::Utc::now().timestamp() as u64;
        let session = GuestSession {
            token: token.clone(),
            user: auth_user.clone(),
            created_at: now_sec,
        };

        // 4. Store in memory
        {
            let mut sessions = self.guest_sessions.write().unwrap();
            sessions.insert(token.clone(), session);
        }

        // 5. Save sessions to disk
        self.save_guest_sessions_to_file();

        // 6. Resolve signature starter Pokémon for this character
        let starter_id = if let Some(char_info) = char_list.iter().find(|c| c.username.eq_ignore_ascii_case(&base_name)) {
            char_info.starter_id
        } else if let Some(proto) = self.pokemon_catalog.values().find(|p| p.name.eq_ignore_ascii_case(&base_name)) {
            proto.id
        } else {
            25 // Default to Pikachu
        };

        let starter_proto = self.pokemon_catalog.get(&starter_id).cloned().unwrap_or_else(|| {
            self.pokemon_catalog.get(&25).cloned().expect("Pikachu (#25) must exist")
        });

        // 7. Ensure player profile and signature starter exist in Supabase DB / local file
        let existing = if let Some(supabase) = &self.supabase {
            supabase.load_player(&final_username).await.unwrap_or(None)
        } else {
            None
        };

        if existing.is_none() {
            let battle_starter = create_battle_pokemon(&starter_proto, 5);
            let mut def_player = PlayerData::default();
            def_player.username = final_username.clone();
            def_player.active_pokemon_id = Some(starter_proto.id);
            def_player.party = vec![battle_starter];
            def_player.stats.lifetime_unique_pids = vec![starter_proto.id];
            def_player.stats.total_lifetime_caught = 1;
            *def_player.stats.lifetime_rarity.entry(starter_proto.rarity.to_lowercase()).or_default() += 1;

            let now_f64 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();

            let caught_entry = CaughtPokemon {
                id: starter_proto.id,
                name: starter_proto.name.clone(),
                types: starter_proto.types.clone(),
                abilities: starter_proto.abilities.clone(),
                stats: starter_proto.stats.clone(),
                rarity: starter_proto.rarity.clone(),
                rarity_score: starter_proto.rarity_score,
                caught_at: now_f64,
                ball_used: Some("poke_ball".to_string()),
            };

            // Save to Supabase Cloud DB
            if let Some(supabase) = &self.supabase {
                if let Err(e) = supabase.save_player(&def_player).await {
                    eprintln!("[Supabase] Failed to save guest player: {}", e);
                }
                if let Err(e) = supabase.save_pokedex(&final_username, &[caught_entry.clone()]).await {
                    eprintln!("[Supabase] Failed to save guest pokedex: {}", e);
                }
            }

            // Save to local storage cache
            let _ = self.save_player_to_file(&def_player);
            let _ = self.save_pokedex_to_file(&final_username, &[caught_entry]);
        }

        Ok(AuthResponse {
            token,
            user: auth_user,
            requires_verification: false,
            message: Some(format!(
                "Trainer Passport issued to Guest {} with partner {}! Adventure awaits!",
                final_username, starter_proto.name
            )),
        })
    }

    pub fn load_player(&self, username: &str) -> PlayerData {
        let mut player = if let Some(supabase) = &self.supabase {
            match Self::block_on(supabase.load_player(username)) {
                Ok(Some(player)) => player,
                Ok(None) => {
                    // Try to migrate from local file if available
                    let local = self.load_player_from_file(username);
                    if local.currency > 0 || local.xp > 0 || !local.party.is_empty() {
                        println!("⚡ Migrating local player data to Supabase for {}...", username);
                        let _ = Self::block_on(supabase.save_player(&local));
                        local
                    } else {
                        let mut default_player = PlayerData::default();
                        default_player.username = username.to_string();
                        let _ = Self::block_on(supabase.save_player(&default_player));
                        default_player
                    }
                }
                Err(e) => {
                    eprintln!("[Supabase] Failed to load player {}: {}. Falling back to local file.", username, e);
                    self.load_player_from_file(username)
                }
            }
        } else {
            self.load_player_from_file(username)
        };

        let mut modified = false;

        // Ensure player has a party if empty
        if player.party.is_empty() {
            let level = get_player_level(player.xp).max(5);
            let mut dex = self.load_pokedex(username);
            if let Some(first_caught) = dex.first() {
                if let Some(proto) = self.pokemon_catalog.get(&first_caught.id) {
                    player.party.push(create_battle_pokemon(proto, level));
                    player.active_pokemon_id = Some(first_caught.id);
                    modified = true;
                }
            } else if let Some(starter) = self.pokemon_catalog.get(&25) { // Pikachu starter
                player.party.push(create_battle_pokemon(starter, level));
                player.active_pokemon_id = Some(25);
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64();
                dex.push(CaughtPokemon {
                    id: 25,
                    name: starter.name.clone(),
                    types: starter.types.clone(),
                    abilities: starter.abilities.clone(),
                    stats: starter.stats.clone(),
                    rarity: starter.rarity.clone(),
                    rarity_score: starter.rarity_score,
                    caught_at: now,
                    ball_used: Some("poke_ball".to_string()),
                });
                let _ = self.save_pokedex(username, &dex);
                if !player.stats.lifetime_unique_pids.contains(&25) {
                    player.stats.lifetime_unique_pids.push(25);
                }
                player.stats.total_lifetime_caught = player.stats.total_lifetime_caught.max(1);
                modified = true;
            }
        }

        // Ensure active_pokemon_id is set
        if player.active_pokemon_id.is_none() && !player.party.is_empty() {
            player.active_pokemon_id = Some(player.party[0].id);
            modified = true;
        }

        // Revert any party member sprite_url that points to remote Supabase to local static sprite
        for member in &mut player.party {
            if member.sprite_url.is_empty() || member.sprite_url.starts_with("http") {
                member.sprite_url = format!("/sprites/{}.png", member.id);
                modified = true;
            }
        }

        // Ensure player has starter Poké Balls
        let total_balls: u32 = player.balls.values().sum();
        if total_balls == 0 {
            player.balls.insert("poke_ball".to_string(), 50);
            modified = true;
        }

        if modified {
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

    fn load_player_from_file(&self, username: &str) -> PlayerData {
        let file = self.player_file(username);
        if file.exists() {
            if let Ok(content) = fs::read_to_string(&file) {
                if let Ok(mut data) = serde_json::from_str::<PlayerData>(&content) {
                    data.username = username.to_string();
                    return data;
                }
            }
        }
        let mut def = PlayerData::default();
        def.username = username.to_string();
        def
    }

    fn save_player_to_file(&self, player: &PlayerData) -> std::io::Result<()> {
        let target = self.save_dir.join(format!("player_{}.json", Self::safe_username(&player.username)));
        let temp = self.save_dir.join(format!("player_{}.json.tmp", Self::safe_username(&player.username)));
        let json = serde_json::to_string_pretty(player)?;
        fs::write(&temp, json)?;
        fs::rename(temp, target)?;
        Ok(())
    }

    pub fn load_pokedex(&self, username: &str) -> Vec<CaughtPokemon> {
        if let Some(supabase) = &self.supabase {
            match Self::block_on(supabase.load_pokedex(username)) {
                Ok(dex) if !dex.is_empty() => return dex,
                Ok(_) => {
                    let local = self.load_pokedex_from_file(username);
                    if !local.is_empty() {
                        println!("⚡ Migrating local Pokédex to Supabase ({} entries)...", local.len());
                        let _ = Self::block_on(supabase.save_pokedex(username, &local));
                        return local;
                    }
                    return Vec::new();
                }
                Err(e) => {
                    eprintln!("[Supabase] Error loading pokedex: {}. Falling back to local file.", e);
                    return self.load_pokedex_from_file(username);
                }
            }
        }
        self.load_pokedex_from_file(username)
    }

    pub fn save_pokedex(&self, username: &str, dex: &[CaughtPokemon]) -> std::io::Result<()> {
        if let Some(supabase) = &self.supabase {
            if let Err(e) = Self::block_on(supabase.save_pokedex(username, dex)) {
                eprintln!("[Supabase] Error saving pokedex: {}", e);
            }
        }
        self.save_pokedex_to_file(username, dex)?;
        Ok(())
    }

    fn load_pokedex_from_file(&self, username: &str) -> Vec<CaughtPokemon> {
        let file = self.pokedex_file(username);
        if file.exists() {
            if let Ok(content) = fs::read_to_string(&file) {
                if let Ok(dex) = serde_json::from_str::<Vec<CaughtPokemon>>(&content) {
                    return dex;
                }
            }
        }
        Vec::new()
    }

    fn save_pokedex_to_file(&self, username: &str, dex: &[CaughtPokemon]) -> std::io::Result<()> {
        let target = self.save_dir.join(format!("pokedex_{}.json", Self::safe_username(username)));
        let temp = self.save_dir.join(format!("pokedex_{}.json.tmp", Self::safe_username(username)));
        let json = serde_json::to_string_pretty(dex)?;
        fs::write(&temp, json)?;
        fs::rename(temp, target)?;
        Ok(())
    }

    pub fn load_wild_encounter(&self, username: &str) -> Option<WildPokemonEncounter> {
        if let Some(supabase) = &self.supabase {
            match Self::block_on(supabase.load_wild_encounter(username)) {
                Ok(wild) => return wild,
                Err(e) => {
                    eprintln!("[Supabase] Error loading wild encounter: {}. Falling back to local file.", e);
                }
            }
        }
        self.load_wild_encounter_from_file(username)
    }

    pub fn save_wild_encounter(&self, username: &str, wild: &WildPokemonEncounter) -> std::io::Result<()> {
        if let Some(supabase) = &self.supabase {
            if let Err(e) = Self::block_on(supabase.save_wild_encounter(username, wild)) {
                eprintln!("[Supabase] Error saving wild encounter: {}", e);
            }
        }
        self.save_wild_encounter_to_file(username, wild)?;
        Ok(())
    }

    fn load_wild_encounter_from_file(&self, username: &str) -> Option<WildPokemonEncounter> {
        let file = self.wild_file(username);
        if file.exists() {
            if let Ok(content) = fs::read_to_string(&file) {
                if let Ok(wild) = serde_json::from_str::<WildPokemonEncounter>(&content) {
                    return Some(wild);
                }
            }
        }
        None
    }

    fn save_wild_encounter_to_file(&self, username: &str, wild: &WildPokemonEncounter) -> std::io::Result<()> {
        let target = self.save_dir.join(format!("wild_{}.json", Self::safe_username(username)));
        let temp = self.save_dir.join(format!("wild_{}.json.tmp", Self::safe_username(username)));
        let json = serde_json::to_string_pretty(wild)?;
        fs::write(&temp, json)?;
        fs::rename(temp, target)?;
        Ok(())
    }

    pub fn clear_wild_encounter(&self, username: &str) {
        if let Some(supabase) = &self.supabase {
            if let Err(e) = Self::block_on(supabase.clear_wild_encounter(username)) {
                eprintln!("[Supabase] Error clearing wild encounter: {}", e);
            }
        }
        let _ = fs::remove_file(self.wild_file(username));
    }

    pub fn load_arena_battle(&self, username: &str) -> Option<ActiveArenaBattle> {
        if let Some(supabase) = &self.supabase {
            match Self::block_on(supabase.load_arena_battle(username)) {
                Ok(battle) => return battle,
                Err(e) => {
                    eprintln!("[Supabase] Error loading arena battle: {}. Falling back to local file.", e);
                }
            }
        }
        self.load_arena_battle_from_file(username)
    }

    pub fn save_arena_battle(&self, username: &str, battle: &ActiveArenaBattle) -> std::io::Result<()> {
        if let Some(supabase) = &self.supabase {
            if let Err(e) = Self::block_on(supabase.save_arena_battle(username, battle)) {
                eprintln!("[Supabase] Error saving arena battle: {}", e);
            }
        }
        self.save_arena_battle_to_file(username, battle)?;
        Ok(())
    }

    fn load_arena_battle_from_file(&self, username: &str) -> Option<ActiveArenaBattle> {
        let file = self.arena_file(username);
        if file.exists() {
            if let Ok(content) = fs::read_to_string(&file) {
                if let Ok(battle) = serde_json::from_str::<ActiveArenaBattle>(&content) {
                    return Some(battle);
                }
            }
        }
        None
    }

    fn save_arena_battle_to_file(&self, username: &str, battle: &ActiveArenaBattle) -> std::io::Result<()> {
        let target = self.save_dir.join(format!("arena_{}.json", Self::safe_username(username)));
        let temp = self.save_dir.join(format!("arena_{}.json.tmp", Self::safe_username(username)));
        let json = serde_json::to_string_pretty(battle)?;
        fs::write(&temp, json)?;
        fs::rename(temp, target)?;
        Ok(())
    }

    pub fn clear_arena_battle(&self, username: &str) {
        if let Some(supabase) = &self.supabase {
            if let Err(e) = Self::block_on(supabase.clear_arena_battle(username)) {
                eprintln!("[Supabase] Error clearing arena battle: {}", e);
            }
        }
        let _ = fs::remove_file(self.arena_file(username));
    }

    pub fn reset_game(&self, username: &str) -> std::io::Result<()> {
        if let Some(supabase) = &self.supabase {
            if let Err(e) = Self::block_on(supabase.reset_game(username)) {
                eprintln!("[Supabase] Error resetting game: {}", e);
            }
        }
        let mut default_player = PlayerData::default();
        default_player.username = username.to_string();
        self.save_player_to_file(&default_player)?;
        self.save_pokedex_to_file(username, &[])?;
        self.clear_wild_encounter(username);
        self.clear_arena_battle(username);
        Ok(())
    }

    pub fn get_leaderboard(&self) -> Vec<LeaderboardEntry> {
        if let Some(supabase) = &self.supabase {
            match Self::block_on(supabase.get_leaderboard()) {
                Ok(entries) => return entries,
                Err(e) => {
                    eprintln!("[Supabase] Error fetching leaderboard: {}", e);
                }
            }
        }

        // Local fallback: scan player_*.json in save_dir
        let mut players = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.save_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("player_") && name.ends_with(".json") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(p) = serde_json::from_str::<PlayerData>(&content) {
                                players.push(p);
                            }
                        }
                    }
                }
            }
        }

        players.sort_by(|a, b| b.xp.cmp(&a.xp));
        let mut list = Vec::new();
        for (i, p) in players.into_iter().enumerate() {
            let lead = p.party.first();
            list.push(LeaderboardEntry {
                rank: i + 1,
                username: p.username,
                level: get_player_level(p.xp),
                xp: p.xp,
                currency: p.currency,
                badges_count: p.arena_badges.len(),
                unique_caught: p.stats.lifetime_unique_pids.len(),
                lead_pokemon_id: lead.map(|l| l.id),
                lead_pokemon_name: lead.map(|l| l.name.clone()),
                lead_pokemon_level: lead.map(|l| l.level),
                lead_pokemon_sprite: lead.map(|l| {
                    if !l.sprite_url.is_empty() && !l.sprite_url.starts_with("http") {
                        l.sprite_url.clone()
                    } else {
                        format!("/sprites/{}.png", l.id)
                    }
                }),
            });
        }
        list
    }

    pub fn now_sec() -> f64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }
}
