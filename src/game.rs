use chrono::Timelike;
use crate::models::Pokemon;

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
    // 0.5% discount per level up to 15% discount at level 30
    (level as f64 * 0.005).min(0.15)
}

pub fn get_level_catch_bonus(level: u32) -> f64 {
    // 1% bonus per level capped at 25% at lvl 25
    (level as f64 * 0.01).min(0.25)
}

pub fn calculate_catch_chance(
    pokemon: &Pokemon,
    ball_id: &str,
    player_level: u32,
    already_caught: bool,
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
            // Regular poke_ball
            multiplier = 1.0;
        }
    }

    let level_bonus = get_level_catch_bonus(player_level);
    let chance = base_rate * multiplier * (1.0 + level_bonus);
    chance.min(1.0)
}
