use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use common::{config::ServiceConfig, http_client::HttpClient, HealthResponse, ServiceResult};
use models::{MessageRequest, MessageResponse};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, instrument};

#[derive(Clone)]
struct AppState {
    config: ServiceConfig,
    http_client: HttpClient,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    
    let config = ServiceConfig::from_env("channel-service", 8001);
    
    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .init();

    let state = AppState {
        config: config.clone(),
        http_client: HttpClient::new(),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/message", post(handle_message))
        .route("/api/v1/email", post(handle_email))
        .with_state(Arc::new(state))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("Channel Service listening on port {}", config.port);

    axum::serve(listener, app).await?;
    Ok(())
}

#[instrument]
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse::new("channel-service"))
}

#[instrument(skip(state))]
async fn handle_message(
    State(state): State<Arc<AppState>>,
    Json(request): Json<MessageRequest>,
) -> ServiceResult<Json<MessageResponse>> {
    info!("Received message: {:?}", request);

    // Forward to AI Agent Service for processing
    let ai_agent_url = format!("{}/api/v1/process", state.config.service_url("ai-agent"));
    
    let response = state
        .http_client
        .post::<MessageRequest, MessageResponse>(&ai_agent_url, &request)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    info!("AI Agent response: {:?}", response);
    Ok(Json(response))
}

#[instrument(skip(state))]
async fn handle_email(
    State(state): State<Arc<AppState>>,
    Json(mut request): Json<MessageRequest>,
) -> ServiceResult<Json<MessageResponse>> {
    info!("Received email: {:?}", request);

    // Set channel type to Email
    request.channel = models::MessageChannel::Email;

    // Forward to AI Agent Service for processing
    let ai_agent_url = format!("{}/api/v1/process", state.config.service_url("ai-agent"));
    
    let response = state
        .http_client
        .post::<MessageRequest, MessageResponse>(&ai_agent_url, &request)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    info!("AI Agent response for email: {:?}", response);
    Ok(Json(response))
}
