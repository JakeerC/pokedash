use std::collections::HashMap;
use std::sync::Arc;
use rand::seq::SliceRandom;
use rand::Rng;

use crate::models::{
    ActiveArenaBattle, ArenaDivision, BattleActionResult, BattleMove, BattlePokemon,
    BuyResult, CatchAttemptResult, CaughtPokemon, ItemUseResult, LeaderboardEntry, PlayerData,
    SellResult, StoreItemInfo, WildPokemonEncounter,
};
use crate::game::{
    calculate_battle_damage, calculate_catch_chance, canonicalize_ball, create_battle_pokemon,
    generate_moves_for_pokemon, get_arena_divisions, get_ball_spec, get_catch_xp_reward,
    get_generation, get_level_discount, get_player_level, get_rarity_sell_price,
    get_rarity_spawn_weights, HUNT_COOLDOWN_SECONDS, STORE_BALLS, XP_REWARD_HUNT,
};
use crate::storage::Storage;

#[derive(Clone)]
pub struct GameService {
    pub storage: Arc<Storage>,
}

impl GameService {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            storage,
        }
    }

    pub fn get_player(&self, username: &str) -> PlayerData {
        self.storage.load_player(username)
    }

    pub fn get_wild(&self, username: &str) -> Option<WildPokemonEncounter> {
        self.storage.load_wild_encounter(username)
    }

    pub fn get_pokedex(&self, username: &str) -> Vec<CaughtPokemon> {
        self.storage.load_pokedex(username)
    }

    pub fn get_arena_battle(&self, username: &str) -> Option<ActiveArenaBattle> {
        self.storage.load_arena_battle(username)
    }

    pub fn get_leaderboard(&self) -> Vec<LeaderboardEntry> {
        self.storage.get_leaderboard()
    }

    pub fn reset_player(&self, username: &str) -> Result<PlayerData, String> {
        self.storage
            .reset_game(username)
            .map_err(|e| e.to_string())?;
        Ok(self.storage.load_player(username))
    }

    pub fn hunt(&self, username: &str) -> Result<WildPokemonEncounter, String> {
        let mut player = self.storage.load_player(username);
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

        let player_lvl = get_player_level(player.xp);
        let wild_level = (player_lvl as i32 + rng.gen_range(-2..=3)).max(3) as u32;
        let scale = (wild_level as f64) / 50.0;
        let max_hp = ((pokemon.stats.hp as f64 * scale) + 15.0).round().max(25.0) as u32;
        let moves = generate_moves_for_pokemon(&pokemon);

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
            current_hp: max_hp,
            max_hp,
            level: wild_level,
            moves,
        };

        self.storage
            .save_wild_encounter(username, &encounter)
            .map_err(|e| e.to_string())?;

        Ok(encounter)
    }

    pub fn catch(&self, username: &str, ball_input: &str) -> Result<CatchAttemptResult, String> {
        let encounter = self
            .storage
            .load_wild_encounter(username)
            .ok_or_else(|| "You haven't found a Pokémon to catch! Search the grass first.".to_string())?;

        let ball_id = canonicalize_ball(ball_input).ok_or_else(|| {
            format!(
                "Unknown ball type '{}'. Options: pb, gb, ub, net, dive, fast, dusk, nest, repeat, quick, mb",
                ball_input
            )
        })?;

        let mut player = self.storage.load_player(username);
        let current_count = player.balls.get(ball_id).copied().unwrap_or(0);
        if current_count == 0 {
            return Err(format!("You don't have any {}s!", ball_id.replace('_', " ")));
        }

        // Spend 1 ball
        let remaining_balls = current_count - 1;
        player.balls.insert(ball_id.to_string(), remaining_balls);

        let pokedex = self.storage.load_pokedex(username);
        let already_caught = pokedex.iter().any(|p| p.id == encounter.pokemon.id);
        let is_new = !already_caught;

        let player_level_before = get_player_level(player.xp);
        let catch_chance = calculate_catch_chance(
            &encounter.pokemon,
            ball_id,
            player_level_before,
            already_caught,
            encounter.current_hp,
            encounter.max_hp,
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
                base_xp *= 2;
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
                .save_pokedex(username, &updated_dex)
                .map_err(|e| e.to_string())?;

            // Add to battle party if party has room
            if player.party.len() < 6 {
                let battle_mon = create_battle_pokemon(&encounter.pokemon, encounter.level);
                player.party.push(battle_mon);
            }

            message = if is_new {
                format!(
                    "Gotcha! {} was caught! [NEW DEX ENTRY] XP doubled to +{} XP!",
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
                "Oh no! Wild {} broke free and fled!",
                encounter.pokemon.name
            );
        }

        self.storage.save_player(&player).map_err(|e| e.to_string())?;
        self.storage.clear_wild_encounter(username);

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

    pub fn wild_attack(&self, username: &str, move_index: usize) -> Result<BattleActionResult, String> {
        let mut encounter = self
            .storage
            .load_wild_encounter(username)
            .ok_or_else(|| "No active wild encounter to battle!".to_string())?;

        let mut player = self.storage.load_player(username);
        if player.party.is_empty() {
            return Err("You have no Pokémon ready to battle!".to_string());
        }

        // Auto-switch to first conscious Pokémon if current lead is fainted
        if player.party[0].current_hp == 0 {
            if let Some(healthy_idx) = player.party.iter().position(|p| p.current_hp > 0) {
                let healthy_mon = player.party.remove(healthy_idx);
                player.party.insert(0, healthy_mon);
                player.active_pokemon_id = Some(player.party[0].id);
                let _ = self.storage.save_player(&player);
            } else {
                return Err("All your Pokémon have fainted! Heal your team at the Pokémon Center first.".to_string());
            }
        }

        let chosen_move = player
            .party[0]
            .moves
            .get(move_index)
            .cloned()
            .ok_or_else(|| "Invalid move selected!".to_string())?;

        // Create temporary enemy BattlePokemon
        let mut enemy_battle_mon = BattlePokemon {
            id: encounter.pokemon.id,
            name: encounter.pokemon.name.clone(),
            level: encounter.level,
            current_hp: encounter.current_hp,
            max_hp: encounter.max_hp,
            stats: encounter.pokemon.stats.clone(),
            types: encounter.pokemon.types.clone(),
            abilities: encounter.pokemon.abilities.clone(),
            moves: encounter.moves.clone(),
            rarity: encounter.pokemon.rarity.clone(),
            sprite_url: encounter.pokemon.sprite_url.clone().unwrap_or_else(|| format!("/sprites/{}.png", encounter.pokemon.id)),
        };

        let mut logs = Vec::new();

        // 1. Player attacks wild Pokemon
        let (p_dmg, p_crit, p_eff, player_mon_name) = {
            let player_mon = &player.party[0];
            let (dmg, crit, eff) = calculate_battle_damage(player_mon, &enemy_battle_mon, &chosen_move);
            (dmg, crit, eff, player_mon.name.clone())
        };
        enemy_battle_mon.current_hp = enemy_battle_mon.current_hp.saturating_sub(p_dmg);
        encounter.current_hp = enemy_battle_mon.current_hp;

        let mut msg = format!("{} used {}!", player_mon_name, chosen_move.name);
        if p_crit {
            msg.push_str(" A critical hit!");
        }
        if p_eff >= 2.0 {
            msg.push_str(" It's super effective!");
        } else if p_eff > 0.0 && p_eff < 1.0 {
            msg.push_str(" It's not very effective...");
        } else if p_eff == 0.0 {
            msg.push_str(" It had no effect!");
        }
        logs.push(msg.clone());

        let enemy_fainted = enemy_battle_mon.current_hp == 0;
        let mut enemy_move_name = None;
        let mut enemy_move_type = None;
        let mut enemy_damage = 0;
        let mut enemy_crit = false;
        let mut enemy_eff = 1.0;
        let mut player_fainted = false;
        let mut battle_ended = false;
        let mut player_won = false;
        let mut reward_money = 0u64;
        let mut reward_xp = 0u32;

        if enemy_fainted {
            battle_ended = true;
            player_won = true;
            logs.push(format!("Wild {} fainted!", enemy_battle_mon.name));

            reward_xp = get_catch_xp_reward(&encounter.pokemon.rarity) * 2;
            reward_money = get_rarity_sell_price(&encounter.pokemon.rarity) / 2;

            player.xp += reward_xp as u64;
            player.currency += reward_money;
            logs.push(format!("Victory! Gained +{} XP and ₽{}.", reward_xp, reward_money));

            self.storage.clear_wild_encounter(username);
        } else {
            // 2. Wild Pokemon strikes back
            let mut rng = rand::thread_rng();
            let enemy_move = enemy_battle_mon.moves.choose(&mut rng).cloned().unwrap_or(BattleMove {
                name: "Tackle".to_string(),
                r#type: "Normal".to_string(),
                power: 40,
                accuracy: 100,
                category: "physical".to_string(),
            });
            enemy_move_name = Some(enemy_move.name.clone());
            enemy_move_type = Some(enemy_move.r#type.clone());

            let (e_dmg, e_crit, e_eff) = {
                let player_mon = &player.party[0];
                calculate_battle_damage(&enemy_battle_mon, player_mon, &enemy_move)
            };
            enemy_damage = e_dmg;
            enemy_crit = e_crit;
            enemy_eff = e_eff;

            player.party[0].current_hp = player.party[0].current_hp.saturating_sub(e_dmg);

            let mut emsg = format!("Wild {} used {}!", enemy_battle_mon.name, enemy_move.name);
            if e_crit {
                emsg.push_str(" Critical hit!");
            }
            if e_eff >= 2.0 {
                emsg.push_str(" Super effective!");
            }
            logs.push(emsg);

            if player.party[0].current_hp == 0 {
                let fainted_name = player.party[0].name.clone();
                if let Some(next_idx) = player.party.iter().position(|p| p.current_hp > 0) {
                    let next_mon = player.party.remove(next_idx);
                    player.party.insert(0, next_mon);
                    let new_leader_name = player.party[0].name.clone();
                    player.active_pokemon_id = Some(player.party[0].id);
                    logs.push(format!("{} fainted! Go, {}!", fainted_name, new_leader_name));
                    let _ = self.storage.save_wild_encounter(username, &encounter);
                } else {
                    player_fainted = true;
                    battle_ended = true;
                    logs.push("All your Pokémon have fainted! Head to the Pokémon Center to heal.".to_string());
                    self.storage.clear_wild_encounter(username);
                }
            } else {
                // Save updated encounter HP
                let _ = self.storage.save_wild_encounter(username, &encounter);
            }
        }

        let p_hp = player.party.first().map(|p| p.current_hp).unwrap_or(0);
        let p_max = player.party.first().map(|p| p.max_hp).unwrap_or(1);

        self.storage.save_player(&player).map_err(|e| e.to_string())?;

        Ok(BattleActionResult {
            player_move_name: chosen_move.name,
            player_move_type: Some(chosen_move.r#type),
            player_damage: p_dmg,
            player_crit: p_crit,
            player_effectiveness: p_eff,
            enemy_move_name,
            enemy_move_type,
            enemy_damage,
            enemy_crit,
            enemy_effectiveness: enemy_eff,
            enemy_current_hp: enemy_battle_mon.current_hp,
            enemy_max_hp: enemy_battle_mon.max_hp,
            player_current_hp: p_hp,
            player_max_hp: p_max,
            enemy_fainted,
            player_fainted,
            battle_ended,
            player_won,
            reward_money,
            reward_xp,
            badge_earned: None,
            message: msg,
            logs,
        })
    }

    pub fn flee(&self, username: &str) -> Result<String, String> {
        if let Some(wild) = self.storage.load_wild_encounter(username) {
            self.storage.clear_wild_encounter(username);
            Ok(format!("Got away safely from {}!", wild.pokemon.name))
        } else {
            Err("No active wild encounter to flee from.".to_string())
        }
    }

    pub fn heal_party(&self, username: &str) -> Result<String, String> {
        let mut player = self.storage.load_player(username);
        for p in &mut player.party {
            p.current_hp = p.max_hp;
        }
        self.storage.save_player(&player).map_err(|e| e.to_string())?;
        Ok("Nurse Joy restored your Pokémon team to full health!".to_string())
    }

    pub fn set_active_pokemon(&self, username: &str, pokemon_id: u32) -> Result<String, String> {
        let mut player = self.storage.load_player(username);
        if let Some(idx) = player.party.iter().position(|p| p.id == pokemon_id) {
            let selected = player.party.remove(idx);
            player.party.insert(0, selected);
            player.active_pokemon_id = Some(pokemon_id);
            self.storage.save_player(&player).map_err(|e| e.to_string())?;
            Ok(format!("{} is now leading your battle party!", player.party[0].name))
        } else {
            // Check if in pokedex
            let dex = self.storage.load_pokedex(username);
            if let Some(caught) = dex.iter().find(|p| p.id == pokemon_id) {
                if let Some(proto) = self.storage.pokemon_catalog.get(&caught.id) {
                    let lvl = get_player_level(player.xp).max(5);
                    let battle_mon = create_battle_pokemon(proto, lvl);
                    if player.party.len() >= 6 {
                        player.party.pop();
                    }
                    player.party.insert(0, battle_mon);
                    player.active_pokemon_id = Some(pokemon_id);
                    self.storage.save_player(&player).map_err(|e| e.to_string())?;
                    return Ok(format!("Added {} to your party as leader!", proto.name));
                }
            }
            Err(format!("Pokemon #{} not in your collection.", pokemon_id))
        }
    }

    pub fn get_arena_list(&self, username: &str) -> Vec<ArenaDivision> {
        let player = self.storage.load_player(username);
        let mut divisions = get_arena_divisions();
        for d in &mut divisions {
            if player.arena_badges.contains(&d.badge) {
                d.completed = true;
            }
        }
        divisions
    }

    pub fn start_arena_battle(&self, username: &str, arena_id: &str) -> Result<ActiveArenaBattle, String> {
        let divisions = get_arena_divisions();
        let div = divisions
            .into_iter()
            .find(|d| d.id == arena_id)
            .ok_or_else(|| format!("Arena division '{}' not found", arena_id))?;

        let mut player = self.storage.load_player(username);
        if player.party.is_empty() {
            return Err("You need at least one Pokémon in your party to enter the Arena!".to_string());
        }

        // Auto-switch to first conscious Pokémon if current lead is fainted
        if player.party[0].current_hp == 0 {
            if let Some(healthy_idx) = player.party.iter().position(|p| p.current_hp > 0) {
                let healthy_mon = player.party.remove(healthy_idx);
                player.party.insert(0, healthy_mon);
                player.active_pokemon_id = Some(player.party[0].id);
                let _ = self.storage.save_player(&player);
            } else {
                return Err("All your Pokémon have fainted! Visit the Pokémon Center to heal first.".to_string());
            }
        }

        let player_active = player
            .party
            .first()
            .cloned()
            .ok_or_else(|| "You need at least one Pokémon in your party to enter the Arena!".to_string())?;

        // Build enemy team
        let mut enemy_team = Vec::new();
        for pid in div.team_preview {
            if let Some(proto) = self.storage.pokemon_catalog.get(&pid) {
                let mon = create_battle_pokemon(proto, div.recommended_level);
                enemy_team.push(mon);
            }
        }

        if enemy_team.is_empty() {
            return Err("Failed to generate arena leader team".to_string());
        }

        let battle = ActiveArenaBattle {
            arena_id: div.id,
            leader_name: div.leader_name.clone(),
            leader_title: div.title,
            current_enemy_index: 0,
            enemy_team,
            player_active,
            log: vec![format!("{}: \"{}\"", div.leader_name, div.dialogue_intro)],
            is_over: false,
            player_won: false,
        };

        self.storage.save_arena_battle(username, &battle).map_err(|e| e.to_string())?;
        Ok(battle)
    }

    pub fn arena_attack(&self, username: &str, move_index: usize) -> Result<BattleActionResult, String> {
        let mut battle = self
            .storage
            .load_arena_battle(username)
            .ok_or_else(|| "No active arena battle found!".to_string())?;

        if battle.is_over {
            return Err("This arena match has already concluded.".to_string());
        }

        let mut player = self.storage.load_player(username);
        if player.party.is_empty() {
            return Err("No active Pokémon in party!".to_string());
        }

        // Auto-switch to first conscious Pokémon if current lead is fainted
        if player.party[0].current_hp == 0 {
            if let Some(healthy_idx) = player.party.iter().position(|p| p.current_hp > 0) {
                let healthy_mon = player.party.remove(healthy_idx);
                player.party.insert(0, healthy_mon);
                player.active_pokemon_id = Some(player.party[0].id);
                battle.player_active = player.party[0].clone();
                let _ = self.storage.save_player(&player);
                let _ = self.storage.save_arena_battle(username, &battle);
            } else {
                return Err("All your Pokémon have fainted! Visit the Pokémon Center to heal.".to_string());
            }
        }

        let chosen_move = player
            .party[0]
            .moves
            .get(move_index)
            .cloned()
            .ok_or_else(|| "Invalid move chosen!".to_string())?;

        let cur_idx = battle.current_enemy_index;
        let mut enemy_mon = battle
            .enemy_team
            .get(cur_idx)
            .cloned()
            .ok_or_else(|| "No current enemy Pokémon!".to_string())?;

        let mut logs = Vec::new();

        // 1. Player attacks
        let (p_dmg, p_crit, p_eff, player_mon_name) = {
            let player_mon = &player.party[0];
            let (dmg, crit, eff) = calculate_battle_damage(player_mon, &enemy_mon, &chosen_move);
            (dmg, crit, eff, player_mon.name.clone())
        };
        enemy_mon.current_hp = enemy_mon.current_hp.saturating_sub(p_dmg);
        battle.enemy_team[cur_idx].current_hp = enemy_mon.current_hp;

        let mut msg = format!("{} used {}!", player_mon_name, chosen_move.name);
        if p_crit { msg.push_str(" Critical hit!"); }
        if p_eff >= 2.0 { msg.push_str(" Super effective!"); }
        else if p_eff > 0.0 && p_eff < 1.0 { msg.push_str(" Not very effective..."); }
        logs.push(msg.clone());

        let enemy_fainted = enemy_mon.current_hp == 0;
        let mut enemy_move_name = None;
        let mut enemy_move_type = None;
        let mut enemy_damage = 0;
        let mut enemy_crit = false;
        let mut enemy_eff = 1.0;
        let mut player_fainted = false;
        let mut battle_ended = false;
        let mut player_won = false;
        let mut reward_money = 0u64;
        let mut reward_xp = 0u32;
        let mut badge_earned = None;

        if enemy_fainted {
            logs.push(format!("Opposing {} fainted!", enemy_mon.name));

            // Check next enemy
            if battle.current_enemy_index + 1 < battle.enemy_team.len() {
                battle.current_enemy_index += 1;
                let next_enemy_name = battle.enemy_team[battle.current_enemy_index].name.clone();
                logs.push(format!("{} sent out {}!", battle.leader_name, next_enemy_name));
                battle.player_active = player.party[0].clone();
                let _ = self.storage.save_arena_battle(username, &battle);
            } else {
                // Entire leader team defeated!
                battle_ended = true;
                player_won = true;
                battle.is_over = true;
                battle.player_won = true;

                let divisions = get_arena_divisions();
                if let Some(div) = divisions.into_iter().find(|d| d.id == battle.arena_id) {
                    reward_money = div.reward_money;
                    reward_xp = div.reward_xp;
                    player.currency += reward_money;
                    player.xp += reward_xp as u64;

                    if !player.arena_badges.contains(&div.badge) {
                        player.arena_badges.push(div.badge.clone());
                        badge_earned = Some(div.badge.clone());
                    }
                    player.stats.arena_victories += 1;
                    logs.push(format!("{}: \"{}\"", battle.leader_name, div.dialogue_defeat));
                    logs.push(format!("Victory! Claimed {} and ₽{} prize!", div.badge, reward_money));
                }
                self.storage.clear_arena_battle(username);
            }
        } else {
            // Enemy attacks
            let mut rng = rand::thread_rng();
            let enemy_move = enemy_mon.moves.choose(&mut rng).cloned().unwrap_or(BattleMove {
                name: "Tackle".to_string(),
                r#type: "Normal".to_string(),
                power: 40,
                accuracy: 100,
                category: "physical".to_string(),
            });
            enemy_move_name = Some(enemy_move.name.clone());
            enemy_move_type = Some(enemy_move.r#type.clone());

            let (e_dmg, e_crit, e_eff) = {
                let player_mon = &player.party[0];
                calculate_battle_damage(&enemy_mon, player_mon, &enemy_move)
            };
            enemy_damage = e_dmg;
            enemy_crit = e_crit;
            enemy_eff = e_eff;
            player.party[0].current_hp = player.party[0].current_hp.saturating_sub(e_dmg);

            let mut emsg = format!("{}'s {} used {}!", battle.leader_name, enemy_mon.name, enemy_move.name);
            if e_crit { emsg.push_str(" Critical hit!"); }
            if e_eff >= 2.0 { emsg.push_str(" Super effective!"); }
            logs.push(emsg);

            if player.party[0].current_hp == 0 {
                let fainted_name = player.party[0].name.clone();
                // Check if any remaining party member is conscious
                if let Some(next_idx) = player.party.iter().position(|p| p.current_hp > 0) {
                    let next_mon = player.party.remove(next_idx);
                    player.party.insert(0, next_mon);
                    let new_leader_name = player.party[0].name.clone();
                    player.active_pokemon_id = Some(player.party[0].id);
                    battle.player_active = player.party[0].clone();
                    logs.push(format!("Your {} fainted! Go, {}!", fainted_name, new_leader_name));
                    let _ = self.storage.save_arena_battle(username, &battle);
                } else {
                    player_fainted = true;
                    battle_ended = true;
                    battle.is_over = true;
                    logs.push(format!("Your {} fainted! All your Pokémon were defeated.", fainted_name));
                    self.storage.clear_arena_battle(username);
                }
            } else {
                battle.player_active = player.party[0].clone();
                let _ = self.storage.save_arena_battle(username, &battle);
            }
        }

        player.stats.arena_battles += 1;
        let p_hp = player.party.first().map(|p| p.current_hp).unwrap_or(0);
        let p_max = player.party.first().map(|p| p.max_hp).unwrap_or(1);
        let e_hp = enemy_mon.current_hp;
        let e_max = enemy_mon.max_hp;

        self.storage.save_player(&player).map_err(|e| e.to_string())?;

        Ok(BattleActionResult {
            player_move_name: chosen_move.name,
            player_move_type: Some(chosen_move.r#type),
            player_damage: p_dmg,
            player_crit: p_crit,
            player_effectiveness: p_eff,
            enemy_move_name,
            enemy_move_type,
            enemy_damage,
            enemy_crit,
            enemy_effectiveness: enemy_eff,
            enemy_current_hp: e_hp,
            enemy_max_hp: e_max,
            player_current_hp: p_hp,
            player_max_hp: p_max,
            enemy_fainted,
            player_fainted,
            battle_ended,
            player_won,
            reward_money,
            reward_xp,
            badge_earned,
            message: msg,
            logs,
        })
    }

    pub fn get_store_catalog(&self, username: &str) -> Vec<StoreItemInfo> {
        let player = self.storage.load_player(username);
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

    pub fn buy(&self, username: &str, item_input: &str, amount: u32) -> Result<BuyResult, String> {
        if amount == 0 {
            return Err("Amount must be greater than 0".to_string());
        }
        if amount > 999 {
            return Err("Maximum purchase limit is 999 at once".to_string());
        }

        let ball_id = canonicalize_ball(item_input)
            .ok_or_else(|| format!("Unknown store item '{}'", item_input))?;
        let spec = get_ball_spec(ball_id).unwrap();

        let mut player = self.storage.load_player(username);
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

    pub fn sell_one(&self, username: &str, pokemon_name: &str) -> Result<SellResult, String> {
        let mut dex = self.storage.load_pokedex(username);
        let mut player = self.storage.load_player(username);

        let lower = pokemon_name.trim().to_lowercase();
        let index = dex
            .iter()
            .position(|p| p.name.to_lowercase() == lower || p.id.to_string() == lower)
            .ok_or_else(|| format!("You do not have any '{}' in your Pokédex to sell.", pokemon_name))?;

        let sold = dex.remove(index);
        let price = get_rarity_sell_price(&sold.rarity);
        player.currency += price;

        self.storage.save_pokedex(username, &dex).map_err(|e| e.to_string())?;
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

    pub fn sell_all_by_rarity(&self, username: &str, rarity_input: &str) -> Result<SellResult, String> {
        let dex = self.storage.load_pokedex(username);
        let mut player = self.storage.load_player(username);

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
        self.storage.save_pokedex(username, &kept).map_err(|e| e.to_string())?;
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

    pub fn sell_duplicates(&self, username: &str) -> Result<SellResult, String> {
        let dex = self.storage.load_pokedex(username);
        let mut player = self.storage.load_player(username);

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
            kept.push(list.remove(0));
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

        self.storage.save_pokedex(username, &kept).map_err(|e| e.to_string())?;
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

    pub fn use_item(&self, username: &str, item_id: &str, target_idx: usize) -> Result<ItemUseResult, String> {
        let mut player = self.storage.load_player(username);
        let canonical_id = canonicalize_ball(item_id).unwrap_or(item_id);
        
        let owned = player.balls.get(canonical_id).copied().unwrap_or(0);
        if owned == 0 {
            return Err(format!("You don't have any '{}' in your bag!", item_id));
        }

        if player.party.is_empty() {
            return Err("Your party is empty!".to_string());
        }

        let idx = if target_idx < player.party.len() { target_idx } else { 0 };
        let mut healed_amount = 0u32;
        let mut new_level = None;
        let mut evolved_into = None;
        let message;

        match canonical_id {
            "potion" | "super_potion" | "hyper_potion" | "max_potion" | "full_restore" => {
                let target = &mut player.party[idx];
                if target.current_hp >= target.max_hp {
                    return Err(format!("{} is already at full HP!", target.name));
                }
                let heal_value = match canonical_id {
                    "potion" => 30,
                    "super_potion" => 70,
                    "hyper_potion" => 150,
                    _ => target.max_hp,
                };
                let prev_hp = target.current_hp;
                target.current_hp = (target.current_hp + heal_value).min(target.max_hp);
                healed_amount = target.current_hp - prev_hp;
                message = format!("Used {} on {}! Restored {} HP! (HP: {}/{})", 
                    canonical_id.replace('_', " ").to_uppercase(),
                    target.name,
                    healed_amount,
                    target.current_hp,
                    target.max_hp
                );
            }
            "antidote" | "burn_heal" | "paralyze_heal" | "awakening" => {
                let target = &player.party[idx];
                message = format!("Used {} on {}! Condition fully cured!", 
                    canonical_id.replace('_', " ").to_uppercase(),
                    target.name
                );
            }
            "exp_crystal_s" | "exp_crystal_m" => {
                let xp_gain = if canonical_id == "exp_crystal_s" { 250 } else { 1000 };
                player.xp += xp_gain;
                let target = &player.party[idx];
                message = format!("Used {} on {}! Granted +{} XP!", 
                    canonical_id.replace('_', " ").to_uppercase(),
                    target.name,
                    xp_gain
                );
            }
            "rare_candy" => {
                let target = &mut player.party[idx];
                target.level += 1;
                new_level = Some(target.level);
                target.stats.hp = (target.stats.hp as f64 * 1.05).round() as u32;
                target.stats.atk = (target.stats.atk as f64 * 1.05).round() as u32;
                target.stats.def = (target.stats.def as f64 * 1.05).round() as u32;
                target.stats.spa = (target.stats.spa as f64 * 1.05).round() as u32;
                target.stats.spd = (target.stats.spd as f64 * 1.05).round() as u32;
                target.stats.spe = (target.stats.spe as f64 * 1.05).round() as u32;
                target.max_hp = 20 + target.level * 5 + (target.stats.hp / 3);
                target.current_hp = target.max_hp;
                message = format!("Rare Candy raised {}'s level to Lv. {}! All stats boosted!", 
                    target.name,
                    target.level
                );
            }
            "thunder_stone" | "fire_stone" | "water_stone" | "leaf_stone" | "moon_stone" | "ice_stone" => {
                let target = &player.party[idx];
                let evolved_id = match (canonical_id, target.id) {
                    ("thunder_stone", 25) => Some(26), // Pikachu -> Raichu
                    ("thunder_stone", 133) => Some(135), // Eevee -> Jolteon
                    ("fire_stone", 37) => Some(38), // Vulpix -> Ninetales
                    ("fire_stone", 58) => Some(59), // Growlithe -> Arcanine
                    ("fire_stone", 133) => Some(136), // Eevee -> Flareon
                    ("water_stone", 61) => Some(62), // Poliwhirl -> Poliwrath
                    ("water_stone", 90) => Some(91), // Shellder -> Cloyster
                    ("water_stone", 120) => Some(121), // Staryu -> Starmie
                    ("water_stone", 133) => Some(134), // Eevee -> Vaporeon
                    ("leaf_stone", 44) => Some(45), // Gloom -> Vileplume
                    ("leaf_stone", 70) => Some(71), // Weepinbell -> Victreebel
                    ("leaf_stone", 102) => Some(103), // Exeggcute -> Exeggutor
                    ("moon_stone", 30) => Some(31), // Nidorina -> Nidoqueen
                    ("moon_stone", 33) => Some(34), // Nidorino -> Nidoking
                    ("moon_stone", 35) => Some(36), // Clefairy -> Clefable
                    ("moon_stone", 39) => Some(40), // Jigglypuff -> Wigglytuff
                    _ => None,
                };
                if let Some(new_id) = evolved_id {
                    let prev_name = target.name.clone();
                    let current_lvl = target.level;
                    if let Some(base_pkmn) = self.storage.pokemon_catalog.get(&new_id).cloned() {
                        let evolved = create_battle_pokemon(&base_pkmn, current_lvl);
                        let evolved_name = evolved.name.clone();
                        evolved_into = Some(evolved_name.clone());
                        player.party[idx] = evolved;
                        message = format!("What?! {} is evolving! Congratulations! Your Pokémon evolved into {}!", 
                            prev_name, evolved_name
                        );
                    } else {
                        return Err("Evolution data could not be loaded.".to_string());
                    }
                } else {
                    return Err(format!("The {} had no effect on {}!", canonical_id.replace('_', " "), target.name));
                }
            }
            _ => {
                return Err(format!("Item '{}' cannot be used right now.", canonical_id));
            }
        }

        // Decrement item count
        if let Some(entry) = player.balls.get_mut(canonical_id) {
            if *entry > 0 {
                *entry -= 1;
            }
        }
        let remaining = player.balls.get(canonical_id).copied().unwrap_or(0);

        self.storage.save_player(&player).map_err(|e| e.to_string())?;

        Ok(ItemUseResult {
            success: true,
            message,
            item: canonical_id.to_string(),
            items_remaining: remaining,
            healed_amount,
            new_level,
            evolved_into,
            party: player.party,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BattleMove, BattlePokemon, PlayerData, PokemonStats};

    #[test]
    fn test_default_player_starter_pikachu() {
        let player = PlayerData::default();
        assert_eq!(player.party.len(), 1, "New trainer should start with 1 starter");
        assert_eq!(player.party[0].id, 25, "Starter should be Pikachu (#25)");
        assert_eq!(player.party[0].level, 5, "Starter should be Level 5");
        assert!(player.party[0].current_hp > 0, "Starter should have full HP");
        assert_eq!(player.party[0].current_hp, player.party[0].max_hp);
        assert_eq!(player.active_pokemon_id, Some(25));
        assert_eq!(player.balls.get("poke_ball").copied().unwrap_or(0), 50);
    }

    #[test]
    fn test_auto_switch_logic() {
        let mut player = PlayerData::default();
        // Faint the starter
        player.party[0].current_hp = 0;

        // Add a second healthy Pokemon
        player.party.push(BattlePokemon {
            id: 1,
            name: "Bulbasaur".to_string(),
            level: 5,
            current_hp: 25,
            max_hp: 25,
            stats: PokemonStats::default(),
            types: vec!["Grass".to_string()],
            abilities: vec![],
            moves: vec![BattleMove {
                name: "Tackle".to_string(),
                r#type: "Normal".to_string(),
                power: 40,
                accuracy: 100,
                category: "physical".to_string(),
            }],
            rarity: "Common".to_string(),
            sprite_url: "/sprites/1.png".to_string(),
        });

        // Test switch: when party[0].current_hp == 0, find first healthy and promote
        assert_eq!(player.party[0].current_hp, 0);
        if let Some(healthy_idx) = player.party.iter().position(|p| p.current_hp > 0) {
            let healthy_mon = player.party.remove(healthy_idx);
            player.party.insert(0, healthy_mon);
            player.active_pokemon_id = Some(player.party[0].id);
        }

        assert_eq!(player.party[0].id, 1, "Bulbasaur should now be the lead");
        assert_eq!(player.party[0].current_hp, 25);
        assert_eq!(player.party[1].id, 25, "Fainted Pikachu should be in position 1");
    }

    #[test]
    fn test_store_items_and_catalog() {
        use crate::game::{STORE_BALLS, canonicalize_ball, get_ball_spec};
        assert!(STORE_BALLS.len() >= 25);
        assert_eq!(canonicalize_ball("potion"), Some("potion"));
        assert_eq!(canonicalize_ball("rare_candy"), Some("rare_candy"));
        assert_eq!(canonicalize_ball("thunder_stone"), Some("thunder_stone"));
        let pot = get_ball_spec("potion").expect("potion spec");
        assert_eq!(pot.category, "medicine");
        let candy = get_ball_spec("rare_candy").expect("rare candy spec");
        assert_eq!(candy.category, "crystals");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_item_usage_mechanics() {
        let storage = Arc::new(Storage::new().await.expect("Failed to initialize Storage"));
        let service = GameService::new(storage);
        let username = "TestTrainerItemUse";

        // Create player with damaged Pokémon
        let mut player = PlayerData::default();
        player.username = username.to_string();
        player.party[0].current_hp = 5;
        player.party[0].max_hp = 20;
        player.party[0].level = 5;
        player.balls.insert("potion".to_string(), 1);
        player.balls.insert("rare_candy".to_string(), 1);
        service.storage.save_player(&player).expect("save player");

        // Test Potion
        let heal_res = service.use_item(username, "potion", 0).expect("use potion");
        assert!(heal_res.success);
        assert_eq!(heal_res.party[0].current_hp, 20); // 5 + 30 capped at max_hp 20
        assert_eq!(heal_res.healed_amount, 15);
        assert_eq!(heal_res.items_remaining, 0);

        // Test Rare Candy
        let candy_res = service.use_item(username, "rare_candy", 0).expect("use rare candy");
        assert!(candy_res.success);
        assert_eq!(candy_res.new_level, Some(6));
        assert_eq!(candy_res.party[0].level, 6);
    }
}
