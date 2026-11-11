mod cli;
mod game;
mod models;
mod server;
mod service;
mod storage;
mod supabase;

use std::sync::Arc;
use service::GameService;
use storage::Storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    supabase::load_dotenv();
    let storage = Arc::new(Storage::new().await?);
    let service = GameService::new(storage);
    cli::run_server(service).await?;
    Ok(())
}
