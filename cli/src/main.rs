use clap::{Parser, Subcommand};
use tonic::{Request, transport::Channel};
use std::error::Error;
use base64::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use common::brain_service;

use brain_service::{
    brain_service_client::BrainServiceClient,
    ComponentRegistration,
    UnregistrationRequest,
    MessageRouteRequest,
    ComponentType,
    MessageType,
};

#[derive(Parser)]
#[command(name = "storage-cli")]
#[command(about = "Distributed Storage CLI", long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value = "[::1]:2207")]
    server_address: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Upload a file to storage
    Upload {
        #[arg(short, long)]
        file: PathBuf,
    },

    /// Download a file from storage
    Download {
        #[arg(short = 'i', long = "file-id")]
        file_id: Option<String>,

        #[arg(short = 'n', long = "file-name")]
        file_name: Option<String>,

        #[arg(short, long)]
        output: PathBuf,
    },

    /// List files in storage
    List,

    /// Delete a file from storage
    Delete {
        #[arg(short = 'i', long = "file-id")]
        file_id: Option<String>,

        #[arg(short = 'n', long = "file-name")]
        file_name: Option<String>,
    },
}

struct StorageCli {
    client: BrainServiceClient<Channel>,
    component_id: String,
}

impl StorageCli {
    async fn new(server_address: &str) -> Result<Self, Box<dyn Error>> {
        let component_id = format!("storage-cli-{}", Uuid::new_v4());

        let mut client = BrainServiceClient::connect(format!("http://{}", server_address)).await?;

        let request = Request::new(ComponentRegistration {
            component_id: component_id.clone(),
            component_type: ComponentType::Cli as i32,
            ip_address: "127.0.0.1".to_string(),
            port: 0,
        });

        let response = client.register_component(request).await?;
        let response_inner = response.into_inner();

        if !response_inner.success {
            return Err(format!("Registration failed: {}", response_inner.error_message).into());
        }

        println!("CLI registered with ID: {}", component_id);

        Ok(StorageCli {
            client,
            component_id,
        })
    }

    async fn unregister(&mut self) -> Result<(), Box<dyn Error>> {
        let request = Request::new(UnregistrationRequest {
            component_id: self.component_id.clone(),
        });

        let response = self.client.unregister_component(request).await?;
        let response_inner = response.into_inner();

        if response_inner.success {
            println!("CLI unregistered successfully");
        } else {
            println!("Failed to unregister CLI: {}", response_inner.error_message);
        }

        Ok(())
    }

    async fn send_storage_command(&mut self, command: String) -> Result<String, Box<dyn Error>> {
        let request = Request::new(MessageRouteRequest{
            source_component: self.component_id.clone(),
            destination_component: "brain".to_string(),
            payload: command.into_bytes(),
            message_type: MessageType::StorageRequest as i32,
        });

        let response = self.client.route_message(request).await?;
        let response_inner = response.into_inner();

        if response_inner.success {
            Ok(response_inner.error_message)
        } else {
            Err(response_inner.error_message.into())
        }
    }

    async fn upload_file(&mut self, file_path: &Path) -> Result<String, Box<dyn Error>> {
        if !file_path.exists() {
            return Err(format!("File not found: {}", file_path.display()).into());
        }

        let file_data = fs::read(&file_path)?;

        let filename = file_path.file_name().ok_or("Invalid filename")?.to_str().ok_or("Invalid filename")?;
        let encoded_data = BASE64_STANDARD.encode(&file_data);

        let command = format!("upload {} {}", filename, encoded_data);

        let result = self.send_storage_command(command).await?;
        
        Ok(result)
    }

    async fn download_file(&mut self, parameter_type: &str, parameter: String, output: PathBuf) -> Result<String, Box<dyn Error>> {
        let command = format!("download {} {}", parameter_type, parameter);
        let result = self.send_storage_command(command).await?;
        let decoded_data = BASE64_STANDARD.decode(&result)?;
        fs::write(&output, decoded_data)?;

        Ok(format!("File downloaded to {}", output.display()))
    }

    async fn delete_file(&mut self, parameter_type: &str, parameter: String)  -> Result<String, Box<dyn Error>> {
        let command = format!("delete {} {}", parameter_type, parameter);
        let result = self.send_storage_command(command).await?;

        Ok(result)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let mut storage_cli = StorageCli::new(&cli.server_address).await?;

    match cli.command {
        Commands::Upload { file } => {
            let result = storage_cli.upload_file(&file).await?;
            println!("{}", result);

        },
        Commands::Download { file_id, file_name, output } => {
            let result = match (file_id, file_name) {
                (Some(id), _) => storage_cli.download_file("id", id, output).await?,
                (None, Some(name)) => storage_cli.download_file("name", name, output).await?,
                _ => return Err("Either file ID or file name must be provided".into()),
            };
            println!("{}", result);
        },
        Commands::List => {
            let result = storage_cli.send_storage_command("list".to_string()).await?;
            println!("{}", result);
        },
        Commands::Delete { file_id, file_name } => {
            let result = match (file_id, file_name) {
                (Some(id), _) => storage_cli.delete_file("id", id).await?,
                (None, Some(name)) => storage_cli.delete_file("name", name).await?,
                _ => return Err("Either file ID or file name must be provided".into()),
            };
            println!("{}", result);
        },
    }
    

    storage_cli.unregister().await?;

    Ok(())
}
