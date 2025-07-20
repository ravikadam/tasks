use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use common::{config::ServiceConfig, HealthResponse, ServiceResult, ServiceError};
use models::{
    Case, Task, ConversationEntry, CaseWorkflow, CreateCaseRequest, UpdateCaseRequest,
    CreateTaskRequest, UpdateTaskRequest, CaseStatus, TaskStatus, Priority, TaskType,
    MessageSender, StepStatus
};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, instrument};
use uuid::Uuid;

mod database_simple;
use database_simple::Database;

#[derive(Clone)]
struct AppState {
    config: ServiceConfig,
    db: Database,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    
    let config = ServiceConfig::from_env("persistence-service", 8005);
    
    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .init();

    // Initialize database
    let default_db_url = "memory://localhost".to_string();
    let database_url = config.database_url.as_ref()
        .unwrap_or(&default_db_url);
    
    let db = Database::new(database_url).await
        .map_err(|e| anyhow::anyhow!("Failed to initialize database: {}", e))?;
    db.migrate().await
        .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;

    let state = AppState {
        config: config.clone(),
        db,
    };

    let app = Router::new()
        .route("/health", get(health_check))
        // Case routes
        .route("/api/v1/cases", post(create_case))
        .route("/api/v1/cases/:id", get(get_case))
        .route("/api/v1/cases/:id", put(update_case))
        .route("/api/v1/cases/:id/history", get(get_conversation_history))
        .route("/api/v1/cases/:id/history", post(add_conversation_entry))
        .route("/api/v1/cases/:id/workflow", get(get_case_workflow))
        .route("/api/v1/cases/:id/workflow", put(update_case_workflow))
        // Task routes
        .route("/api/v1/tasks", post(create_task))
        .route("/api/v1/tasks/:id", get(get_task))
        .route("/api/v1/tasks/:id", put(update_task))
        .route("/api/v1/tasks/:id", delete(delete_task))
        .route("/api/v1/cases/:case_id/tasks", get(get_tasks_for_case))
        .with_state(Arc::new(state))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("Persistence Service listening on port {}", config.port);

    axum::serve(listener, app).await?;
    Ok(())
}

#[instrument]
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse::new("persistence-service"))
}

// Case endpoints
#[instrument(skip(state))]
async fn create_case(
    State(state): State<Arc<AppState>>,
    Json(case): Json<Case>,
) -> ServiceResult<Json<Case>> {
    info!("Creating case: {}", case.id);
    let created_case = state.db.create_case(case).await?;
    Ok(Json(created_case))
}

#[instrument(skip(state))]
async fn get_case(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ServiceResult<Json<Case>> {
    info!("Getting case: {}", id);
    let case = state.db.get_case(id).await?;
    Ok(Json(case))
}

#[instrument(skip(state))]
async fn update_case(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateCaseRequest>,
) -> ServiceResult<Json<Case>> {
    info!("Updating case: {}", id);
    let updated_case = state.db.update_case(id, request).await?;
    Ok(Json(updated_case))
}

#[instrument(skip(state))]
async fn get_conversation_history(
    State(state): State<Arc<AppState>>,
    Path(case_id): Path<Uuid>,
) -> ServiceResult<Json<Vec<ConversationEntry>>> {
    info!("Getting conversation history for case: {}", case_id);
    let history = state.db.get_conversation_history(case_id).await?;
    Ok(Json(history))
}

#[instrument(skip(state))]
async fn add_conversation_entry(
    State(state): State<Arc<AppState>>,
    Path(case_id): Path<Uuid>,
    Json(entry): Json<ConversationEntry>,
) -> ServiceResult<Json<ConversationEntry>> {
    info!("Adding conversation entry for case: {}", case_id);
    let saved_entry = state.db.add_conversation_entry(entry).await?;
    Ok(Json(saved_entry))
}

#[instrument(skip(state))]
async fn get_case_workflow(
    State(state): State<Arc<AppState>>,
    Path(case_id): Path<Uuid>,
) -> ServiceResult<Json<CaseWorkflow>> {
    info!("Getting workflow for case: {}", case_id);
    let workflow = state.db.get_case_workflow(case_id).await?;
    Ok(Json(workflow))
}

#[instrument(skip(state))]
async fn update_case_workflow(
    State(state): State<Arc<AppState>>,
    Path(case_id): Path<Uuid>,
    Json(workflow): Json<CaseWorkflow>,
) -> ServiceResult<Json<CaseWorkflow>> {
    info!("Updating workflow for case: {}", case_id);
    let updated_workflow = state.db.update_case_workflow(workflow).await?;
    Ok(Json(updated_workflow))
}

// Task endpoints
#[instrument(skip(state))]
async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(task): Json<Task>,
) -> ServiceResult<Json<Task>> {
    info!("Creating task: {}", task.id);
    let created_task = state.db.create_task(task).await?;
    Ok(Json(created_task))
}

#[instrument(skip(state))]
async fn get_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ServiceResult<Json<Task>> {
    info!("Getting task: {}", id);
    let task = state.db.get_task(id).await?;
    Ok(Json(task))
}

#[instrument(skip(state))]
async fn update_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateTaskRequest>,
) -> ServiceResult<Json<Task>> {
    info!("Updating task: {}", id);
    let updated_task = state.db.update_task(id, request).await?;
    Ok(Json(updated_task))
}

#[instrument(skip(state))]
async fn delete_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ServiceResult<StatusCode> {
    info!("Deleting task: {}", id);
    state.db.delete_task(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(state))]
async fn get_tasks_for_case(
    State(state): State<Arc<AppState>>,
    Path(case_id): Path<Uuid>,
) -> ServiceResult<Json<Vec<Task>>> {
    info!("Getting tasks for case: {}", case_id);
    let tasks = state.db.get_tasks_for_case(case_id).await?;
    Ok(Json(tasks))
}
