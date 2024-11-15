use cloud_storage::{storage::disk::{DiskStorage, StorageBackend}, Result};
use clap::{Parser, Subcommand};
use std::{collections::HashMap, path::PathBuf};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "cloud-storage")]
#[command(about = "Cloud Storage CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Upload {
        #[arg(short, long)]
        file: PathBuf,
    },
    Download {
        #[arg(short = 'i', long = "file-id")]
        file_id: Option<String>, // Optional file_id
        #[arg(short = 'n', long = "file-name")]
        file_name: Option<String>, // Optional file_name
        #[arg(short, long)]
        output: PathBuf,
    },
    Delete {
        #[arg(short = 'i', long = "file-id")]
        file_id: Option<String>, // Optional file_id
        #[arg(short = 'n', long = "file-name")]
        file_name: Option<String>, // Optional file_name
    },
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let storage = DiskStorage::new("./storage").await?.with_encryption([0u8; 32]).with_cache(100);

    match cli.command {
        Commands::Upload { file } => {
            println!("Uploading file: {:?}", file);
            let data = tokio::fs::read(&file).await?;
            let filename = file.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
            let metadata = storage.store_file(&filename, &data).await?;

            println!("File uploaded successfully!");
            println!("File ID: {}", metadata.id);
            println!("Size: {} bytes", metadata.size);
            println!("Checksum: {}", metadata.checksum);
        }
        Commands::Download { file_id, file_name, output } => {

            let id = if let Some(id_str) = file_id {
                Uuid::parse_str(&id_str).map_err(|e| cloud_storage::StorageError::Storage(format!("Invalid UUID: {}", e)))?
            } else if let Some(name) = file_name {
                // Look up the UUID by name in the index file
                let index_path = PathBuf::from("./storage/name_to_id.json");

                if !index_path.exists() {
                    return Err(cloud_storage::StorageError::NotFound("Index file not found".to_string()));
                }

                // Load and parse the index file
                let content = tokio::fs::read_to_string(&index_path).await?;
                let index: HashMap<String, Uuid> = serde_json::from_str(&content)
                    .map_err(|e| cloud_storage::StorageError::Storage(format!("Failed to parse index: {}", e)))?;
                
                // Retrieve the UUID corresponding to the file name
                index.get(&name)
                    .cloned()
                    .ok_or_else(|| cloud_storage::StorageError::NotFound(format!("File name '{}' not found", name)))?
            } else {
                return Err(cloud_storage::StorageError::Storage("Either file_id or file_name must be provided".to_string()));
            };

            println!("Downloading file: {} to {:?}", id, output);
            // Get the file data
            let data = storage.get_file(&id).await?;
            
            // Create parent directories if they don't exist
            if let Some(parent) = output.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            
            // Write the file to the output path
            tokio::fs::write(&output, data).await?;
            
            println!("File downloaded successfully to: {:?}", output);
        }
        Commands::List => {
            println!("Listing files:");
            
            // Get all files
            let files = storage.list_files().await?;
            
            if files.is_empty() {
                println!("No files found.");
                return Ok(());
            }
            
            // Print file information in a formatted way
            println!("\n{:<36} | {:<20} | {:<10} | {}", "ID", "Name", "Size (bytes)", "Created At");
            println!("{:-<80}", "");
            
            for metadata in files {
                println!("{:<36} | {:<20} | {:<10} | {}", 
                    metadata.id,
                    metadata.name,
                    metadata.size,
                    metadata.created_at.format("%Y-%m-%d %H:%M:%S")
                );
            }
        }
        Commands::Delete { file_id, file_name } => {
            let id = if let Some(id_str) = file_id {
                Uuid::parse_str(&id_str).map_err(|e| cloud_storage::StorageError::Storage(format!("Invalid UUID: {}", e)))?
            } else if let Some(name) = file_name {
                // Look up the UUID by name in the index file
                let index_path = PathBuf::from("./storage/name_to_id.json");

                if !index_path.exists() {
                    return Err(cloud_storage::StorageError::NotFound("Index file not found".to_string()));
                }

                // Load and parse the index file
                let content = tokio::fs::read_to_string(&index_path).await?;
                let index: HashMap<String, Uuid> = serde_json::from_str(&content)
                    .map_err(|e| cloud_storage::StorageError::Storage(format!("Failed to parse index: {}", e)))?;
                
                // Retrieve the UUID corresponding to the file name
                index.get(&name)
                    .cloned()
                    .ok_or_else(|| cloud_storage::StorageError::NotFound(format!("File name '{}' not found", name)))?
            } else {
                return Err(cloud_storage::StorageError::Storage("Either file_id or file_name must be provided".to_string()));
            };

            println!("Deleting file: {}", id);
            match storage.delete_file(&id).await {
                Ok(()) => println!("File deleted successfully!"),
                Err(cloud_storage::StorageError::NotFound(_)) => println!("File not found."),
                Err(e) => println!("Error deleting file: {}", e),
            }
        }
    }

    Ok(())
}