use colored::*;
use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, ContentArrangement, Table};

use crate::models::{format_number, CaughtPokemon, PlayerData, Pokemon};
use crate::game::{
    get_generation, get_level_discount, get_level_xp_range, get_player_level, STORE_BALLS,
};

pub fn format_type(type_name: &str) -> ColoredString {
    match type_name.to_lowercase().as_str() {
        "fire" => type_name.red().bold(),
        "water" => type_name.blue().bold(),
        "grass" => type_name.green().bold(),
        "electric" => type_name.yellow().bold(),
        "ice" => type_name.cyan().bold(),
        "fighting" => type_name.bright_red().bold(),
        "poison" => type_name.magenta().bold(),
        "ground" => type_name.yellow(),
        "flying" => type_name.bright_cyan(),
        "psychic" => type_name.bright_magenta().bold(),
        "bug" => type_name.bright_green(),
        "rock" => type_name.truecolor(180, 160, 50),
        "ghost" => type_name.purple(),
        "dragon" => type_name.bright_blue().bold(),
        "dark" => type_name.truecolor(100, 100, 100).bold(),
        "steel" => type_name.white().bold(),
        "fairy" => type_name.bright_purple(),
        _ => type_name.normal(),
    }
}

pub fn format_rarity(rarity: &str) -> ColoredString {
    match rarity.to_lowercase().as_str() {
        "common" => rarity.white(),
        "uncommon" => rarity.green().bold(),
        "rare" => rarity.blue().bold(),
        "ultrarare" => rarity.cyan().bold(),
        "epic" => rarity.magenta().bold(),
        "legendary" => rarity.yellow().bold(),
        "mythical" => rarity.bright_purple().bold(),
        _ => rarity.normal(),
    }
}

pub fn render_encounter_card(pokemon: &Pokemon, already_caught: bool) {
    let title = format!("⚔️  WILD {} APPEARED!", pokemon.name.to_uppercase()).bold().yellow();
    println!("\n╭──────────────────────────────────────────────────────────────╮");
    println!("│ {:<60} │", title);
    println!("├──────────────────────────────────────────────────────────────┤");

    let types_str = pokemon
        .types
        .iter()
        .map(|t| format!("{}", format_type(t)))
        .collect::<Vec<_>>()
        .join(" / ");

    let abilities_str = pokemon.abilities.join(", ");
    let caught_badge = if already_caught {
        " [CAUGHT IN DEX]".bright_green()
    } else {
        " [NEW SPECIES! ✨]".bright_yellow().bold()
    };

    println!(
        "│ {:<12} #{:<4} {:<24} {:<10} │",
        "Species:", pokemon.id, pokemon.name.bold(), caught_badge
    );
    println!("│ {:<12} {:<48} │", "Rarity:", format_rarity(&pokemon.rarity));
    println!("│ {:<12} {:<48} │", "Type:", types_str);
    println!("│ {:<12} {:<48} │", "Abilities:", abilities_str.cyan());
    println!("├──────────────────────────────────────────────────────────────┤");

    // Stat bars
    println!("│ Base Stats:                                                  │");
    render_stat_row("HP", pokemon.stats.hp, 255);
    render_stat_row("Attack", pokemon.stats.atk, 190);
    render_stat_row("Defense", pokemon.stats.def, 230);
    render_stat_row("Sp. Atk", pokemon.stats.spa, 194);
    render_stat_row("Sp. Def", pokemon.stats.spd, 230);
    render_stat_row("Speed", pokemon.stats.spe, 200);
    println!("│                                                              │");
    println!(
        "│ Base Stat Total: {:<5}      Generation: {:<17} │",
        pokemon.stats.total.to_string().bold().yellow(),
        get_generation(pokemon.id).to_string().bold()
    );
    println!("╰──────────────────────────────────────────────────────────────╯\n");
}

fn render_stat_row(label: &str, value: u32, max: u32) {
    let bar_len = 24;
    let filled = ((value as f64 / max as f64) * bar_len as f64).round() as usize;
    let filled = filled.min(bar_len);
    let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);

    let colored_bar = if value >= 100 {
        bar.bright_green()
    } else if value >= 70 {
        bar.yellow()
    } else {
        bar.cyan()
    };

    println!(
        "│   {:<8} {:>3} [{}] {:<13} │",
        label, value, colored_bar, ""
    );
}

pub fn render_stats_dashboard(player: &PlayerData) {
    let level = get_player_level(player.xp);
    let (prev_xp, next_xp) = get_level_xp_range(level);
    let current_xp = player.xp;
    let xp_in_level = current_xp.saturating_sub(prev_xp);
    let xp_needed = next_xp.saturating_sub(prev_xp);
    let xp_pct = if xp_needed > 0 {
        (xp_in_level as f64 / xp_needed as f64) * 100.0
    } else {
        0.0
    };

    let total_hunts = player.stats.total_hunts;
    let success = player.stats.successful_catches;
    let failed = player.stats.failed_catches;
    let catch_rate = if total_hunts > 0 {
        (success as f64 / total_hunts as f64) * 100.0
    } else {
        0.0
    };

    let unique = player.stats.lifetime_unique_pids.len();
    let col_pct = (unique as f64 / 1025.0) * 100.0;
    let discount = (get_level_discount(level) * 100.0).round() as u32;

    println!();
    println!("  {}", player.username.bold().cyan());
    println!(
        "  {} {}   •   {}   •   {} to Level {}",
        "Level".white(),
        level.to_string().bold(),
        format!("₽ {}", format_number(player.currency)).bold().yellow(),
        format!("{:.1}%", xp_pct).cyan(),
        (level + 1).to_string().bold()
    );
    println!("  Store Discount: {}%", discount.to_string().bold().green());
    println!();

    println!("  {}", "✧ COLLECTION ✧".bold().green());
    println!(
        "  Caught: {:<8} Unique: {:<8} Pokédex: {} ({:.1}%)",
        player.stats.total_lifetime_caught,
        unique,
        format!("{}/1025", unique).bold(),
        col_pct
    );
    println!();

    println!("  {}", "✧ RECORD ✧".bold().cyan());
    println!(
        "  Hunts: {:<9} Catch Rate: {:.1}%",
        total_hunts, catch_rate
    );
    println!(
        "  Success: {:<7} Failed: {}",
        success.to_string().green(),
        failed.to_string().red()
    );
    println!();

    println!("  {}", "✧ RARITY DISTRIBUTION ✧".bold().magenta());
    let r = &player.stats.lifetime_rarity;
    println!(
        "  Common: {:<6} Uncommon: {:<6} Rare: {:<6} UltraRare: {}",
        r.get("common").unwrap_or(&0),
        r.get("uncommon").unwrap_or(&0),
        r.get("rare").unwrap_or(&0),
        r.get("ultrarare").unwrap_or(&0)
    );
    println!(
        "  Epic: {:<8} Legendary: {:<6} Mythical: {}",
        r.get("epic").unwrap_or(&0),
        r.get("legendary").unwrap_or(&0),
        r.get("mythical").unwrap_or(&0)
    );
    println!();

    println!("  {}", "✧ INVENTORY (BALLS) ✧".bold().yellow());
    let mut ball_strs = Vec::new();
    for spec in STORE_BALLS {
        let count = player.balls.get(spec.id).copied().unwrap_or(0);
        if count > 0 {
            ball_strs.push(format!("{}: {}", spec.name, count.to_string().bold().yellow()));
        }
    }
    if ball_strs.is_empty() {
        println!("  Empty bag. Visit the Pokémart to buy Poké Balls!");
    } else {
        for chunk in ball_strs.chunks(3) {
            println!("  {}", chunk.join("    "));
        }
    }
    println!();
}

pub fn render_pokedex_table(
    pokemon_list: &[CaughtPokemon],
    total_unique: usize,
    total_caught: usize,
    page: usize,
    total_pages: usize,
) {
    println!("\n╭─────────────────────────────────────────────────────────────────────────────╮");
    let header = format!("YOUR POKÉDEX  •  {}/1025 Unique  •  {} Total Caught", total_unique, total_caught);
    println!("│ {:^75} │", header.bold().cyan());
    println!("╰─────────────────────────────────────────────────────────────────────────────╯");

    if pokemon_list.is_empty() {
        println!("\n  No Pokémon match your filters or your Pokédex is empty.\n");
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("#").add_attribute(Attribute::Bold),
            Cell::new("Name").add_attribute(Attribute::Bold),
            Cell::new("Type").add_attribute(Attribute::Bold),
            Cell::new("Rarity").add_attribute(Attribute::Bold),
            Cell::new("BST").add_attribute(Attribute::Bold),
            Cell::new("Gen").add_attribute(Attribute::Bold),
        ]);

    for p in pokemon_list {
        let types = p.types.join("/");
        table.add_row(vec![
            Cell::new(format!("{:03}", p.id)),
            Cell::new(&p.name).fg(Color::Cyan),
            Cell::new(types),
            Cell::new(&p.rarity),
            Cell::new(p.stats.total.to_string()).fg(Color::Yellow),
            Cell::new(get_generation(p.id).to_string()),
        ]);
    }

    println!("{table}");
    println!(
        "  Showing page {} of {}. Use '--page <N>' to navigate.\n",
        page, total_pages
    );
}

pub fn render_store(player: &PlayerData) {
    let level = get_player_level(player.xp);
    let discount = get_level_discount(level);

    println!("\n╭──────────────────────────────────────────────────────────────╮");
    println!("│                      POKÉ MART STORE                         │");
    println!("╰──────────────────────────────────────────────────────────────╯");
    println!(
        "  Trainer Level: {}  •  Balance: {}  •  Discount: {}%",
        level.to_string().bold(),
        format!("₽ {}", format_number(player.currency)).bold().yellow(),
        ((discount * 100.0).round() as u32).to_string().bold().green()
    );
    println!();

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_header(vec![
            Cell::new("Ball Item").add_attribute(Attribute::Bold),
            Cell::new("Alias").add_attribute(Attribute::Bold),
            Cell::new("Price").add_attribute(Attribute::Bold),
            Cell::new("Owned").add_attribute(Attribute::Bold),
            Cell::new("Req. Lv").add_attribute(Attribute::Bold),
            Cell::new("Special Effect / Multiplier").add_attribute(Attribute::Bold),
        ]);

    for spec in STORE_BALLS {
        let discounted = (spec.price as f64 * (1.0 - discount)).round() as u64;
        let owned = player.balls.get(spec.id).copied().unwrap_or(0);
        let unlocked = level >= spec.unlock_level;

        let req_cell = if unlocked {
            Cell::new(format!("Lv {}", spec.unlock_level)).fg(Color::Green)
        } else {
            Cell::new(format!("Lv {}", spec.unlock_level)).fg(Color::Red)
        };

        let price_cell = if discount > 0.0 {
            Cell::new(format!("₽{} (₽{})", discounted, spec.price)).fg(Color::Yellow)
        } else {
            Cell::new(format!("₽{}", spec.price)).fg(Color::Yellow)
        };

        table.add_row(vec![
            Cell::new(spec.name).fg(if unlocked { Color::White } else { Color::DarkGrey }),
            Cell::new(spec.alias),
            price_cell,
            Cell::new(owned.to_string()),
            req_cell,
            Cell::new(spec.description),
        ]);
    }

    println!("{table}");
    println!("  {}", "SELL PRICES:".bold().magenta());
    println!("  Common: ₽150  |  Uncommon: ₽300  |  Rare: ₽800  |  UltraRare: ₽1,500");
    println!("  Epic: ₽3,000  |  Legendary: ₽10,000  |  Mythical: ₽25,000");
    println!();
    println!("  Commands:");
    println!("    pokedash store buy <ball_or_alias> [amount]");
    println!("    pokedash store sell <pokemon_name>");
    println!("    pokedash store sellall <rarity>");
    println!("    pokedash store sell-dupes\n");
}
