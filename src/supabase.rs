use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

use crate::models::{ActiveArenaBattle, CaughtPokemon, PlayerData, WildPokemonEncounter};

#[derive(Clone, Debug)]
pub struct SupabaseConfig {
    pub url: String,
    pub anon_key: String,
    pub username: String,
}

impl SupabaseConfig {
    pub fn from_env_or_default() -> Self {
        load_dotenv();

        let url = std::env::var("SUPABASE_URL")
            .unwrap_or_else(|_| "https://njylaotcvsycimtutlaf.supabase.co".to_string());
        let anon_key = std::env::var("SUPABASE_KEY")
            .unwrap_or_else(|_| "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Im5qeWxhb3RjdnN5Y2ltdHV0bGFmIiwicm9sZSI6ImFub24iLCJpYXQiOjE3OTQwMzIwNTQsImV4cCI6MjEwOTYwODA1NH0.NyIfTisDgacQAltN0b7tndjDhA3tIqXuy_MH7U5qG_A".to_string());
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "Trainer".to_string());

        Self {
            url,
            anon_key,
            username,
        }
    }
}

pub fn load_dotenv() {
    let candidates = [".env", "../.env"];
    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    let k = k.trim();
                    let v = v.trim().trim_matches('"').trim_matches('\'');
                    if std::env::var(k).is_err() {
                        std::env::set_var(k, v);
                    }
                }
            }
            break;
        }
    }
}

#[derive(Clone)]
pub struct SupabaseClient {
    client: reqwest::Client,
    pub config: SupabaseConfig,
}

impl SupabaseClient {
    pub fn new(config: SupabaseConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut headers = HeaderMap::new();

        let mut key_val = HeaderValue::from_str(&config.anon_key)?;
        key_val.set_sensitive(true);
        headers.insert("apikey", key_val.clone());

        let mut auth_val = HeaderValue::from_str(&format!("Bearer {}", config.anon_key))?;
        auth_val.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth_val);

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(15))
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Self { client, config })
    }

    pub async fn load_player(&self) -> Result<Option<PlayerData>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/rest/v1/players?username=eq.{}&select=username,currency,xp,balls,last_hunt_time,stats,active_pokemon_id,party,arena_badges",
            self.config.url, self.config.username
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase load_player error: {}", err).into());
        }
        let players: Vec<PlayerData> = resp.json().await?;
        Ok(players.into_iter().next())
    }

    pub async fn save_player(&self, player: &PlayerData) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/rest/v1/players", self.config.url);
        let resp = self
            .client
            .post(&url)
            .header("Prefer", "resolution=merge-duplicates")
            .json(player)
            .send()
            .await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase save_player error: {}", err).into());
        }
        Ok(())
    }

    pub async fn load_pokedex(&self) -> Result<Vec<CaughtPokemon>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/rest/v1/pokedex?username=eq.{}&select=id,name,types,abilities,stats,rarity,rarity_score,caught_at,ball_used&order=caught_at.asc",
            self.config.url, self.config.username
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase load_pokedex error: {}", err).into());
        }
        let dex: Vec<CaughtPokemon> = resp.json().await?;
        Ok(dex)
    }

    pub async fn save_pokedex(&self, dex: &[CaughtPokemon]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/rest/v1/rpc/save_pokedex", self.config.url);
        #[derive(Serialize)]
        struct RpcArgs<'a> {
            p_username: &'a str,
            p_entries: &'a [CaughtPokemon],
        }
        let resp = self
            .client
            .post(&url)
            .json(&RpcArgs {
                p_username: &self.config.username,
                p_entries: dex,
            })
            .send()
            .await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase save_pokedex error: {}", err).into());
        }
        Ok(())
    }

    pub async fn load_wild_encounter(&self) -> Result<Option<WildPokemonEncounter>, Box<dyn std::error::Error + Send + Sync>> {
        #[derive(Deserialize)]
        struct Row {
            encounter: Option<WildPokemonEncounter>,
        }
        let url = format!(
            "{}/rest/v1/wild_encounters?username=eq.{}&select=encounter",
            self.config.url, self.config.username
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase load_wild_encounter error: {}", err).into());
        }
        let rows: Vec<Row> = resp.json().await?;
        Ok(rows.into_iter().next().and_then(|r| r.encounter))
    }

    pub async fn save_wild_encounter(&self, wild: &WildPokemonEncounter) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[derive(Serialize)]
        struct Row<'a> {
            username: &'a str,
            encounter: &'a WildPokemonEncounter,
        }
        let url = format!("{}/rest/v1/wild_encounters", self.config.url);
        let resp = self
            .client
            .post(&url)
            .header("Prefer", "resolution=merge-duplicates")
            .json(&Row {
                username: &self.config.username,
                encounter: wild,
            })
            .send()
            .await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase save_wild_encounter error: {}", err).into());
        }
        Ok(())
    }

    pub async fn clear_wild_encounter(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/rest/v1/wild_encounters?username=eq.{}",
            self.config.url, self.config.username
        );
        let resp = self.client.delete(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase clear_wild_encounter error: {}", err).into());
        }
        Ok(())
    }

    pub async fn load_arena_battle(&self) -> Result<Option<ActiveArenaBattle>, Box<dyn std::error::Error + Send + Sync>> {
        #[derive(Deserialize)]
        struct Row {
            battle: Option<ActiveArenaBattle>,
        }
        let url = format!(
            "{}/rest/v1/arena_battles?username=eq.{}&select=battle",
            self.config.url, self.config.username
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase load_arena_battle error: {}", err).into());
        }
        let rows: Vec<Row> = resp.json().await?;
        Ok(rows.into_iter().next().and_then(|r| r.battle))
    }

    pub async fn save_arena_battle(&self, battle: &ActiveArenaBattle) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[derive(Serialize)]
        struct Row<'a> {
            username: &'a str,
            battle: &'a ActiveArenaBattle,
        }
        let url = format!("{}/rest/v1/arena_battles", self.config.url);
        let resp = self
            .client
            .post(&url)
            .header("Prefer", "resolution=merge-duplicates")
            .json(&Row {
                username: &self.config.username,
                battle,
            })
            .send()
            .await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase save_arena_battle error: {}", err).into());
        }
        Ok(())
    }

    pub async fn clear_arena_battle(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/rest/v1/arena_battles?username=eq.{}",
            self.config.url, self.config.username
        );
        let resp = self.client.delete(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase clear_arena_battle error: {}", err).into());
        }
        Ok(())
    }

    pub async fn reset_game(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let default_player = PlayerData::default();
        self.save_player(&default_player).await?;
        self.save_pokedex(&[]).await?;
        let _ = self.clear_wild_encounter().await;
        let _ = self.clear_arena_battle().await;
        Ok(())
    }
}
