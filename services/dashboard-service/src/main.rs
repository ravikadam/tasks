use axum::{
    extract::State,
    response::{Html, Json},
    routing::get,
    Router,
};
use common::{config::ServiceConfig, http_client::HttpClient, HealthResponse, ServiceResult};
use models::Task;
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

    let config = ServiceConfig::from_env("dashboard-service", 8006);

    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .init();

    let state = AppState {
        config: config.clone(),
        http_client: HttpClient::new(),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/", get(show_pending_tasks))
        .with_state(Arc::new(state))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("Dashboard Service listening on port {}", config.port);

    axum::serve(listener, app).await?;
    Ok(())
}

#[instrument]
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse::new("dashboard-service"))
}

#[instrument(skip(state))]
async fn show_pending_tasks(State(state): State<Arc<AppState>>) -> ServiceResult<Html<String>> {
    let url = format!(
        "{}/api/v1/tasks?status=Pending",
        state.config.service_url("task-management")
    );
    let tasks = state
        .http_client
        .get::<Vec<Task>>(&url)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    let mut html = String::from("<html><head><title>Pending Tasks</title></head><body><h1>Pending Tasks</h1><ul>");
    for task in tasks {
        html.push_str(&format!("<li>{}</li>", task.title));
    }
    html.push_str("</ul></body></html>");

    Ok(Html(html))
}
