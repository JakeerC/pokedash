use clap::{Args, Parser, Subcommand};
use colored::*;

use crate::game::{get_generation, get_player_level};
use crate::models::{format_number, CaughtPokemon, Pokemon};
use crate::server::{create_router, AppState};
use crate::service::GameService;
use crate::tui::{
    format_rarity, render_encounter_card, render_pokedex_table, render_stats_dashboard,
    render_store,
};

#[derive(Parser)]
#[command(name = "pokedash")]
#[command(about = "A Pokémon hunting and collecting game for Terminal and Web", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Search for a wild Pokémon to catch
    Hunt,

    /// Throw a Poké Ball to catch the wild Pokémon
    Catch {
        /// Ball type or alias (e.g. pb, gb, ub, net, dive, fast, dusk, nest, repeat, quick, mb)
        ball: String,
    },

    /// Let the wild Pokémon flee
    Flee,

    /// View your Pokédex collection
    #[command(alias = "dex")]
    Pokedex(PokedexArgs),

    /// Check your ball inventory and Poké Dollars balance
    #[command(alias = "inv")]
    Inventory,

    /// View trainer stats, level, achievements, and records
    #[command(alias = "s")]
    Stats,

    /// Visit the Poké Mart to buy and sell
    Store(StoreArgs),

    /// Start the HTTP backend server and Web UI
    Serve(ServeArgs),

    /// Start an interactive terminal game session
    Play,
}

#[derive(Args)]
pub struct PokedexArgs {
    /// Name or ID of a specific Pokémon to inspect
    pub pokemon_name: Option<String>,

    /// Filter by Generation (1-9)
    #[arg(short, long)]
    pub gen: Option<u32>,

    /// Filter by Type (e.g. Water, Fire, Dragon)
    #[arg(short, long)]
    pub r#type: Option<String>,

    /// Filter by Rarity (Common, Uncommon, Rare, UltraRare, Epic, Legendary, Mythical)
    #[arg(short, long)]
    pub rarity: Option<String>,

    /// Sort by: id, rarity, bst, recent
    #[arg(short, long, default_value = "id")]
    pub sort: String,

    /// Page number (default: 1)
    #[arg(short, long, default_value_t = 1)]
    pub page: usize,
}

#[derive(Args)]
pub struct StoreArgs {
    #[command(subcommand)]
    pub action: Option<StoreAction>,
}

#[derive(Subcommand)]
pub enum StoreAction {
    /// Buy Poké Balls from the store
    Buy {
        /// Ball type or alias (e.g. pb, gb, ub, net, dive, fast, dusk, nest, repeat, quick, mb)
        item: String,
        /// Number of balls to purchase (default: 1)
        #[arg(default_value_t = 1)]
        amount: u32,
    },
    /// Sell one caught Pokémon from your Pokédex
    Sell {
        /// Name of the Pokémon to sell
        pokemon_name: String,
    },
    /// Sell all Pokémon of a specific rarity tier
    Sellall {
        /// Rarity tier: Common, Uncommon, Rare, UltraRare, Epic, Legendary, Mythical
        rarity: String,
    },
    /// Sell duplicate Pokémon (keeps 1 of each caught species)
    #[command(name = "sell-dupes")]
    SellDupes,
}

#[derive(Args)]
pub struct ServeArgs {
    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    /// Host interface to bind to
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Automatically open in browser
    #[arg(long)]
    pub open: bool,
}

pub async fn run_cli(service: GameService) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Hunt) => {
            handle_hunt(&service);
        }
        Some(Commands::Catch { ball }) => {
            handle_catch(&service, &ball);
        }
        Some(Commands::Flee) => {
            match service.flee() {
                Ok(msg) => println!("{}", msg.yellow()),
                Err(e) => eprintln!("{}", e.red()),
            }
        }
        Some(Commands::Pokedex(args)) => {
            handle_pokedex(&service, args);
        }
        Some(Commands::Inventory) => {
            handle_inventory(&service);
        }
        Some(Commands::Stats) => {
            let player = service.get_player();
            render_stats_dashboard(&player);
        }
        Some(Commands::Store(args)) => {
            handle_store(&service, args);
        }
        Some(Commands::Serve(args)) => {
            handle_serve(service, args).await?;
        }
        Some(Commands::Play) => {
            handle_interactive_play(service).await?;
        }
        None => {
            println!("{}", "Welcome to PokéDash! Use --help to see commands, or 'pokedash play' for interactive mode, or 'pokedash serve' for web mode.".bold().cyan());
            handle_inventory(&service);
        }
    }

    Ok(())
}

fn handle_hunt(service: &GameService) {
    match service.hunt() {
        Ok(encounter) => {
            let dex = service.get_pokedex();
            let already_caught = dex.iter().any(|p| p.id == encounter.pokemon.id);
            render_encounter_card(&encounter.pokemon, already_caught);
            println!("  {}", "Use 'pokedash catch <ball>' (e.g. 'pokedash catch pb') to throw a ball!".bold().cyan());
        }
        Err(e) => {
            eprintln!("\n  {} {}\n", "⏳".yellow(), e.yellow());
        }
    }
}

fn handle_catch(service: &GameService, ball: &str) {
    match service.catch(ball) {
        Ok(res) => {
            if res.success {
                println!("\n  ✨ {}", res.message.bold().green());
                println!("  Caught: #{} {} ({})", res.pokemon.id, res.pokemon.name.bold(), format_rarity(&res.pokemon.rarity));
                println!("  Remaining {}s: {}", res.ball, res.balls_remaining.to_string().yellow());
                if res.leveled_up {
                    println!("\n  🎉🎉 LEVEL UP! You are now Trainer Level {}! 🎉🎉\n", res.new_level.to_string().bold().yellow());
                } else {
                    println!();
                }
            } else {
                println!("\n  ❌ {}", res.message.bold().red());
                println!("  Remaining {}s: {}\n", res.ball, res.balls_remaining.to_string().yellow());
            }
        }
        Err(e) => {
            eprintln!("\n  {} {}\n", "⚠️".red(), e.red());
        }
    }
}

fn handle_inventory(service: &GameService) {
    let player = service.get_player();
    let level = get_player_level(player.xp);

    println!("\n╭──────────────────────────────────────────────────────────────╮");
    println!("│                       TRAINER BAG                            │");
    println!("╰──────────────────────────────────────────────────────────────╯");
    println!("  Trainer: {}  •  Level: {}", player.username.bold().cyan(), level.to_string().bold());
    println!("  Balance: {}", format!("₽ {}", format_number(player.currency)).bold().yellow());
    println!("  XP: {}", format!("{}", format_number(player.xp)).bold().cyan());
    println!("\n  {}", "Poké Balls in Bag:".bold().white());

    let mut has_any = false;
    for spec in crate::game::STORE_BALLS {
        let count = player.balls.get(spec.id).copied().unwrap_or(0);
        if count > 0 {
            println!("    {:<15} (alias: {:<4}): {:>4}x", spec.name, spec.alias.cyan(), count.to_string().bold().yellow());
            has_any = true;
        }
    }

    if !has_any {
        println!("    (Your bag has no Poké Balls! Buy some with 'pokedash store buy pb')");
    }
    println!();
}

fn handle_pokedex(service: &GameService, args: PokedexArgs) {
    let mut dex = service.get_pokedex();
    let total_caught = dex.len();
    let mut unique_ids = std::collections::HashSet::new();
    for p in &dex {
        unique_ids.insert(p.id);
    }
    let total_unique = unique_ids.len();

    if let Some(target) = args.pokemon_name {
        let lower = target.trim().to_lowercase();
        let matches: Vec<&CaughtPokemon> = dex
            .iter()
            .filter(|p| p.name.to_lowercase() == lower || p.id.to_string() == lower)
            .collect();

        if matches.is_empty() {
            println!("\n  No caught Pokémon matching '{}' found in your Pokédex.\n", target);
            return;
        }

        let first = matches[0];
        let p = &Pokemon {
            id: first.id,
            name: first.name.clone(),
            types: first.types.clone(),
            abilities: first.abilities.clone(),
            stats: first.stats.clone(),
            rarity: first.rarity.clone(),
            rarity_score: first.rarity_score,
        };
        render_encounter_card(p, true);
        println!("  Total owned of this species: {}\n", matches.len().to_string().bold().yellow());
        return;
    }

    // Filter
    if let Some(gen) = args.gen {
        dex.retain(|p| get_generation(p.id) == gen);
    }
    if let Some(ref t) = args.r#type {
        let lower = t.trim().to_lowercase();
        dex.retain(|p| p.types.iter().any(|item| item.to_lowercase() == lower));
    }
    if let Some(ref r) = args.rarity {
        let lower = r.trim().to_lowercase();
        dex.retain(|p| p.rarity.to_lowercase() == lower);
    }

    // Sort
    match args.sort.as_str() {
        "rarity" => {
            let rarity_rank = |r: &str| match r.to_lowercase().as_str() {
                "mythical" => 7,
                "legendary" => 6,
                "epic" => 5,
                "ultrarare" => 4,
                "rare" => 3,
                "uncommon" => 2,
                _ => 1,
            };
            dex.sort_by(|a, b| rarity_rank(&b.rarity).cmp(&rarity_rank(&a.rarity)));
        }
        "bst" => dex.sort_by(|a, b| b.stats.total.cmp(&a.stats.total)),
        "recent" => dex.sort_by(|a, b| b.caught_at.partial_cmp(&a.caught_at).unwrap_or(std::cmp::Ordering::Equal)),
        _ => dex.sort_by_key(|p| p.id),
    }

    let items_per_page = 20;
    let total_filtered = dex.len();
    let total_pages = (total_filtered + items_per_page - 1).max(1) / items_per_page;
    let page = args.page.clamp(1, total_pages.max(1));

    let start = (page - 1) * items_per_page;
    let page_items: Vec<CaughtPokemon> = dex.into_iter().skip(start).take(items_per_page).collect();

    render_pokedex_table(&page_items, total_unique, total_caught, page, total_pages);
}

fn handle_store(service: &GameService, args: StoreArgs) {
    match args.action {
        Some(StoreAction::Buy { item, amount }) => {
            match service.buy(&item, amount) {
                Ok(res) => println!("\n  ✅ {}\n", res.message.bold().green()),
                Err(e) => eprintln!("\n  ❌ {}\n", e.red()),
            }
        }
        Some(StoreAction::Sell { pokemon_name }) => {
            match service.sell_one(&pokemon_name) {
                Ok(res) => println!("\n  ✅ {}\n", res.message.bold().green()),
                Err(e) => eprintln!("\n  ❌ {}\n", e.red()),
            }
        }
        Some(StoreAction::Sellall { rarity }) => {
            match service.sell_all_by_rarity(&rarity) {
                Ok(res) => println!("\n  ✅ {}\n", res.message.bold().green()),
                Err(e) => eprintln!("\n  ❌ {}\n", e.red()),
            }
        }
        Some(StoreAction::SellDupes) => {
            match service.sell_duplicates() {
                Ok(res) => println!("\n  ✅ {}\n", res.message.bold().green()),
                Err(e) => eprintln!("\n  ❌ {}\n", e.red()),
            }
        }
        None => {
            let player = service.get_player();
            render_store(&player);
        }
    }
}

async fn handle_serve(service: GameService, args: ServeArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app_state = AppState {
        service: service.clone(),
    };
    let app = create_router(app_state);

    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("\n╭──────────────────────────────────────────────────────────────╮");
    println!("│             🚀 POKÉDASH WEB & API SERVER ACTIVE             │");
    println!("╰──────────────────────────────────────────────────────────────╯");
    println!("  Web Application:  {}", format!("http://localhost:{}", args.port).bold().green());
    println!("  API Base URL:     {}", format!("http://localhost:{}/api", args.port).bold().cyan());
    println!("  Terminal & Web synced via live backend storage (~/.pokedash)");
    println!("  Press Ctrl+C to stop.\n");

    if args.open {
        let _ = open_browser(&format!("http://localhost:{}", args.port));
    }

    axum::serve(listener, app).await?;
    Ok(())
}

fn open_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(url).spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd").args(["/C", "start", url]).spawn()?;
    }
    Ok(())
}

async fn handle_interactive_play(service: GameService) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{}", "\n🎮 ENTERING POKÉDASH INTERACTIVE TERMINAL MODE 🎮".bold().cyan());

    loop {
        let options = vec![
            "1. ⚔️  Hunt Wild Pokémon",
            "2. ⚾ Throw Poké Ball (Catch)",
            "3. 📖 View Pokédex",
            "4. 🏪 Poké Mart (Store)",
            "5. 🎒 Trainer Inventory",
            "6. 📊 Profile & Statistics",
            "7. 🌐 Launch Web Server",
            "8. 🚪 Exit",
        ];

        let ans = match inquire::Select::new("Choose an action:", options).prompt() {
            Ok(choice) => choice,
            Err(_) => break,
        };

        if ans.starts_with("1.") {
            handle_hunt(&service);
        } else if ans.starts_with("2.") {
            let wild = service.get_wild();
            if wild.is_none() {
                println!("\n  {} Use 'Hunt Wild Pokémon' first!\n", "⚠️".yellow());
                continue;
            }

            let player = service.get_player();
            let mut available_balls = Vec::new();
            for b in crate::game::STORE_BALLS {
                let count = player.balls.get(b.id).copied().unwrap_or(0);
                if count > 0 {
                    available_balls.push(format!("{} ({}x)", b.name, count));
                }
            }

            if available_balls.is_empty() {
                println!("\n  {} You have no Poké Balls! Buy some at the Poké Mart.\n", "❌".red());
                continue;
            }

            let ball_choice = match inquire::Select::new("Select a Poké Ball to throw:", available_balls).prompt() {
                Ok(c) => c,
                Err(_) => continue,
            };

            let ball_name = ball_choice.split(" (").next().unwrap();
            handle_catch(&service, ball_name);
        } else if ans.starts_with("3.") {
            handle_pokedex(
                &service,
                PokedexArgs {
                    pokemon_name: None,
                    gen: None,
                    r#type: None,
                    rarity: None,
                    sort: "id".to_string(),
                    page: 1,
                },
            );
        } else if ans.starts_with("4.") {
            let store_opts = vec![
                "View Mart Catalog",
                "Buy Poké Balls",
                "Sell Duplicates",
                "Back to Main Menu",
            ];
            let sub = match inquire::Select::new("Poké Mart:", store_opts).prompt() {
                Ok(s) => s,
                Err(_) => continue,
            };

            if sub == "View Mart Catalog" {
                let player = service.get_player();
                render_store(&player);
            } else if sub == "Buy Poké Balls" {
                let ball_opts: Vec<String> = crate::game::STORE_BALLS
                    .iter()
                    .map(|b| format!("{} (₽{})", b.name, b.price))
                    .collect();
                if let Ok(choice) = inquire::Select::new("Select ball to buy:", ball_opts).prompt() {
                    let bname = choice.split(" (").next().unwrap();
                    let qty = match inquire::CustomType::<u32>::new("Quantity to buy:").with_default(1).prompt() {
                        Ok(q) => q,
                        Err(_) => 1,
                    };
                    handle_store(&service, StoreArgs { action: Some(StoreAction::Buy { item: bname.to_string(), amount: qty }) });
                }
            } else if sub == "Sell Duplicates" {
                handle_store(&service, StoreArgs { action: Some(StoreAction::SellDupes) });
            }
        } else if ans.starts_with("5.") {
            handle_inventory(&service);
        } else if ans.starts_with("6.") {
            let player = service.get_player();
            render_stats_dashboard(&player);
        } else if ans.starts_with("7.") {
            println!("\nStarting server on http://localhost:8080...");
            handle_serve(service.clone(), ServeArgs { port: 8080, host: "0.0.0.0".to_string(), open: true }).await?;
            break;
        } else if ans.starts_with("8.") {
            println!("\nGoodbye, Trainer! Happy catching! ⚡\n");
            break;
        }
    }

    Ok(())
}
