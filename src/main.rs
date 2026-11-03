mod cli;
mod game;
mod models;
mod server;
mod service;
mod storage;
mod tui;

use std::sync::Arc;
use service::GameService;
use storage::Storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let storage = Arc::new(Storage::new()?);
    let service = GameService::new(storage);
    cli::run_cli(service).await?;
    Ok(())
}
