use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
    Router,
};
use common::{config::ServiceConfig, http_client::HttpClient, HealthResponse, ServiceResult};
use models::{
    Case, CaseStatus, ConversationEntry, CreateCaseRequest, MessageSender, 
    Priority, UpdateCaseRequest, CaseWorkflow, WorkflowStep, StepStatus
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, instrument};
use uuid::Uuid;
use chrono::Utc;

#[derive(Clone)]
struct AppState {
    config: ServiceConfig,
    http_client: HttpClient,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    
    let config = ServiceConfig::from_env("case-management-service", 8002);
    
    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .init();

    let state = AppState {
        config: config.clone(),
        http_client: HttpClient::new(),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/cases", post(create_case))
        .route("/api/v1/cases/:id", get(get_case))
        .route("/api/v1/cases/:id/state", put(update_case_state))
        .route("/api/v1/cases/:id/history", get(get_conversation_history))
        .route("/api/v1/cases/:id/history", post(add_conversation_entry))
        .route("/api/v1/cases/:id/workflow", get(get_case_workflow))
        .route("/api/v1/cases/:id/workflow", put(update_case_workflow))
        .with_state(Arc::new(state))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("Case Management Service listening on port {}", config.port);

    axum::serve(listener, app).await?;
    Ok(())
}

#[instrument]
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse::new("case-management-service"))
}

#[instrument(skip(state))]
async fn create_case(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateCaseRequest>,
) -> ServiceResult<Json<Case>> {
    info!("Creating new case: {:?}", request);

    let case = Case {
        id: Uuid::new_v4(),
        title: request.title,
        description: request.description,
        status: CaseStatus::Open,
        priority: request.priority,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        assigned_to: request.assigned_to,
        metadata: serde_json::json!({}),
    };

    // Forward to persistence service
    let persistence_url = format!("{}/api/v1/cases", state.config.service_url("persistence"));
    let saved_case = state
        .http_client
        .post::<Case, Case>(&persistence_url, &case)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    info!("Case created with ID: {}", saved_case.id);
    Ok(Json(saved_case))
}

#[instrument(skip(state))]
async fn get_case(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ServiceResult<Json<Case>> {
    info!("Getting case: {}", id);

    let persistence_url = format!("{}/api/v1/cases/{}", state.config.service_url("persistence"), id);
    let case = state
        .http_client
        .get::<Case>(&persistence_url)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    Ok(Json(case))
}

#[instrument(skip(state))]
async fn update_case_state(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateCaseRequest>,
) -> ServiceResult<Json<Case>> {
    info!("Updating case state: {} with {:?}", id, request);

    let persistence_url = format!("{}/api/v1/cases/{}", state.config.service_url("persistence"), id);
    let updated_case = state
        .http_client
        .put::<UpdateCaseRequest, Case>(&persistence_url, &request)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    Ok(Json(updated_case))
}

#[instrument(skip(state))]
async fn get_conversation_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ServiceResult<Json<Vec<ConversationEntry>>> {
    info!("Getting conversation history for case: {}", id);

    let persistence_url = format!("{}/api/v1/cases/{}/history", state.config.service_url("persistence"), id);
    let history = state
        .http_client
        .get::<Vec<ConversationEntry>>(&persistence_url)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    Ok(Json(history))
}

#[instrument(skip(state))]
async fn add_conversation_entry(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(mut entry): Json<ConversationEntry>,
) -> ServiceResult<Json<ConversationEntry>> {
    info!("Adding conversation entry for case: {}", id);

    entry.case_id = id;
    entry.id = Uuid::new_v4();
    entry.timestamp = Utc::now();

    let persistence_url = format!("{}/api/v1/cases/{}/history", state.config.service_url("persistence"), id);
    let saved_entry = state
        .http_client
        .post::<ConversationEntry, ConversationEntry>(&persistence_url, &entry)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    Ok(Json(saved_entry))
}

#[instrument(skip(state))]
async fn get_case_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ServiceResult<Json<CaseWorkflow>> {
    info!("Getting workflow for case: {}", id);

    let persistence_url = format!("{}/api/v1/cases/{}/workflow", state.config.service_url("persistence"), id);
    let workflow = state
        .http_client
        .get::<CaseWorkflow>(&persistence_url)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    Ok(Json(workflow))
}

#[instrument(skip(state))]
async fn update_case_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(workflow): Json<CaseWorkflow>,
) -> ServiceResult<Json<CaseWorkflow>> {
    info!("Updating workflow for case: {}", id);

    let persistence_url = format!("{}/api/v1/cases/{}/workflow", state.config.service_url("persistence"), id);
    let updated_workflow = state
        .http_client
        .put::<CaseWorkflow, CaseWorkflow>(&persistence_url, &workflow)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    Ok(Json(updated_workflow))
}
