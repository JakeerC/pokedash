use chrono::Timelike;
use rand::Rng;
use crate::models::{ArenaDivision, BattleMove, BattlePokemon, Pokemon};

pub const HUNT_COOLDOWN_SECONDS: f64 = 15.0;

pub fn get_generation(pokemon_id: u32) -> u32 {
    if pokemon_id <= 151 { 1 }
    else if pokemon_id <= 251 { 2 }
    else if pokemon_id <= 386 { 3 }
    else if pokemon_id <= 493 { 4 }
    else if pokemon_id <= 649 { 5 }
    else if pokemon_id <= 721 { 6 }
    else if pokemon_id <= 809 { 7 }
    else if pokemon_id <= 905 { 8 }
    else { 9 }
}

pub fn get_rarity_sell_price(rarity: &str) -> u64 {
    match rarity.to_lowercase().as_str() {
        "common" => 150,
        "uncommon" => 300,
        "rare" => 800,
        "ultrarare" => 1500,
        "epic" => 3000,
        "legendary" => 10000,
        "mythical" => 25000,
        _ => 150,
    }
}

pub fn get_rarity_spawn_weights() -> &'static [(&'static str, f64)] {
    &[
        ("Common", 50.0),
        ("Uncommon", 28.0),
        ("Rare", 12.0),
        ("UltraRare", 6.0),
        ("Epic", 2.5),
        ("Legendary", 1.0),
        ("Mythical", 0.5),
    ]
}

pub fn get_base_catch_rate(rarity: &str) -> f64 {
    match rarity.to_lowercase().as_str() {
        "common" => 0.50,
        "uncommon" => 0.40,
        "rare" => 0.30,
        "ultrarare" => 0.20,
        "epic" => 0.10,
        "legendary" => 0.05,
        "mythical" => 0.02,
        _ => 0.10,
    }
}

pub fn get_catch_xp_reward(rarity: &str) -> u32 {
    match rarity.to_lowercase().as_str() {
        "common" => 10,
        "uncommon" => 20,
        "rare" => 50,
        "ultrarare" => 100,
        "epic" => 200,
        "legendary" => 500,
        "mythical" => 1000,
        _ => 10,
    }
}

pub const XP_REWARD_HUNT: u32 = 5;

pub struct BallSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub alias: &'static str,
    pub price: u64,
    pub unlock_level: u32,
    pub category: &'static str,
    pub description: &'static str,
}

pub const STORE_BALLS: &[BallSpec] = &[
    BallSpec { id: "poke_ball", name: "Poké Ball", alias: "pb", price: 50, unlock_level: 1, category: "balls", description: "Standard catch chance (1.0x)" },
    BallSpec { id: "great_ball", name: "Great Ball", alias: "gb", price: 150, unlock_level: 3, category: "balls", description: "Improved catch rate (1.5x)" },
    BallSpec { id: "ultra_ball", name: "Ultra Ball", alias: "ub", price: 300, unlock_level: 8, category: "balls", description: "High performance catch rate (2.0x)" },
    BallSpec { id: "net_ball", name: "Net Ball", alias: "net", price: 250, unlock_level: 5, category: "special", description: "3.0x on Water or Bug types" },
    BallSpec { id: "dive_ball", name: "Dive Ball", alias: "dive", price: 250, unlock_level: 5, category: "special", description: "3.5x on Water types" },
    BallSpec { id: "fast_ball", name: "Fast Ball", alias: "fast", price: 250, unlock_level: 5, category: "special", description: "2.0x if Speed >= 80, 3.0x if Speed >= 120" },
    BallSpec { id: "dusk_ball", name: "Dusk Ball", alias: "dusk", price: 250, unlock_level: 5, category: "special", description: "3.5x at night (18:00 - 06:00)" },
    BallSpec { id: "nest_ball", name: "Nest Ball", alias: "nest", price: 250, unlock_level: 10, category: "special", description: "2.0x if BST <= 400, 3.0x if BST <= 300" },
    BallSpec { id: "repeat_ball", name: "Repeat Ball", alias: "repeat", price: 250, unlock_level: 10, category: "special", description: "3.0x if species already caught before" },
    BallSpec { id: "quick_ball", name: "Quick Ball", alias: "quick", price: 400, unlock_level: 15, category: "special", description: "Flat 2.5x catch bonus" },
    BallSpec { id: "master_ball", name: "Master Ball", alias: "mb", price: 50000, unlock_level: 25, category: "master", description: "Guaranteed 100% catch rate" },
];

pub fn canonicalize_ball(name_or_alias: &str) -> Option<&'static str> {
    let lower = name_or_alias.trim().to_lowercase().replace('-', "_");
    for ball in STORE_BALLS {
        if ball.id == lower || ball.alias == lower || ball.id.replace('_', "") == lower {
            return Some(ball.id);
        }
    }
    None
}

pub fn get_ball_spec(ball_id: &str) -> Option<&'static BallSpec> {
    STORE_BALLS.iter().find(|b| b.id == ball_id)
}

pub fn get_player_level(xp: u64) -> u32 {
    let mut level = 1u32;
    while xp >= (100.0 * (level as f64).powf(1.5)) as u64 {
        level += 1;
    }
    level
}

pub fn get_level_xp_range(level: u32) -> (u64, u64) {
    let prev_xp = if level > 1 {
        (100.0 * ((level - 1) as f64).powf(1.5)) as u64
    } else {
        0
    };
    let next_xp = (100.0 * (level as f64).powf(1.5)) as u64;
    (prev_xp, next_xp)
}

pub fn get_level_discount(level: u32) -> f64 {
    (level as f64 * 0.005).min(0.15)
}

pub fn get_level_catch_bonus(level: u32) -> f64 {
    (level as f64 * 0.01).min(0.25)
}

pub fn calculate_catch_chance(
    pokemon: &Pokemon,
    ball_id: &str,
    player_level: u32,
    already_caught: bool,
    current_hp: u32,
    max_hp: u32,
) -> f64 {
    let base_rate = get_base_catch_rate(&pokemon.rarity);
    let mut multiplier = 1.0f64;

    let types_lower: Vec<String> = pokemon.types.iter().map(|t| t.to_lowercase()).collect();
    let speed = pokemon.stats.spe;
    let bst = pokemon.stats.total;

    match ball_id {
        "master_ball" => {
            return 1.0;
        }
        "ultra_ball" => {
            multiplier = 2.0;
        }
        "great_ball" => {
            multiplier = 1.5;
        }
        "net_ball" => {
            if types_lower.iter().any(|t| t == "water" || t == "bug") {
                multiplier = 3.0;
            }
        }
        "dive_ball" => {
            if types_lower.iter().any(|t| t == "water") {
                multiplier = 3.5;
            }
        }
        "fast_ball" => {
            if speed >= 120 {
                multiplier = 3.0;
            } else if speed >= 80 {
                multiplier = 2.0;
            }
        }
        "dusk_ball" => {
            let hour = chrono::Local::now().hour();
            if hour >= 18 || hour < 6 {
                multiplier = 3.5;
            }
        }
        "nest_ball" => {
            if bst <= 300 {
                multiplier = 3.0;
            } else if bst <= 400 {
                multiplier = 2.0;
            }
        }
        "repeat_ball" => {
            if already_caught {
                multiplier = 3.0;
            }
        }
        "quick_ball" => {
            multiplier = 2.5;
        }
        _ => {
            multiplier = 1.0;
        }
    }

    let level_bonus = get_level_catch_bonus(player_level);

    // HP Factor: Weakening the Pokémon in battle increases catch rate up to 3x!
    let hp_ratio = if max_hp > 0 {
        (current_hp as f64 / max_hp as f64).clamp(0.05, 1.0)
    } else {
        1.0
    };
    let hp_bonus = 1.0 + (1.0 - hp_ratio) * 2.0; // 1.0x at full HP -> ~2.8x at 10% HP

    let chance = base_rate * multiplier * (1.0 + level_bonus) * hp_bonus;
    chance.min(1.0)
}

// --- BATTLE MECHANICS & TYPE EFFECTIVENESS ---

pub fn get_single_type_effectiveness(attacker_type: &str, defender_type: &str) -> f64 {
    let a = attacker_type.to_lowercase();
    let d = defender_type.to_lowercase();

    match (a.as_str(), d.as_str()) {
        ("fire", "grass") | ("fire", "ice") | ("fire", "bug") | ("fire", "steel") => 2.0,
        ("fire", "fire") | ("fire", "water") | ("fire", "rock") | ("fire", "dragon") => 0.5,

        ("water", "fire") | ("water", "ground") | ("water", "rock") => 2.0,
        ("water", "water") | ("water", "grass") | ("water", "dragon") => 0.5,

        ("grass", "water") | ("grass", "ground") | ("grass", "rock") => 2.0,
        ("grass", "fire") | ("grass", "grass") | ("grass", "poison") | ("grass", "flying") | ("grass", "bug") | ("grass", "dragon") | ("grass", "steel") => 0.5,

        ("electric", "water") | ("electric", "flying") => 2.0,
        ("electric", "electric") | ("electric", "grass") | ("electric", "dragon") => 0.5,
        ("electric", "ground") => 0.0,

        ("ice", "grass") | ("ice", "ground") | ("ice", "flying") | ("ice", "dragon") => 2.0,
        ("ice", "fire") | ("ice", "water") | ("ice", "ice") | ("ice", "steel") => 0.5,

        ("fighting", "normal") | ("fighting", "ice") | ("fighting", "rock") | ("fighting", "dark") | ("fighting", "steel") => 2.0,
        ("fighting", "poison") | ("fighting", "flying") | ("fighting", "psychic") | ("fighting", "bug") | ("fighting", "fairy") => 0.5,
        ("fighting", "ghost") => 0.0,

        ("poison", "grass") | ("poison", "fairy") => 2.0,
        ("poison", "poison") | ("poison", "ground") | ("poison", "rock") | ("poison", "ghost") => 0.5,
        ("poison", "steel") => 0.0,

        ("ground", "fire") | ("ground", "electric") | ("ground", "poison") | ("ground", "rock") | ("ground", "steel") => 2.0,
        ("ground", "grass") | ("ground", "bug") => 0.5,
        ("ground", "flying") => 0.0,

        ("flying", "grass") | ("flying", "fighting") | ("flying", "bug") => 2.0,
        ("flying", "electric") | ("flying", "rock") | ("flying", "steel") => 0.5,

        ("psychic", "fighting") | ("psychic", "poison") => 2.0,
        ("psychic", "psychic") | ("psychic", "steel") => 0.5,
        ("psychic", "dark") => 0.0,

        ("bug", "grass") | ("bug", "psychic") | ("bug", "dark") => 2.0,
        ("bug", "fire") | ("bug", "fighting") | ("bug", "poison") | ("bug", "flying") | ("bug", "ghost") | ("bug", "steel") | ("bug", "fairy") => 0.5,

        ("rock", "fire") | ("rock", "ice") | ("rock", "flying") | ("rock", "bug") => 2.0,
        ("rock", "fighting") | ("rock", "ground") | ("rock", "steel") => 0.5,

        ("ghost", "psychic") | ("ghost", "ghost") => 2.0,
        ("ghost", "dark") => 0.5,
        ("ghost", "normal") => 0.0,

        ("dragon", "dragon") => 2.0,
        ("dragon", "steel") => 0.5,
        ("dragon", "fairy") => 0.0,

        ("steel", "ice") | ("steel", "rock") | ("steel", "fairy") => 2.0,
        ("steel", "fire") | ("steel", "water") | ("steel", "electric") | ("steel", "steel") => 0.5,

        ("dark", "psychic") | ("dark", "ghost") => 2.0,
        ("dark", "fighting") | ("dark", "dark") | ("dark", "fairy") => 0.5,

        ("fairy", "fighting") | ("fairy", "dragon") | ("fairy", "dark") => 2.0,
        ("fairy", "fire") | ("fairy", "poison") | ("fairy", "steel") => 0.5,

        ("normal", "rock") | ("normal", "steel") => 0.5,
        ("normal", "ghost") => 0.0,

        _ => 1.0,
    }
}

pub fn get_total_type_effectiveness(attacker_type: &str, defender_types: &[String]) -> f64 {
    let mut total = 1.0;
    for def in defender_types {
        total *= get_single_type_effectiveness(attacker_type, def);
    }
    total
}

pub fn generate_moves_for_pokemon(p: &Pokemon) -> Vec<BattleMove> {
    let mut moves = Vec::new();
    let primary_type = p.types.first().cloned().unwrap_or_else(|| "Normal".to_string());
    let secondary_type = p.types.get(1).cloned();

    // Move 1: Standard Quick Move
    moves.push(BattleMove {
        name: "Tackle".to_string(),
        r#type: "Normal".to_string(),
        power: 40,
        accuracy: 100,
        category: "physical".to_string(),
    });

    // Move 2: Primary STAB Move
    let (m2_name, m2_pwr, m2_cat) = match primary_type.to_lowercase().as_str() {
        "fire" => ("Flamethrower", 80, "special"),
        "water" => ("Surf", 80, "special"),
        "grass" => ("Energy Ball", 80, "special"),
        "electric" => ("Thunderbolt", 80, "special"),
        "ice" => ("Ice Beam", 80, "special"),
        "fighting" => ("Close Combat", 85, "physical"),
        "poison" => ("Sludge Bomb", 80, "special"),
        "ground" => ("Earthquake", 85, "physical"),
        "flying" => ("Air Slash", 75, "special"),
        "psychic" => ("Psychic", 80, "special"),
        "bug" => ("Bug Buzz", 80, "special"),
        "rock" => ("Rock Slide", 75, "physical"),
        "ghost" => ("Shadow Ball", 80, "special"),
        "dragon" => ("Dragon Claw", 80, "physical"),
        "dark" => ("Dark Pulse", 80, "special"),
        "steel" => ("Flash Cannon", 80, "special"),
        "fairy" => ("Moonblast", 85, "special"),
        _ => ("Body Slam", 75, "physical"),
    };
    moves.push(BattleMove {
        name: m2_name.to_string(),
        r#type: primary_type.clone(),
        power: m2_pwr,
        accuracy: 100,
        category: m2_cat.to_string(),
    });

    // Move 3: Secondary STAB or High-Power Attack
    if let Some(sec) = secondary_type {
        let (m3_name, m3_pwr, m3_cat) = match sec.to_lowercase().as_str() {
            "fire" => ("Fire Punch", 75, "physical"),
            "water" => ("Aqua Tail", 80, "physical"),
            "grass" => ("Seed Bomb", 80, "physical"),
            "electric" => ("Thunder Punch", 75, "physical"),
            "ice" => ("Ice Punch", 75, "physical"),
            "fighting" => ("Brick Break", 75, "physical"),
            "poison" => ("Poison Jab", 80, "physical"),
            "ground" => ("Bulldoze", 65, "physical"),
            "flying" => ("Brave Bird", 90, "physical"),
            "psychic" => ("Zen Headbutt", 80, "physical"),
            "bug" => ("X-Scissor", 80, "physical"),
            "rock" => ("Stone Edge", 85, "physical"),
            "ghost" => ("Shadow Claw", 75, "physical"),
            "dragon" => ("Outrage", 90, "physical"),
            "dark" => ("Crunch", 80, "physical"),
            "steel" => ("Iron Head", 80, "physical"),
            "fairy" => ("Play Rough", 85, "physical"),
            _ => ("Slash", 70, "physical"),
        };
        moves.push(BattleMove {
            name: m3_name.to_string(),
            r#type: sec,
            power: m3_pwr,
            accuracy: 95,
            category: m3_cat.to_string(),
        });
    } else {
        moves.push(BattleMove {
            name: "Slash".to_string(),
            r#type: "Normal".to_string(),
            power: 70,
            accuracy: 100,
            category: "physical".to_string(),
        });
    }

    // Move 4: Ultimate Finisher
    let (m4_name, m4_type, m4_pwr, m4_cat) = if p.stats.total >= 500 {
        ("Hyper Beam", "Normal", 120, "special")
    } else if p.stats.atk >= p.stats.spa {
        ("Giga Impact", "Normal", 110, "physical")
    } else {
        ("Swift", "Normal", 60, "special")
    };
    moves.push(BattleMove {
        name: m4_name.to_string(),
        r#type: m4_type.to_string(),
        power: m4_pwr,
        accuracy: 90,
        category: m4_cat.to_string(),
    });

    moves
}

pub fn create_battle_pokemon(p: &Pokemon, level: u32) -> BattlePokemon {
    let scale = (level as f64) / 50.0;
    let max_hp = ((p.stats.hp as f64 * scale) + 15.0).round() as u32;
    let moves = generate_moves_for_pokemon(p);

    BattlePokemon {
        id: p.id,
        name: p.name.clone(),
        level,
        current_hp: max_hp.max(20),
        max_hp: max_hp.max(20),
        stats: p.stats.clone(),
        types: p.types.clone(),
        abilities: p.abilities.clone(),
        moves,
        rarity: p.rarity.clone(),
        sprite_url: format!("/sprites/{}.png", p.id),
    }
}

pub fn calculate_battle_damage(
    attacker: &BattlePokemon,
    defender: &BattlePokemon,
    chosen_move: &BattleMove,
) -> (u32, bool, f64) {
    let mut rng = rand::thread_rng();

    // Critical Hit: 10% chance
    let is_crit = rng.gen_range(0.0..1.0) < 0.10;
    let crit_multiplier = if is_crit { 1.5 } else { 1.0 };

    // Type Effectiveness
    let effectiveness = get_total_type_effectiveness(&chosen_move.r#type, &defender.types);
    if effectiveness == 0.0 {
        return (0, false, 0.0);
    }

    // STAB (Same-Type Attack Bonus): 1.5x if move matches attacker's type
    let stab = if attacker.types.iter().any(|t| t.eq_ignore_ascii_case(&chosen_move.r#type)) {
        1.5
    } else {
        1.0
    };

    // Stat selection based on move category
    let (attack_val, defense_val) = if chosen_move.category == "special" {
        (attacker.stats.spa.max(20) as f64, defender.stats.spd.max(20) as f64)
    } else {
        (attacker.stats.atk.max(20) as f64, defender.stats.def.max(20) as f64)
    };

    // Authentic formula scaled to level
    let level_factor = (2.0 * attacker.level as f64 / 5.0) + 2.0;
    let base_damage = ((level_factor * chosen_move.power as f64 * (attack_val / defense_val)) / 50.0) + 2.0;
    let random_factor = rng.gen_range(0.85..1.00);

    let raw_damage = base_damage * crit_multiplier * effectiveness * stab * random_factor;
    let final_damage = raw_damage.round().max(1.0) as u32;

    (final_damage, is_crit, effectiveness)
}

// Predefined Champion & Gym Arena Divisions
pub fn get_arena_divisions() -> Vec<ArenaDivision> {
    vec![
        ArenaDivision {
            id: "pewter".to_string(),
            name: "Pewter Gym Arena".to_string(),
            leader_name: "Brock".to_string(),
            title: "Rock-Solid Trainer".to_string(),
            badge: "Boulder Badge".to_string(),
            badge_icon: "🪨".to_string(),
            recommended_level: 5,
            reward_money: 2500,
            reward_xp: 250,
            team_preview: vec![74, 95], // Geodude, Onix
            completed: false,
            dialogue_intro: "I'm Brock! My rock-hard determination will shatter your team!".to_string(),
            dialogue_defeat: "You overpowered my rock defense! Take the Boulder Badge!".to_string(),
        },
        ArenaDivision {
            id: "cerulean".to_string(),
            name: "Cerulean Gym Arena".to_string(),
            leader_name: "Misty".to_string(),
            title: "Tomboyish Mermaid".to_string(),
            badge: "Cascade Badge".to_string(),
            badge_icon: "💧".to_string(),
            recommended_level: 10,
            reward_money: 5000,
            reward_xp: 500,
            team_preview: vec![120, 121], // Staryu, Starmie
            completed: false,
            dialogue_intro: "My water Pokémon are sweet and deadly! Can you handle this current?".to_string(),
            dialogue_defeat: "Wow, you really washed away my tactics! Here is the Cascade Badge!".to_string(),
        },
        ArenaDivision {
            id: "vermilion".to_string(),
            name: "Vermilion Gym Arena".to_string(),
            leader_name: "Lt. Surge".to_string(),
            title: "The Lightning American".to_string(),
            badge: "Thunder Badge".to_string(),
            badge_icon: "⚡".to_string(),
            recommended_level: 15,
            reward_money: 8000,
            reward_xp: 800,
            team_preview: vec![100, 26], // Voltorb, Raichu
            completed: false,
            dialogue_intro: "Ten-hut! Electric shocks saved me in battle! Let's see your voltage!".to_string(),
            dialogue_defeat: "Now that was shocking! You've earned the Thunder Badge!".to_string(),
        },
        ArenaDivision {
            id: "celadon".to_string(),
            name: "Celadon Gym Arena".to_string(),
            leader_name: "Erika".to_string(),
            title: "Nature-Loving Maiden".to_string(),
            badge: "Rainbow Badge".to_string(),
            badge_icon: "🌸".to_string(),
            recommended_level: 20,
            reward_money: 12000,
            reward_xp: 1200,
            team_preview: vec![114, 45], // Tangela, Vileplume
            completed: false,
            dialogue_intro: "My fragrant flowers bloom with immense power in the sun!".to_string(),
            dialogue_defeat: "Such vibrant passion... Please accept the Rainbow Badge.".to_string(),
        },
        ArenaDivision {
            id: "saffron".to_string(),
            name: "Saffron Gym Arena".to_string(),
            leader_name: "Sabrina".to_string(),
            title: "Master of Psychic Power".to_string(),
            badge: "Marsh Badge".to_string(),
            badge_icon: "🔮".to_string(),
            recommended_level: 25,
            reward_money: 18000,
            reward_xp: 1800,
            team_preview: vec![64, 65], // Kadabra, Alakazam
            completed: false,
            dialogue_intro: "I foresaw your arrival. My psychic waves will overwhelm your mind!".to_string(),
            dialogue_defeat: "I did not foresee your remarkable victory! The Marsh Badge is yours.".to_string(),
        },
        ArenaDivision {
            id: "fuchsia".to_string(),
            name: "Fuchsia Gym Arena".to_string(),
            leader_name: "Koga".to_string(),
            title: "Poisonous Ninja Master".to_string(),
            badge: "Soul Badge".to_string(),
            badge_icon: "☣️".to_string(),
            recommended_level: 30,
            reward_money: 25000,
            reward_xp: 2500,
            team_preview: vec![49, 110], // Venomoth, Weezing
            completed: false,
            dialogue_intro: "A ninja strikes with unseen toxins! Can you endure my poisons?".to_string(),
            dialogue_defeat: "Your skill pierces the shadows. Take this Soul Badge!".to_string(),
        },
        ArenaDivision {
            id: "cinnabar".to_string(),
            name: "Cinnabar Gym Arena".to_string(),
            leader_name: "Blaine".to_string(),
            title: "Hot-Headed Quiz Master".to_string(),
            badge: "Volcano Badge".to_string(),
            badge_icon: "🔥".to_string(),
            recommended_level: 35,
            reward_money: 32000,
            reward_xp: 3200,
            team_preview: vec![78, 59], // Rapidash, Arcanine
            completed: false,
            dialogue_intro: "Hah! Better have burn heal! My fiery passion incinerates everything!".to_string(),
            dialogue_defeat: "I burned out completely! Magnificent! Take the Volcano Badge!".to_string(),
        },
        ArenaDivision {
            id: "viridian".to_string(),
            name: "Viridian Gym Arena".to_string(),
            leader_name: "Giovanni".to_string(),
            title: "Team Rocket Boss".to_string(),
            badge: "Earth Badge".to_string(),
            badge_icon: "🌍".to_string(),
            recommended_level: 40,
            reward_money: 45000,
            reward_xp: 4500,
            team_preview: vec![31, 34, 112], // Nidoqueen, Nidoking, Rhydon
            completed: false,
            dialogue_intro: "I am Giovanni, master of this Gym! I shall crush your ambition underfoot!".to_string(),
            dialogue_defeat: "Unbelievable... Defeated by a young prodigy... Take the Earth Badge!".to_string(),
        },
        ArenaDivision {
            id: "indigo".to_string(),
            name: "Indigo Plateau Arena".to_string(),
            leader_name: "Lance".to_string(),
            title: "Dragon Master Champion".to_string(),
            badge: "Dragon Crest".to_string(),
            badge_icon: "🐉".to_string(),
            recommended_level: 50,
            reward_money: 75000,
            reward_xp: 8000,
            team_preview: vec![130, 142, 149], // Gyarados, Aerodactyl, Dragonite
            completed: false,
            dialogue_intro: "I have trained with legendary dragons! None can withstand their roar!".to_string(),
            dialogue_defeat: "You have tamed the dragons themselves! You truly are a champion!".to_string(),
        },
        ArenaDivision {
            id: "battle_tower".to_string(),
            name: "Master Battle Tower".to_string(),
            leader_name: "Cynthia".to_string(),
            title: "Supreme Champion".to_string(),
            badge: "Master Trophy".to_string(),
            badge_icon: "🏆".to_string(),
            recommended_level: 60,
            reward_money: 100000,
            reward_xp: 15000,
            team_preview: vec![442, 448, 445], // Spiritomb, Lucario, Garchomp
            completed: false,
            dialogue_intro: "When every heart beats as one with Pokémon, victory follows! Let us battle!".to_string(),
            dialogue_defeat: "What an exhilarating clash! You are truly a Master Trainer!".to_string(),
        },
    ]
}
