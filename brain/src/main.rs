use std::{collections::HashMap, error::Error, path::PathBuf, sync::Arc};

use base64::Engine;
use brain::managers::storage_manager::StorageManager;
use tokio::sync::Mutex;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{info, warn};
use common::brain_service::{self, MessageType};


use brain_service::{
    brain_service_server::{BrainService, BrainServiceServer},
    ComponentRegistration, ComponentStatus, ComponentType, MessageRouteRequest,
    MessageRouteResponse, RegistrationResponse, SystemStatusRequest, SystemStatusResponse,
    UnregistrationRequest, UnregistrationResponse, ComponentInfo, SystemHealth,
};
use uuid::Uuid;

#[derive(Clone)]
struct RegisteredComponent {
    id: String,
    component_type: ComponentType,
    ip_address: String,
    port: i32,
    status: ComponentStatus,
}

// Brain service state
#[derive(Default)]
struct BrainServiceState {
    system_id: String,
    components: HashMap<String, RegisteredComponent>,
}

// #[derive(Default)]
struct BrainServiceImpl {
    state: Arc<Mutex<BrainServiceState>>,
    storage: Arc<StorageManager>,
}

impl BrainServiceImpl {
    async fn new() -> Result<Self, Box<dyn Error>> {
        let storage_manager = StorageManager::new("./storage").await?;
        Ok(Self {
            state: Arc::new(Mutex::new(BrainServiceState::default())),
            storage: Arc::new(storage_manager),
        })
    }
}

#[tonic::async_trait]
impl BrainService for BrainServiceImpl {
    async fn register_component(
        &self,
        request: Request<ComponentRegistration>,
    ) -> Result<Response<RegistrationResponse>, Status> {
        let registration = request.into_inner();
        let mut state = self.state.lock().await;

        if state.system_id.is_empty() {
            state.system_id = Uuid::new_v4().to_string();
        }

        if state.components.contains_key(&registration.component_id) {
            return Err(Status::already_exists("component already exists"));
        }

        let component_type = ComponentType::try_from(registration.component_type).map_err(|_| Status::invalid_argument("Invalid component type"))?;

        let new_component = RegisteredComponent {
            id: registration.component_id.clone(),
            component_type: component_type,
            ip_address: registration.ip_address,
            port: registration.port,
            status: ComponentStatus::Running,
        };

        state
            .components
            .insert(registration.component_id.clone(), new_component);

        info!(
            "Registered component: {} (Type: {:?})",
            registration.component_id, registration.component_type
        );

        Ok(Response::new(RegistrationResponse {
            success: true,
            system_id: state.system_id.clone(),
            error_message: String::new(),
        }))
    }

    async fn unregister_component(
        &self,
        request: Request<UnregistrationRequest>,
    ) -> Result<Response<UnregistrationResponse>, Status> {
        let unregistration = request.into_inner();
        let mut state = self.state.lock().await;

        // Remove the component
        match state.components.remove(&unregistration.component_id) {
            Some(_) => {
                info!(
                    "Unregistered component: {}", 
                    unregistration.component_id
                );
                Ok(Response::new(UnregistrationResponse {
                    success: true,
                    error_message: String::new(),
                }))
            }
            None => {
                warn!(
                    "Attempted to unregister non-existent component: {}", 
                    unregistration.component_id
                );
                Err(Status::not_found("Component not found"))
            }
        }
    }

    async fn route_message(
        &self,
        request: Request<MessageRouteRequest>,
    ) -> Result<Response<MessageRouteResponse>, Status> {
        let message = request.into_inner();
        let state = self.state.lock().await;

        info!(message.destination_component);

        // Validate source and destination components
        if !state.components.contains_key(&message.source_component) {
            return Err(Status::not_found("Source component not registered"));
        }

        if message.destination_component == "brain" {
            if message.message_type == MessageType::StorageRequest as i32 {
                let storage_response = self.handle_storage_message(&message).await?;
                return Ok(Response::new(storage_response));
            }
        }

        if !state.components.contains_key(&message.destination_component) | (message.destination_component != "brain") {
            return Err(Status::not_found("Destination component not registered"));
        }

        // In a real implementation, this would actually route the message
        // Here we're just simulating the routing
        info!(
            "Routing message from {} to {}", 
            message.source_component, 
            message.destination_component
        );

        Ok(Response::new(MessageRouteResponse {
            success: true,
            error_message: String::new(),
        }))
    }

    async fn get_system_status(
        &self,
        _request: Request<SystemStatusRequest>,
    ) -> Result<Response<SystemStatusResponse>, Status> {
        let state = self.state.lock().await;

        // Convert internal components to protobuf ComponentInfo
        let registered_components: Vec<ComponentInfo> = state.components
            .values()
            .map(|comp| ComponentInfo {
                component_id: comp.id.clone(),
                component_type: comp.component_type as i32,
                ip_address: comp.ip_address.clone(),
                port: comp.port,
                status: comp.status as i32,
            })
            .collect();

        // Determine overall system health
        let overall_health = match registered_components.len() {
            0 => SystemHealth::Critical,
            1..=2 => SystemHealth::Degraded,
            _ => SystemHealth::Healthy,
        };

        Ok(Response::new(SystemStatusResponse {
            system_id: state.system_id.clone(),
            registered_components,
            overall_health: overall_health as i32,
        }))
    }
}

impl  BrainServiceImpl {
    async fn handle_storage_message(&self, message: &MessageRouteRequest,) -> Result<MessageRouteResponse, Status> {
        let payload = &message.payload;
        let command = String::from_utf8(payload.to_vec()).map_err(|_| Status::invalid_argument("Invalid payload"))?;
        let parts: Vec<&str> = command.splitn(3, ' ').collect();
        let operation = parts[0];
        println!("{}", operation);

        let mut response = MessageRouteResponse{
            success: true,
            error_message: String::new(),
        };

        match (operation, parts.get(1).copied(), parts.get(2).copied()) {
            ("list", None, None) => {
                match self.storage.list_files().await {
                    Ok(files) => {
                        let file_list: Vec<String> = files.iter().map(|f| format!("{}: {}", f.id, f.name)).collect();
                        response.error_message = file_list.join("\n");
                    }
                    Err(e) => {
                        response.success = false;
                        response.error_message = format!("List failed: {}", e);
                    }
                }
            }
            ("upload", Some(file_name), Some(data)) => {
                match base64::prelude::BASE64_STANDARD.decode(data) {
                    Ok(file_content) => {
                        match self.storage.upload_file(file_name, &file_content).await {
                            Ok(file_id) => {
                                response.error_message = format!("File uploaded successfully. File ID: {}", file_id.id);
                            }
                            Err(e) => {
                                response.success = false;
                                response.error_message = format!("Upload failed: {}", e);
                            }
                        }
                    }
                    Err(_) => {
                        response.success = false;
                        response.error_message = "Invalid base64 content".to_string();
                    }
                }
            }
            ("download", Some(param_type), Some(param)) => {
                match param_type {
                    "id" => {
                        match self.storage.download_file(&Uuid::parse_str(param).unwrap()).await {
                            Ok(file_contents) => {
                                response.error_message = base64::prelude::BASE64_STANDARD.encode(&file_contents);
                            }
                            Err(e) => {
                                response.success = false;
                                response.error_message = format!("Download failed: {}", e);
                            }
                        }
                    }
                    "name" => {
                        let index_path = PathBuf::from("./storage/name_to_id.json");
                        if !index_path.exists() {
                            return Err(Status::not_found("index file not found"));
                        }
                        
                        let content = tokio::fs::read_to_string(&index_path).await?;
                        let index: HashMap<String, Uuid> = serde_json::from_str(&content).map_err(|e:  serde_json::Error| Status::not_found(format!("failed to parse index {}", e))).unwrap();
                        let id = index.get(param).cloned().ok_or_else(|| Status::not_found(format!("file {} not found", param)))?;

                        match self.storage.download_file(&id).await {
                            Ok(file_contents) => {
                                response.error_message = base64::prelude::BASE64_STANDARD.encode(&file_contents);
                            }
                            Err(e) => {
                                response.success = false;
                                response.error_message = format!("Download failed: {}", e);
                            }
                        }
                    }
                    _ => {
                        response.success = false;
                        response.error_message = "Invalid download identifier type".to_string();
                    }
                }
            }
            ("delete", Some(param_type), Some(param)) => {
                match param_type {
                    "id" => {
                        match self.storage.delete_file(&Uuid::parse_str(param).unwrap()).await {
                            Ok(_) => {
                                response.error_message = format!("File with ID {} deleted", param);
                            }
                            Err(e) => {
                                response.success = false;
                                response.error_message = format!("Delete failed: {}", e);
                            }
                        }
                    }
                    "name" => {
                        let index_path = PathBuf::from("./storage/name_to_id.json");
                        if !index_path.exists() {
                            return Err(Status::not_found("index file not found"));
                        }
                        
                        let content = tokio::fs::read_to_string(&index_path).await?;
                        let index: HashMap<String, Uuid> = serde_json::from_str(&content).map_err(|e:  serde_json::Error| Status::not_found(format!("failed to parse index {}", e))).unwrap();
                        let id = index.get(param).cloned().ok_or_else(|| Status::not_found(format!("file {} not found", param)))?;

                        match self.storage.delete_file(&id).await {
                            Ok(_) => {
                                response.error_message = format!("File with ID {} deleted", id);
                            }
                            Err(e) => {
                                response.success = false;
                                response.error_message = format!("Delete failed: {}", e);
                            }
                        }
                    }
                    _ => {
                        response.success = false;
                        response.error_message = "Invalid download identifier type".to_string();
                    }
                }
            }
            _ => return Err(Status::invalid_argument("Invalid storage operation")),
        }

        Ok(response)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let addr = "[::1]:2207".parse().unwrap();

    let brain_service = BrainServiceImpl::new().await?;
    info!("Brain service starting on {}", addr);
    let reflection = tonic_reflection::server::Builder::configure().register_encoded_file_descriptor_set(brain_service::FILE_DESCRIPTOR_SET).build_v1()?;
    Server::builder()
    .add_service(reflection)
    .add_service(BrainServiceServer::new(brain_service)).serve(addr).await?;

    
    Ok(())
}
