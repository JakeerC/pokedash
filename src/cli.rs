use clap::Parser;
use colored::*;

use crate::server::{create_router, AppState};
use crate::service::GameService;

#[derive(Parser, Debug)]
#[command(name = "pokedash")]
#[command(about = "PokéDash — Pokémon Hunting & Collecting Web Game", long_about = None)]
pub struct CliArgs {
    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    /// Host interface to bind to
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Automatically open in default browser
    #[arg(long)]
    pub open: bool,
}

pub async fn run_server(service: GameService) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = CliArgs::parse();

    let app_state = AppState {
        service: service.clone(),
    };
    let app = create_router(app_state);

    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("\n╭──────────────────────────────────────────────────────────────╮");
    println!("│             🚀 POKÉDASH WEB SERVER ACTIVE                    │");
    println!("╰──────────────────────────────────────────────────────────────╯");
    println!("  🎮 Web Game:       {}", format!("http://localhost:{}", args.port).bold().green());
    println!("  📡 REST API:       {}", format!("http://localhost:{}/api", args.port).bold().cyan());
    if let Some(supabase) = &service.storage.supabase {
        println!("  💾 Persistent DB:  {} ({})", "Supabase DB".bold().green(), supabase.config.url.cyan());
        println!("  👤 Player Account: {}", supabase.config.username.bold().yellow());
    } else {
        println!("  💾 Persistent DB:  ~/.pokedash (local fallback)");
    }
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
