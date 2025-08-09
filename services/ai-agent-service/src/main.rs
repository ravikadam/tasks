use axum::{
    extract::State,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use common::{config::ServiceConfig, http_client::HttpClient, HealthResponse, ServiceResult};
use models::{
    ConversationEntry, MessageRequest, MessageResponse, MessageSender, 
    CreateTaskRequest, CreateCaseRequest, Priority, Case, Task
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, instrument};
use uuid::Uuid;

mod llm_client;
use llm_client::LLMClient;

#[derive(Clone)]
struct AppState {
    config: ServiceConfig,
    http_client: HttpClient,
    llm_client: LLMClient,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    
    let config = ServiceConfig::from_env("ai-agent-service", 8004);
    
    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .init();

    let state = AppState {
        config: config.clone(),
        http_client: HttpClient::new(),
        llm_client: LLMClient::new(config.openai_api_key.clone()),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/process", post(process_message))
        .with_state(Arc::new(state))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("AI Agent Service listening on port {}", config.port);

    axum::serve(listener, app).await?;
    Ok(())
}

#[instrument]
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse::new("ai-agent-service"))
}

#[instrument(skip(state))]
async fn process_message(
    State(state): State<Arc<AppState>>,
    Json(request): Json<MessageRequest>,
) -> ServiceResult<Json<MessageResponse>> {
    info!("Processing message: {:?}", request);

    let mut case_id = request.case_id;
    let mut actions_taken = Vec::new();
    let mut tasks_created = Vec::new();
    let mut tasks_updated = Vec::new();

    // Step 1: Determine if this is a new case or existing case
    if case_id.is_none() {
        // Check if we can find an existing case based on context
        case_id = find_or_create_case(&state, &request.message, &request.sender_id).await?;
        actions_taken.push("Created new case".to_string());
    }

    let case_id = case_id.unwrap();

    // Step 2: Add conversation entry
    let conversation_entry = ConversationEntry {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(), // TODO: Extract from session token
        case_id,
        message: request.message.clone(),
        sender: MessageSender::User,
        timestamp: Utc::now(),
        metadata: serde_json::json!({
            "channel": request.channel,
            "sender_id": request.sender_id
        }),
    };

    let case_mgmt_url = format!("{}/api/v1/cases/{}/history", state.config.service_url("case-management"), case_id);
    state.http_client
        .post::<ConversationEntry, ConversationEntry>(&case_mgmt_url, &conversation_entry)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    actions_taken.push("Added conversation entry".to_string());

    // Step 3: Process message with LLM to extract tasks and actions
    let ai_response = state.llm_client.process_message(&request.message, case_id).await
        .map_err(|e| common::ServiceError::Internal(anyhow::anyhow!("AI processing failed: {}", e)))?;
    
    // Step 4: Create tasks based on AI analysis
    for task_data in ai_response.tasks {
        let create_task_request = CreateTaskRequest {
            title: task_data.title,
            description: task_data.description,
            task_type: task_data.task_type,
            priority: task_data.priority,
            due_date: task_data.due_date,
        };

        let task_mgmt_url = format!("{}/api/v1/cases/{}/tasks", state.config.service_url("task-management"), case_id);
        let created_task = state.http_client
            .post::<CreateTaskRequest, Task>(&task_mgmt_url, &create_task_request)
            .await
            .map_err(|e| common::ServiceError::HttpClient(e))?;

        tasks_created.push(created_task.id);
        actions_taken.push(format!("Created task: {}", created_task.title));
    }

    // Step 5: Add AI response to conversation
    let ai_conversation_entry = ConversationEntry {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(), // TODO: Extract from session token
        case_id,
        message: ai_response.response.clone(),
        sender: MessageSender::Agent,
        timestamp: Utc::now(),
        metadata: serde_json::json!({}),
    };

    state.http_client
        .post::<ConversationEntry, ConversationEntry>(&case_mgmt_url, &ai_conversation_entry)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    let response = MessageResponse {
        case_id,
        response: ai_response.response,
        actions_taken,
        tasks_created,
        tasks_updated,
    };

    info!("Message processed successfully: {:?}", response);
    Ok(Json(response))
}

async fn find_or_create_case(
    state: &AppState,
    message: &str,
    sender_id: &str,
) -> ServiceResult<Option<Uuid>> {
    // For now, always create a new case
    // In a real implementation, you might search for existing cases based on context
    
    let case_title = extract_case_title(message);
    let create_case_request = CreateCaseRequest {
        title: case_title,
        description: Some(message.to_string()),
        priority: Priority::Medium,
        assigned_to: Some(sender_id.to_string()),
    };

    let case_mgmt_url = format!("{}/api/v1/cases", state.config.service_url("case-management"));
    let created_case = state.http_client
        .post::<CreateCaseRequest, Case>(&case_mgmt_url, &create_case_request)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    Ok(Some(created_case.id))
}

fn extract_case_title(message: &str) -> String {
    // Simple heuristic to extract a title from the message
    let words: Vec<&str> = message.split_whitespace().take(6).collect();
    let title = words.join(" ");
    
    if title.len() > 50 {
        format!("{}...", &title[..47])
    } else {
        title
    }
}
