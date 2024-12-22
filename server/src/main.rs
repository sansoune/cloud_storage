use common::brain_service::{self, MessageRouteResponse, UnregistrationRequest};
use rocket::{
    get, post, routes,
    serde::{json::Json, Deserialize, Serialize},
    State,
};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{transport::Channel, Request};
use uuid::Uuid;

use brain_service::{
    brain_service_client::BrainServiceClient, ComponentRegistration, ComponentType,
    MessageRouteRequest, MessageType,
};

struct ApiServer {
    client: BrainServiceClient<Channel>,
    component_id: String,
}

impl ApiServer {
    async fn new() -> Result<Self, Box<dyn Error>> {
        let channel = Channel::from_static("http://[::1]:2207").connect().await?;
        let mut client = BrainServiceClient::new(channel);

        let component_id = "api_server".to_string();

        let request = Request::new(ComponentRegistration {
            component_id: component_id.clone(),
            component_type: ComponentType::Server as i32,
            ip_address: "127.0.0.1".to_string(),
            port: 8000,
        });

        let response = client.register_component(request).await?;
        let response_inner = response.into_inner();

        if !response_inner.success {
            return Err(format!("Registration failed: {}", response_inner.error_message).into());
        }

        Ok(ApiServer {
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
            println!("API Server unregistered successfully");
        } else {
            println!(
                "Failed to unregister API Server: {}",
                response_inner.error_message
            );
        }

        Ok(())
    }

    async fn route_message(
        &mut self,
        source: String,
        destination: &str,
        payload: String,
        message_type: MessageType,
    ) -> Result<MessageRouteResponse, Box<dyn Error>> {
        let request = Request::new(MessageRouteRequest {
            source_component: source,
            destination_component: destination.to_string(),
            payload: payload.into_bytes(),
            message_type: message_type as i32,
        });

        let response = self.client.route_message(request).await?;
        let response_inner = response.into_inner();

        Ok(response_inner)
    }
}

struct AppState {
    client: Arc<Mutex<ApiServer>>,
}

#[derive(Serialize, Deserialize)]
struct StorageUploadRequest {
    file_name: String,
    file_content: String, // base64 encoded
}

#[derive(Serialize, Deserialize)]
struct StorageResponse {
    success: bool,
    message: String,
}

#[get("/")]
fn index() -> &'static str {
    "hello world!"
}

#[get("/storage/list")]
async fn list_files(state: &State<AppState>) -> Json<StorageResponse> {
    let mut client = state.client.lock().await;

    let component_id = client.component_id.clone();

    match client
        .route_message(
            component_id,
            "brain",
            "list".to_string(),
            MessageType::StorageRequest,
        )
        .await
    {
        Ok(response) => Json(StorageResponse {
            success: response.success,
            message: response.error_message,
        }),
        Err(e) => Json(StorageResponse {
            success: false,
            message: format!("Error listing files: {}", e),
        }),
    }
}

#[post("/storage/upload", format = "json", data = "<upload_request>")]
async fn upload_file(state: &State<AppState>, upload_request: Json<StorageUploadRequest>) -> Json<StorageResponse> {
    let mut client = state.client.lock().await;

    let command = format!("upload {} {}", upload_request.file_name, upload_request.file_content);

    let component_id = client.component_id.clone();

    match client.route_message(component_id, "brain", command, MessageType::StorageRequest).await {
        Ok(response) => Json(StorageResponse {
            success: response.success,
            message: response.error_message,
        }),
        Err(e) => Json(StorageResponse {
            success: false,
            message: format!("Error uploading file: {}", e),
        })
    }
}

#[derive(Debug)]
enum Identifier {
    Id(String),
    Name(String),
}

impl<'r> rocket::request::FromParam<'r> for Identifier {
    type Error = &'static str;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        if let Some(name) = param.strip_prefix("name:") {
            Ok(Identifier::Name(name.to_string()))
        }else if let Some(id) = param.strip_prefix("id:") {
            Ok(Identifier::Id(id.to_string()))
        }else {
            Ok(Identifier::Id(param.to_string()))
        }
    }
}

#[get("/storage/download/<identifier>")]
async fn download_file(state: &State<AppState>, identifier: Identifier) -> Json<StorageResponse> {
    let mut client = state.client.lock().await;

    let command = match identifier {
        Identifier::Id(id) => format!("download id {}", id),
        Identifier::Name(name) => format!("download name {}", name),
    };

    let component_id = client.component_id.clone();

    match client.route_message(component_id, "brain", command, MessageType::StorageRequest).await {
        Ok(response) => Json(StorageResponse {
            success: response.success,
            message: response.error_message,
        }),
        Err(e) => Json(StorageResponse {
            success: false,
            message: format!("Error downloading file: {}", e),
        })
    }
}

#[post("/storage/delete/<identifier>")]
async fn delete_file(state: &State<AppState>, identifier: Identifier) -> Json<StorageResponse> {
    let mut client = state.client.lock().await;

    let command = match identifier {
        Identifier::Id(id) => format!("delete id {}", id),
        Identifier::Name(name) => format!("delete name {}", name),
    };

    let component_id = client.component_id.clone();

    match client.route_message(component_id, "brain", command, MessageType::StorageRequest).await {
        Ok(response) => Json(StorageResponse {
            success: response.success,
            message: response.error_message,
        }),
        Err(e) => Json(StorageResponse {
            success: false,
            message: format!("Error downloading file: {}", e),
        })
    }
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let client = ApiServer::new()
        .await
        .expect("Failed to create brain service client");
    let app_state = AppState {
        client: Arc::new(Mutex::new(client)),
    };

    let shutdown_state = app_state.client.clone();

    let rocket = rocket::build()
        .manage(app_state)
        .mount("/", routes![index, list_files, upload_file, download_file, delete_file])
        .attach(rocket::fairing::AdHoc::on_shutdown(
            "Unregister Component",
            move |_| {
                Box::pin(async move {
                    let mut client = shutdown_state.lock().await;
                    if let Err(e) = client.unregister().await {
                        eprintln!("Error during unregistration: {}", e);
                    } else {
                        println!("Clean shutdown: Unregistered successfully.");
                    }
                })
            },
        ));

    rocket.launch().await?;
    Ok(())
}
