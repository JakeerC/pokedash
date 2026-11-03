use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use rand::seq::SliceRandom;
use rand::Rng;

use crate::models::{
    BuyResult, CatchAttemptResult, CaughtPokemon, PlayerData,
    SellResult, StoreItemInfo, WildPokemonEncounter,
};
use crate::game::{
    calculate_catch_chance, canonicalize_ball, get_ball_spec, get_catch_xp_reward,
    get_generation, get_level_discount, get_player_level, get_rarity_sell_price,
    get_rarity_spawn_weights, HUNT_COOLDOWN_SECONDS, STORE_BALLS, XP_REWARD_HUNT,
};
use crate::storage::Storage;

#[derive(Clone)]
pub struct GameService {
    pub storage: Arc<Storage>,
    pub lock: Arc<RwLock<()>>,
}

impl GameService {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            storage,
            lock: Arc::new(RwLock::new(())),
        }
    }

    pub fn get_player(&self) -> PlayerData {
        let _guard = self.lock.read().unwrap();
        self.storage.load_player()
    }

    pub fn get_wild(&self) -> Option<WildPokemonEncounter> {
        let _guard = self.lock.read().unwrap();
        self.storage.load_wild_encounter()
    }

    pub fn get_pokedex(&self) -> Vec<CaughtPokemon> {
        let _guard = self.lock.read().unwrap();
        self.storage.load_pokedex()
    }

    pub fn hunt(&self) -> Result<WildPokemonEncounter, String> {
        let _guard = self.lock.write().unwrap();
        let mut player = self.storage.load_player();
        let now = Storage::now_sec();

        let time_since_last = now - player.last_hunt_time;
        if time_since_last < HUNT_COOLDOWN_SECONDS {
            let wait = (HUNT_COOLDOWN_SECONDS - time_since_last).ceil() as u32;
            return Err(format!(
                "You are tired from your last hunt. Please wait {} more seconds.",
                wait
            ));
        }

        // Roll rarity
        let weights = get_rarity_spawn_weights();
        let mut rng = rand::thread_rng();
        let total_weight: f64 = weights.iter().map(|(_, w)| *w).sum();
        let mut roll = rng.gen_range(0.0..total_weight);
        let mut chosen_rarity = "Common";
        for (rarity, weight) in weights {
            if roll < *weight {
                chosen_rarity = rarity;
                break;
            }
            roll -= *weight;
        }

        // Pick random Pokemon of that rarity
        let pool = self
            .storage
            .rarity_pools
            .get(&chosen_rarity.to_lowercase())
            .ok_or_else(|| format!("No pokemon found for rarity {}", chosen_rarity))?;

        if pool.is_empty() {
            return Err(format!("Empty pool for rarity {}", chosen_rarity));
        }

        let chosen_id = *pool.choose(&mut rng).unwrap();
        let pokemon = self
            .storage
            .pokemon_catalog
            .get(&chosen_id)
            .cloned()
            .ok_or_else(|| "Selected Pokémon not in catalog".to_string())?;

        // Update player data for hunt
        player.last_hunt_time = now;
        player.stats.total_hunts += 1;
        player.stats.xp_from_hunting += XP_REWARD_HUNT;
        player.xp += XP_REWARD_HUNT as u64;

        self.storage.save_player(&player).map_err(|e| e.to_string())?;

        let encounter = WildPokemonEncounter {
            pokemon,
            encountered_at: now,
            cooldown_seconds: HUNT_COOLDOWN_SECONDS as u32,
        };

        self.storage
            .save_wild_encounter(&encounter)
            .map_err(|e| e.to_string())?;

        Ok(encounter)
    }

    pub fn catch(&self, ball_input: &str) -> Result<CatchAttemptResult, String> {
        let _guard = self.lock.write().unwrap();
        let encounter = self
            .storage
            .load_wild_encounter()
            .ok_or_else(|| "You haven't found a Pokémon to catch! Use 'hunt' first.".to_string())?;

        let ball_id = canonicalize_ball(ball_input).ok_or_else(|| {
            format!(
                "Unknown ball type '{}'. Options: pb, gb, ub, net, dive, fast, dusk, nest, repeat, quick, mb",
                ball_input
            )
        })?;

        let mut player = self.storage.load_player();
        let current_count = player.balls.get(ball_id).copied().unwrap_or(0);
        if current_count == 0 {
            return Err(format!("You don't have any {}s!", ball_id.replace('_', " ")));
        }

        // Spend 1 ball
        let remaining_balls = current_count - 1;
        player.balls.insert(ball_id.to_string(), remaining_balls);

        let pokedex = self.storage.load_pokedex();
        let already_caught = pokedex.iter().any(|p| p.id == encounter.pokemon.id);
        let is_new = !already_caught;

        let player_level_before = get_player_level(player.xp);
        let catch_chance = calculate_catch_chance(
            &encounter.pokemon,
            ball_id,
            player_level_before,
            already_caught,
        );

        let mut rng = rand::thread_rng();
        let roll: f64 = rng.gen();
        let success = roll < catch_chance;

        let mut xp_gained = 0u32;
        let mut leveled_up = false;
        let mut new_level = player_level_before;
        let message;

        if success {
            let mut base_xp = get_catch_xp_reward(&encounter.pokemon.rarity);
            if is_new {
                base_xp *= 2; // Doubled for new species
            }
            xp_gained = base_xp;

            player.xp += xp_gained as u64;
            player.stats.successful_catches += 1;
            player.stats.total_lifetime_caught += 1;
            player.stats.xp_from_catching += xp_gained;

            let rarity_key = encounter.pokemon.rarity.to_lowercase();
            *player.stats.lifetime_rarity.entry(rarity_key).or_insert(0) += 1;

            let gen_key = get_generation(encounter.pokemon.id).to_string();
            *player.stats.lifetime_gens.entry(gen_key).or_insert(0) += 1;

            if is_new {
                player.stats.first_time_catches += 1;
                player.stats.lifetime_unique_pids.push(encounter.pokemon.id);
            }

            new_level = get_player_level(player.xp);
            if new_level > player_level_before {
                leveled_up = true;
            }

            // Save to Pokedex
            let mut updated_dex = pokedex;
            updated_dex.push(CaughtPokemon {
                id: encounter.pokemon.id,
                name: encounter.pokemon.name.clone(),
                types: encounter.pokemon.types.clone(),
                abilities: encounter.pokemon.abilities.clone(),
                stats: encounter.pokemon.stats.clone(),
                rarity: encounter.pokemon.rarity.clone(),
                rarity_score: encounter.pokemon.rarity_score,
                caught_at: Storage::now_sec(),
                ball_used: Some(ball_id.to_string()),
            });
            self.storage
                .save_pokedex(&updated_dex)
                .map_err(|e| e.to_string())?;

            message = if is_new {
                format!(
                    "Gotcha! {} was caught! ✨ New Pokédex entry! XP doubled to +{} XP!",
                    encounter.pokemon.name, xp_gained
                )
            } else {
                format!(
                    "Gotcha! {} was caught! You earned +{} XP!",
                    encounter.pokemon.name, xp_gained
                )
            };
        } else {
            player.stats.failed_catches += 1;
            message = format!(
                "Oh no! Wild {} broke free and ran away!",
                encounter.pokemon.name
            );
        }

        self.storage.save_player(&player).map_err(|e| e.to_string())?;
        self.storage.clear_wild_encounter();

        Ok(CatchAttemptResult {
            success,
            pokemon: encounter.pokemon,
            ball: ball_id.to_string(),
            catch_chance,
            xp_gained,
            is_new,
            leveled_up,
            new_level,
            message,
            balls_remaining: remaining_balls,
        })
    }

    pub fn flee(&self) -> Result<String, String> {
        let _guard = self.lock.write().unwrap();
        if let Some(wild) = self.storage.load_wild_encounter() {
            self.storage.clear_wild_encounter();
            Ok(format!("{} fled into the wild.", wild.pokemon.name))
        } else {
            Err("No active wild Pokemon to flee from.".to_string())
        }
    }

    pub fn get_store_catalog(&self) -> Vec<StoreItemInfo> {
        let _guard = self.lock.read().unwrap();
        let player = self.storage.load_player();
        let level = get_player_level(player.xp);
        let discount = get_level_discount(level);

        STORE_BALLS
            .iter()
            .map(|spec| {
                let discounted = (spec.price as f64 * (1.0 - discount)).round() as u64;
                let owned = player.balls.get(spec.id).copied().unwrap_or(0);
                StoreItemInfo {
                    id: spec.id.to_string(),
                    name: spec.name.to_string(),
                    price: spec.price,
                    discounted_price: discounted.max(1),
                    category: spec.category.to_string(),
                    unlock_level: spec.unlock_level,
                    owned,
                    description: spec.description.to_string(),
                    alias: spec.alias.to_string(),
                }
            })
            .collect()
    }

    pub fn buy(&self, item_input: &str, amount: u32) -> Result<BuyResult, String> {
        if amount == 0 {
            return Err("Amount must be greater than 0".to_string());
        }
        if amount > 999 {
            return Err("Maximum purchase limit is 999 at once".to_string());
        }

        let ball_id = canonicalize_ball(item_input)
            .ok_or_else(|| format!("Unknown store item '{}'", item_input))?;
        let spec = get_ball_spec(ball_id).unwrap();

        let _guard = self.lock.write().unwrap();
        let mut player = self.storage.load_player();
        let level = get_player_level(player.xp);

        if level < spec.unlock_level {
            return Err(format!(
                "{} unlocks at Trainer Level {}. Your current level is {}.",
                spec.name, spec.unlock_level, level
            ));
        }

        let discount = get_level_discount(level);
        let unit_price = (spec.price as f64 * (1.0 - discount)).round() as u64;
        let unit_price = unit_price.max(1);
        let total_cost = unit_price * amount as u64;

        if player.currency < total_cost {
            return Err(format!(
                "Not enough Poké Dollars! Need ₽{}, but you only have ₽{}.",
                total_cost, player.currency
            ));
        }

        player.currency -= total_cost;
        let new_owned = player.balls.entry(ball_id.to_string()).or_insert(0);
        *new_owned += amount;
        let count = *new_owned;

        self.storage.save_player(&player).map_err(|e| e.to_string())?;

        Ok(BuyResult {
            item: spec.name.to_string(),
            amount,
            total_cost,
            new_balance: player.currency,
            new_owned_count: count,
            message: format!(
                "Purchased {}x {} for ₽{}. Remaining balance: ₽{}",
                amount, spec.name, total_cost, player.currency
            ),
        })
    }

    pub fn sell_one(&self, pokemon_name: &str) -> Result<SellResult, String> {
        let _guard = self.lock.write().unwrap();
        let mut dex = self.storage.load_pokedex();
        let mut player = self.storage.load_player();

        let lower = pokemon_name.trim().to_lowercase();
        let index = dex
            .iter()
            .position(|p| p.name.to_lowercase() == lower || p.id.to_string() == lower)
            .ok_or_else(|| format!("You do not have any '{}' in your Pokédex to sell.", pokemon_name))?;

        let sold = dex.remove(index);
        let price = get_rarity_sell_price(&sold.rarity);
        player.currency += price;

        self.storage.save_pokedex(&dex).map_err(|e| e.to_string())?;
        self.storage.save_player(&player).map_err(|e| e.to_string())?;

        Ok(SellResult {
            sold_count: 1,
            total_earnings: price,
            new_balance: player.currency,
            message: format!(
                "Sold {} ({}) for ₽{}. New balance: ₽{}",
                sold.name, sold.rarity, price, player.currency
            ),
        })
    }

    pub fn sell_all_by_rarity(&self, rarity_input: &str) -> Result<SellResult, String> {
        let _guard = self.lock.write().unwrap();
        let dex = self.storage.load_pokedex();
        let mut player = self.storage.load_player();

        let target_rarity = rarity_input.trim().to_lowercase();
        let mut kept = Vec::new();
        let mut sold_count = 0;
        let mut total_earnings = 0u64;

        for p in dex {
            if p.rarity.to_lowercase() == target_rarity {
                let price = get_rarity_sell_price(&p.rarity);
                total_earnings += price;
                sold_count += 1;
            } else {
                kept.push(p);
            }
        }

        if sold_count == 0 {
            return Err(format!("You have no {} Pokémon to sell.", rarity_input));
        }

        player.currency += total_earnings;
        self.storage.save_pokedex(&kept).map_err(|e| e.to_string())?;
        self.storage.save_player(&player).map_err(|e| e.to_string())?;

        Ok(SellResult {
            sold_count,
            total_earnings,
            new_balance: player.currency,
            message: format!(
                "Sold {}x {} Pokémon for ₽{}. New balance: ₽{}",
                sold_count, rarity_input, total_earnings, player.currency
            ),
        })
    }

    pub fn sell_duplicates(&self) -> Result<SellResult, String> {
        let _guard = self.lock.write().unwrap();
        let dex = self.storage.load_pokedex();
        let mut player = self.storage.load_player();

        if dex.is_empty() {
            return Err("Your Pokédex is empty! There is nothing to sell.".to_string());
        }

        let mut species_map: HashMap<u32, Vec<CaughtPokemon>> = HashMap::new();
        for p in dex {
            species_map.entry(p.id).or_default().push(p);
        }

        let mut kept = Vec::new();
        let mut sold_count = 0;
        let mut total_earnings = 0u64;

        for (_pid, mut list) in species_map {
            // Keep first instance
            kept.push(list.remove(0));
            // Sell remaining duplicates
            for dup in list {
                let price = get_rarity_sell_price(&dup.rarity);
                total_earnings += price;
                sold_count += 1;
            }
        }

        if sold_count == 0 {
            return Err("You don't have any duplicate Pokémon to sell!".to_string());
        }

        kept.sort_by_key(|p| p.id);
        player.currency += total_earnings;

        self.storage.save_pokedex(&kept).map_err(|e| e.to_string())?;
        self.storage.save_player(&player).map_err(|e| e.to_string())?;

        Ok(SellResult {
            sold_count,
            total_earnings,
            new_balance: player.currency,
            message: format!(
                "Sold {} duplicate Pokémon for ₽{}. New balance: ₽{}",
                sold_count, total_earnings, player.currency
            ),
        })
    }
}
