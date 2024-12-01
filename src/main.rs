use clap::Parser;
use cloud_storage::Result;


mod storage_manager;
mod cli;


use storage_manager::StorageManager;
use cli::{Cli, execute_command};




#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    // let storage = DiskStorage::new("./storage").await?.with_encryption([0u8; 32]).with_cache(100).with_compression(true);

    let storage = StorageManager::new("./storage").await?;

    if let Some(command) = cli.command {
        execute_command(&storage, command).await?;
    }

    Ok(())
}