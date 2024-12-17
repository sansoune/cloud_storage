use rocket::{futures::channel, get, post, response, routes, serde::{json::Json, Deserialize, Serialize}, State};
use tonic::{transport::Channel, Request};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use common::brain_service;

use brain_service::{
    brain_service_client::BrainServiceClient,
    ComponentRegistration,
    ComponentType,
    MessageRouteRequest,
    MessageType,
    SystemStatusRequest,
};


struct ApiServer {
    client: BrainServiceClient<Channel>,
}

impl ApiServer {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let channel = Channel::from_static("http://[::1]:2207").connect().await?;
        let mut client = BrainServiceClient::new(channel);

        let request = Request::new(ComponentRegistration {
            component_id: "api_server".to_string(),
            component_type: ComponentType::Server as i32,
            ip_address: "127.0.0.1".to_string(),
            port: 8000
        });

        let response = client.register_component(request).await?;
        let response_inner = response.into_inner();

        if !response_inner.success {
            return Err(format!("Registration failed: {}", response_inner.error_message).into());
        }

        Ok(ApiServer { client })
    }


}

struct AppState {
    client: Arc<Mutex<ApiServer>>
}

#[get("/")]
fn index() -> &'static str {
    "hello world!"
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let client = ApiServer::new().await.expect("Failed to create brain service client");
    let app_state = AppState {
        client: Arc::new(Mutex::new(client))
    };

    rocket::build().manage(app_state).mount("/", routes![index]).launch().await?;
    Ok(())
}