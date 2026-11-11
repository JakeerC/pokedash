use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::game::get_player_level;
use crate::models::{
    ActiveArenaBattle, AuthResponse, AuthUser, CaughtPokemon, LeaderboardEntry, PlayerData,
    PokemonDbRow, WildPokemonEncounter,
};

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
    token_cache: Arc<RwLock<HashMap<String, (AuthUser, Instant)>>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct SupabaseAuthUser {
    id: String,
    email: Option<String>,
    user_metadata: Option<HashMap<String, serde_json::Value>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct SupabaseTokenResponse {
    access_token: Option<String>,
    user: Option<SupabaseAuthUser>,
    msg: Option<String>,
    message: Option<String>,
    error_description: Option<String>,
    error: Option<String>,
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
            .timeout(Duration::from_secs(15))
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Self {
            client,
            config,
            token_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    // ============================================================
    // AUTHENTICATION METHODS
    // ============================================================

    /// Register a new user with email, password, and chosen trainer username
    pub async fn signup(
        &self,
        email: &str,
        password: &str,
        username: &str,
    ) -> Result<AuthResponse, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/auth/v1/signup", self.config.url);
        let mut user_metadata = HashMap::new();
        user_metadata.insert("username", serde_json::json!(username));

        let payload = serde_json::json!({
            "email": email,
            "password": password,
            "data": user_metadata
        });

        let resp = self.client.post(&url).json(&payload).send().await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let err_msg = body
                .get("msg")
                .or_else(|| body.get("message"))
                .or_else(|| body.get("error_description"))
                .and_then(|v| v.as_str())
                .unwrap_or("Registration failed");
            return Err(err_msg.to_string().into());
        }

        // Supabase returns 200 with empty identities when email already exists and enumeration protection is on
        if let Some(identities) = body.get("identities").and_then(|v| v.as_array()) {
            if identities.is_empty() {
                return Err("An account with this email already exists. Please sign in instead.".into());
            }
        }

        let user_id = body
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let auth_user = AuthUser {
            id: user_id,
            email: email.to_string(),
            username: username.to_string(),
        };

        if !token.is_empty() {
            let mut cache = self.token_cache.write().unwrap();
            cache.insert(token.clone(), (auth_user.clone(), Instant::now()));
            Ok(AuthResponse {
                token,
                user: auth_user,
                requires_verification: false,
                message: Some("Trainer Passport issued successfully!".to_string()),
            })
        } else {
            Ok(AuthResponse {
                token: String::new(),
                user: auth_user,
                requires_verification: true,
                message: Some(
                    "Confirmation code sent to your email! Please enter the code or click the confirmation link to activate your Trainer ID."
                        .to_string(),
                ),
            })
        }
    }

    /// Sign in with existing email and password
    pub async fn login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<AuthResponse, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/auth/v1/token?grant_type=password", self.config.url);
        let payload = serde_json::json!({
            "email": email,
            "password": password
        });

        let resp = self.client.post(&url).json(&payload).send().await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let error_code = body.get("error_code").and_then(|v| v.as_str()).unwrap_or("");
            let err_msg = body
                .get("msg")
                .or_else(|| body.get("message"))
                .or_else(|| body.get("error_description"))
                .and_then(|v| v.as_str())
                .unwrap_or("Login failed");

            if error_code == "email_not_confirmed" {
                return Err(
                    "Email not confirmed. Please enter your 6-digit confirmation code or click the email link."
                        .into(),
                );
            }

            return Err(err_msg.to_string().into());
        }

        let token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or("Missing access token in auth response")?
            .to_string();

        let user_obj = body.get("user").ok_or("Missing user in auth response")?;
        let user_id = user_obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let user_email = user_obj
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or(email)
            .to_string();

        let username = user_obj
            .get("user_metadata")
            .and_then(|m| m.get("username"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                email
                    .split('@')
                    .next()
                    .unwrap_or("Trainer")
                    .to_string()
            });

        let auth_user = AuthUser {
            id: user_id,
            email: user_email,
            username,
        };

        let mut cache = self.token_cache.write().unwrap();
        cache.insert(token.clone(), (auth_user.clone(), Instant::now()));

        Ok(AuthResponse {
            token,
            user: auth_user,
            requires_verification: false,
            message: Some("Welcome back, Trainer!".to_string()),
        })
    }

    /// Verify an email code / token
    pub async fn verify_code(
        &self,
        email: &str,
        token: &str,
        verify_type: Option<&str>,
    ) -> Result<AuthResponse, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/auth/v1/verify", self.config.url);
        let payload = serde_json::json!({
            "type": verify_type.unwrap_or("signup"),
            "email": email,
            "token": token
        });

        let resp = self.client.post(&url).json(&payload).send().await?;
        let status = resp.status();
        let body: serde_json::Value = resp.json().await?;

        if !status.is_success() {
            let err_msg = body
                .get("msg")
                .or_else(|| body.get("message"))
                .or_else(|| body.get("error_description"))
                .and_then(|v| v.as_str())
                .unwrap_or("Verification failed");
            return Err(err_msg.to_string().into());
        }

        let access_token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or("Verification succeeded but no session token was issued")?
            .to_string();

        let user_obj = body.get("user").ok_or("Missing user in verify response")?;
        let user_id = user_obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let user_email = user_obj
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or(email)
            .to_string();

        let username = user_obj
            .get("user_metadata")
            .and_then(|m| m.get("username"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                email
                    .split('@')
                    .next()
                    .unwrap_or("Trainer")
                    .to_string()
            });

        let auth_user = AuthUser {
            id: user_id,
            email: user_email,
            username,
        };

        let mut cache = self.token_cache.write().unwrap();
        cache.insert(access_token.clone(), (auth_user.clone(), Instant::now()));

        Ok(AuthResponse {
            token: access_token,
            user: auth_user,
            requires_verification: false,
            message: Some("Trainer email verified successfully!".to_string()),
        })
    }

    /// Verify a Bearer token and return the associated AuthUser (cached for performance)
    pub async fn verify_token(
        &self,
        token: &str,
    ) -> Result<AuthUser, Box<dyn std::error::Error + Send + Sync>> {
        // 1. Check in-memory cache (TTL: 10 minutes)
        {
            let cache = self.token_cache.read().unwrap();
            if let Some((user, timestamp)) = cache.get(token) {
                if timestamp.elapsed() < Duration::from_secs(600) {
                    return Ok(user.clone());
                }
            }
        }

        // 2. Fetch from Supabase Auth
        let url = format!("{}/auth/v1/user", self.config.url);
        let resp = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err("Invalid or expired session token".into());
        }

        let user_obj: serde_json::Value = resp.json().await?;
        let user_id = user_obj
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let user_email = user_obj
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown@trainer.net")
            .to_string();

        let username = user_obj
            .get("user_metadata")
            .and_then(|m| m.get("username"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                user_email
                    .split('@')
                    .next()
                    .unwrap_or("Trainer")
                    .to_string()
            });

        let auth_user = AuthUser {
            id: user_id,
            email: user_email,
            username,
        };

        // Cache valid user
        let mut cache = self.token_cache.write().unwrap();
        cache.insert(token.to_string(), (auth_user.clone(), Instant::now()));

        Ok(auth_user)
    }

    /// Check if a username is already taken by another player (case-insensitive)
    pub async fn is_username_taken(
        &self,
        username: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/rest/v1/players?username=ilike.{}&select=username",
            self.config.url, username
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(false);
        }
        #[derive(Deserialize)]
        struct Row {
            #[allow(dead_code)]
            username: String,
        }
        let rows: Vec<Row> = resp.json().await?;
        Ok(!rows.is_empty())
    }

    /// Resend email confirmation code
    pub async fn resend_code(
        &self,
        email: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/auth/v1/resend", self.config.url);
        let payload = serde_json::json!({
            "type": "signup",
            "email": email
        });
        let resp = self.client.post(&url).json(&payload).send().await?;
        if !resp.status().is_success() {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            let err_msg = body
                .get("msg")
                .or_else(|| body.get("message"))
                .or_else(|| body.get("error_description"))
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to resend confirmation code");
            return Err(err_msg.to_string().into());
        }
        Ok(())
    }

    // ============================================================
    // PLAYER AND GAME DATA METHODS
    // ============================================================

    pub async fn load_player(
        &self,
        username: &str,
    ) -> Result<Option<PlayerData>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/rest/v1/players?username=eq.{}&select=username,currency,xp,balls,last_hunt_time,stats,active_pokemon_id,party,arena_badges",
            self.config.url, username
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase load_player error: {}", err).into());
        }
        let players: Vec<PlayerData> = resp.json().await?;
        Ok(players.into_iter().next())
    }

    pub async fn save_player(
        &self,
        player: &PlayerData,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    pub async fn load_pokedex(
        &self,
        username: &str,
    ) -> Result<Vec<CaughtPokemon>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/rest/v1/pokedex?username=eq.{}&select=id,name,types,abilities,stats,rarity,rarity_score,caught_at,ball_used&order=caught_at.asc",
            self.config.url, username
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase load_pokedex error: {}", err).into());
        }
        let dex: Vec<CaughtPokemon> = resp.json().await?;
        Ok(dex)
    }

    pub async fn save_pokedex(
        &self,
        username: &str,
        dex: &[CaughtPokemon],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                p_username: username,
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

    pub async fn load_wild_encounter(
        &self,
        username: &str,
    ) -> Result<Option<WildPokemonEncounter>, Box<dyn std::error::Error + Send + Sync>> {
        #[derive(Deserialize)]
        struct Row {
            encounter: Option<WildPokemonEncounter>,
        }
        let url = format!(
            "{}/rest/v1/wild_encounters?username=eq.{}&select=encounter",
            self.config.url, username
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase load_wild_encounter error: {}", err).into());
        }
        let rows: Vec<Row> = resp.json().await?;
        Ok(rows.into_iter().next().and_then(|r| r.encounter))
    }

    pub async fn save_wild_encounter(
        &self,
        username: &str,
        wild: &WildPokemonEncounter,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                username,
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

    pub async fn clear_wild_encounter(
        &self,
        username: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/rest/v1/wild_encounters?username=eq.{}",
            self.config.url, username
        );
        let resp = self.client.delete(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase clear_wild_encounter error: {}", err).into());
        }
        Ok(())
    }

    pub async fn load_arena_battle(
        &self,
        username: &str,
    ) -> Result<Option<ActiveArenaBattle>, Box<dyn std::error::Error + Send + Sync>> {
        #[derive(Deserialize)]
        struct Row {
            battle: Option<ActiveArenaBattle>,
        }
        let url = format!(
            "{}/rest/v1/arena_battles?username=eq.{}&select=battle",
            self.config.url, username
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase load_arena_battle error: {}", err).into());
        }
        let rows: Vec<Row> = resp.json().await?;
        Ok(rows.into_iter().next().and_then(|r| r.battle))
    }

    pub async fn save_arena_battle(
        &self,
        username: &str,
        battle: &ActiveArenaBattle,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                username,
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

    pub async fn clear_arena_battle(
        &self,
        username: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/rest/v1/arena_battles?username=eq.{}",
            self.config.url, username
        );
        let resp = self.client.delete(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase clear_arena_battle error: {}", err).into());
        }
        Ok(())
    }

    pub async fn reset_game(
        &self,
        username: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut default_player = PlayerData::default();
        default_player.username = username.to_string();
        self.save_player(&default_player).await?;
        self.save_pokedex(username, &[]).await?;
        let _ = self.clear_wild_encounter(username).await;
        let _ = self.clear_arena_battle(username).await;
        Ok(())
    }

    /// Fetch real-time multiplayer leaderboard ranked by XP descending
    pub async fn get_leaderboard(
        &self,
    ) -> Result<Vec<LeaderboardEntry>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "{}/rest/v1/players?select=*&order=xp.desc&limit=50",
            self.config.url
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase get_leaderboard error: {}", err).into());
        }
        let players: Vec<PlayerData> = resp.json().await?;
        let mut entries = Vec::new();
        for (i, p) in players.into_iter().enumerate() {
            let lead = p.party.first();
            entries.push(LeaderboardEntry {
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
                    if !l.sprite_url.is_empty() {
                        l.sprite_url.clone()
                    } else {
                        format!("/sprites/{}.png", l.id)
                    }
                }),
            });
        }
        Ok(entries)
    }

    pub async fn load_all_pokemon(&self) -> Result<Vec<PokemonDbRow>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/rest/v1/pokemon?select=*&order=id.asc", self.config.url);
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(format!("Supabase load_all_pokemon error: {}", err).into());
        }
        let list: Vec<PokemonDbRow> = resp.json().await?;
        Ok(list)
    }
}
