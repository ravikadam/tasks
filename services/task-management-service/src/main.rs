use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use common::{config::ServiceConfig, http_client::HttpClient, HealthResponse, ServiceResult};
use models::{
    Task, TaskStatus, CreateTaskRequest, UpdateTaskRequest, TaskType, Priority
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

#[derive(Debug, serde::Deserialize)]
struct TaskQuery {
    status: Option<TaskStatus>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    
    let config = ServiceConfig::from_env("task-management-service", 8003);
    
    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .init();

    let state = AppState {
        config: config.clone(),
        http_client: HttpClient::new(),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/cases/:case_id/tasks", get(get_tasks_for_case))
        .route("/api/v1/cases/:case_id/tasks", post(create_task))
        .route("/api/v1/tasks", get(get_tasks))
        .route("/api/v1/tasks/:id", get(get_task))
        .route("/api/v1/tasks/:id", put(update_task))
        .route("/api/v1/tasks/:id", delete(delete_task))
        .route("/api/v1/tasks/:id/complete", put(complete_task))
        .with_state(Arc::new(state))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("Task Management Service listening on port {}", config.port);

    axum::serve(listener, app).await?;
    Ok(())
}

#[instrument]
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse::new("task-management-service"))
}

#[instrument(skip(state))]
async fn get_tasks_for_case(
    State(state): State<Arc<AppState>>,
    Path(case_id): Path<Uuid>,
) -> ServiceResult<Json<Vec<Task>>> {
    info!("Getting tasks for case: {}", case_id);

    let persistence_url = format!("{}/api/v1/cases/{}/tasks", state.config.service_url("persistence"), case_id);
    let tasks = state
        .http_client
        .get::<Vec<Task>>(&persistence_url)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    Ok(Json(tasks))
}

#[instrument(skip(state))]
async fn create_task(
    State(state): State<Arc<AppState>>,
    Path(case_id): Path<Uuid>,
    Json(request): Json<CreateTaskRequest>,
) -> ServiceResult<Json<Task>> {
    info!("Creating task for case {}: {:?}", case_id, request);

    let now = Utc::now();
    let task_id = Uuid::new_v4();
    let task = Task {
        id: task_id,
        user_id: Uuid::new_v4(), // TODO: Extract from session token
        case_id,
        title: request.title,
        description: request.description,
        task_type: request.task_type,
        status: TaskStatus::Pending,
        priority: request.priority,
        due_date: request.due_date,
        created_at: now,
        updated_at: now,
        completed_at: None,
        metadata: serde_json::json!({}),
    };

    let persistence_url = format!("{}/api/v1/tasks", state.config.service_url("persistence"));
    let saved_task = state
        .http_client
        .post::<Task, Task>(&persistence_url, &task)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    info!("Task created with ID: {}", saved_task.id);
    Ok(Json(saved_task))
}

#[instrument(skip(state))]
async fn get_tasks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TaskQuery>,
) -> ServiceResult<Json<Vec<Task>>> {
    info!("Getting tasks with query: {:?}", query.status);

    let base_url = format!("{}/api/v1/tasks", state.config.service_url("persistence"));
    let url = if let Some(status) = query.status {
        let status_str = format!("{:?}", status);
        format!("{}?status={}", base_url, status_str)
    } else {
        base_url
    };

    let tasks = state
        .http_client
        .get::<Vec<Task>>(&url)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    Ok(Json(tasks))
}

#[instrument(skip(state))]
async fn get_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ServiceResult<Json<Task>> {
    info!("Getting task: {}", id);

    let persistence_url = format!("{}/api/v1/tasks/{}", state.config.service_url("persistence"), id);
    let task = state
        .http_client
        .get::<Task>(&persistence_url)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    Ok(Json(task))
}

#[instrument(skip(state))]
async fn update_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateTaskRequest>,
) -> ServiceResult<Json<Task>> {
    info!("Updating task {}: {:?}", id, request);

    let persistence_url = format!("{}/api/v1/tasks/{}", state.config.service_url("persistence"), id);
    let updated_task = state
        .http_client
        .put::<UpdateTaskRequest, Task>(&persistence_url, &request)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    Ok(Json(updated_task))
}

#[instrument(skip(state))]
async fn delete_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ServiceResult<StatusCode> {
    info!("Deleting task: {}", id);

    let persistence_url = format!("{}/api/v1/tasks/{}", state.config.service_url("persistence"), id);
    state
        .http_client
        .delete(&persistence_url)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(state))]
async fn complete_task(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ServiceResult<Json<Task>> {
    info!("Completing task: {}", id);

    let update_request = UpdateTaskRequest {
        title: None,
        description: None,
        status: Some(TaskStatus::Completed),
        priority: None,
        due_date: None,
    };

    let persistence_url = format!("{}/api/v1/tasks/{}", state.config.service_url("persistence"), id);
    let updated_task = state
        .http_client
        .put::<UpdateTaskRequest, Task>(&persistence_url, &update_request)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    Ok(Json(updated_task))
}
