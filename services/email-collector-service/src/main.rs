//! Email Collector Service
//!
//! This microservice exposes a simple HTTP endpoint that accepts email data
//! (for example from a webhook or another mail relay) and forwards the
//! contents to the channel service for further processing. It also supports
//! actively fetching emails from IMAP servers.
//!
//! Features:
//! - Webhook endpoint for receiving email data
//! - IMAP client for fetching emails from email servers
//! - Automatic forwarding to channel service for AI processing
//!
//! The service can be configured via environment variables defined in
//! `shared/common/src/config.rs`.

use axum::{
    extract::State,
    response::Json,
    routing::{get, post},
    Router,
};
use common::{config::ServiceConfig, http_client::HttpClient, HealthResponse, ServiceResult};
use models::{MessageRequest, MessageResponse, MessageChannel};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{info, error, warn, instrument};
use reqwest;
use serde_json;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// Email server configuration for Microsoft Graph API
#[derive(Debug, Clone)]
struct EmailConfig {
    username: String,
    oauth_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphMessage {
    id: String,
    subject: Option<String>,
    #[serde(rename = "bodyPreview")]
    body_preview: Option<String>,
    body: Option<GraphMessageBody>,
    from: Option<GraphEmailAddress>,
    #[serde(rename = "isRead")]
    is_read: bool,
}

#[derive(Debug, Deserialize)]
struct GraphMessageBody {
    content: Option<String>,
    #[serde(rename = "contentType")]
    content_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphEmailAddress {
    #[serde(rename = "emailAddress")]
    email_address: Option<GraphEmail>,
}

#[derive(Debug, Deserialize)]
struct GraphEmail {
    address: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphMessagesResponse {
    value: Vec<GraphMessage>,
}

/// OAuth token data received from dashboard service
#[derive(Debug, Deserialize, Serialize)]
struct OAuthTokenRequest {
    access_token: String,
    token_type: String,
}

/// OAuth token response
#[derive(Debug, Serialize)]
struct OAuthTokenResponse {
    status: String,
    message: String,
}

impl EmailConfig {
    fn from_env() -> Option<Self> {
        let email_config = if let Ok(username) = std::env::var("IMAP_USERNAME") {
            Some(EmailConfig {
                username,
                oauth_token: None,
            })
        } else {
            warn!("Email configuration not found in environment variables");
            None
        };
        email_config
    }
}

/// Shared application state.  Holds the service configuration and an
/// HTTP client for talking to downstream services.
#[derive(Clone)]
struct AppState {
    config: ServiceConfig,
    http_client: HttpClient,
    email_config: Arc<Mutex<Option<EmailConfig>>>,
}

/// Schema for the incoming email payload.  Many mail providers can be
/// configured to POST email data as JSON to a webhook.  At minimum the
/// service requires a sender identifier and the message body.  The subject
/// field is optional but, if present, will be prefixed to the message body
/// when constructing the request to the channel service.
#[derive(Debug, Deserialize)]
struct IncomingEmail {
    /// Email address of the sender.  Used as the sender_id on the
    /// downstream message.
    sender: String,
    /// Optional subject line of the email.
    subject: Option<String>,
    /// Body of the email.
    body: String,
    /// Optional case identifier.  If provided, the message will be
    /// associated with an existing case.  Otherwise a new case will be
    /// created by the AI agent.
    case_id: Option<uuid::Uuid>,
}

/// Checks if an email is work-related based on content and metadata
fn is_work_related_email(message: &GraphMessage) -> bool {
    let subject = message.subject.as_deref().unwrap_or("").to_lowercase();
    let body = message.body
        .as_ref()
        .and_then(|b| b.content.as_ref())
        .or(message.body_preview.as_ref())
        .map_or("".to_string(), |v| v.to_lowercase());
    
    let sender_email = message.from
        .as_ref()
        .and_then(|from| from.email_address.as_ref())
        .map_or("".to_string(), |addr| addr.address.as_deref().unwrap_or("").to_lowercase());
    
    // Work-related keywords in subject or body
    let work_keywords = [
        "project", "task", "deadline", "meeting", "urgent", "action required",
        "follow up", "review", "approval", "budget", "proposal", "contract",
        "client", "customer", "deliverable", "milestone", "schedule", "priority",
        "report", "analysis", "presentation", "document", "specification",
        "requirement", "issue", "bug", "feature", "development", "implementation",
        "deployment", "release", "testing", "qa", "quality", "performance",
        "security", "compliance", "audit", "invoice", "payment", "expense",
        "hr", "human resources", "policy", "procedure", "training", "onboarding",
        "team", "collaboration", "sync", "standup", "retrospective", "sprint",
        "agile", "scrum", "kanban", "jira", "confluence", "slack", "teams"
    ];
    
    // Check for work keywords in subject or body
    let has_work_keywords = work_keywords.iter().any(|keyword| {
        subject.contains(keyword) || body.contains(keyword)
    });
    
    // Check for business email domains (common work domains)
    let business_domains = [
        "company.com", "corp.com", "inc.com", "ltd.com", "org", "gov",
        "fincentive.co" // Add user's work domain
    ];
    
    let is_business_sender = business_domains.iter().any(|domain| {
        sender_email.contains(domain)
    });
    
    // Filter out obvious personal/promotional emails
    let personal_keywords = [
        "unsubscribe", "newsletter", "promotion", "deal", "sale", "discount",
        "offer", "marketing", "advertisement", "spam", "social media",
        "facebook", "twitter", "instagram", "linkedin notification",
        "youtube", "netflix", "amazon prime", "shopping", "order confirmation",
        "delivery", "tracking", "receipt", "personal", "family", "friend"
    ];
    
    let is_personal = personal_keywords.iter().any(|keyword| {
        subject.contains(keyword) || body.contains(keyword) || sender_email.contains(keyword)
    });
    
    // Email is work-related if:
    // 1. Contains work keywords AND not personal, OR
    // 2. From business domain AND not personal
    (has_work_keywords || is_business_sender) && !is_personal
}

/// Fetches new emails from Microsoft Graph API and processes them
async fn fetch_emails(state: &AppState) -> anyhow::Result<()> {
    let config_guard = state.email_config.lock().await;
    let config = match config_guard.as_ref() {
        Some(config) => config.clone(),
        None => {
            warn!("No email configuration available");
            return Ok(());
        }
    };
    drop(config_guard);

    let oauth_token = match &config.oauth_token {
        Some(token) => token,
        None => {
            warn!("No OAuth token available for Microsoft Graph API");
            return Ok(());
        }
    };

    info!("Fetching work-related emails via Microsoft Graph API for user: {}", config.username);

    // Get unread messages from Microsoft Graph API with expanded properties for better filtering
    let graph_url = "https://graph.microsoft.com/v1.0/me/messages?$filter=isRead eq false&$top=20&$select=id,subject,bodyPreview,body,from,receivedDateTime,importance,categories";
    
    let client = reqwest::Client::new();
    let response = client
        .get(graph_url)
        .header("Authorization", format!("Bearer {}", oauth_token))
        .header("Content-Type", "application/json")
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!("Microsoft Graph API request failed with status {}: {}", status, error_text));
    }

    let messages_response: GraphMessagesResponse = response.json().await?;
    info!("Found {} unread emails, filtering for work-related content", messages_response.value.len());

    let mut work_emails_processed = 0;
    let mut total_emails_checked = 0;

    for message in messages_response.value {
        total_emails_checked += 1;
        
        // Apply work-related filtering
        if !is_work_related_email(&message) {
            info!("Skipping non-work-related email: {}", 
                message.subject.as_deref().unwrap_or("[No Subject]"));
            continue;
        }
        
        work_emails_processed += 1;
        info!("Processing work-related email: {}", 
            message.subject.as_deref().unwrap_or("[No Subject]"));
        
        if let Err(e) = process_graph_message(&message, state, oauth_token).await {
            error!("Failed to process email message {}: {}", message.id, e);
        } else {
            info!("Successfully processed work email message {}", message.id);
            // Mark message as read
            if let Err(e) = mark_message_as_read(&message.id, oauth_token).await {
                warn!("Failed to mark message {} as read: {}", message.id, e);
            }
        }
    }

    info!("Email filtering complete: {} work-related emails processed out of {} total emails checked", 
        work_emails_processed, total_emails_checked);

    Ok(())
}

/// Processes a single email message and forwards it to the channel service.
async fn process_graph_message(message: &GraphMessage, state: &AppState, _oauth_token: &str) -> anyhow::Result<()> {
    let sender = message.from
        .as_ref()
        .and_then(|from| from.email_address.as_ref())
        .and_then(|email| email.address.as_ref())
        .map_or("unknown@unknown.com".to_string(), |v| v.clone());
    
    let subject = message.subject.as_deref().unwrap_or("[No Subject]");
    let body = message.body
        .as_ref()
        .and_then(|b| b.content.as_ref())
        .or(message.body_preview.as_ref())
        .map_or("[No body content]".to_string(), |v| v.clone());
    
    let message_text = if !subject.is_empty() && subject != "[No Subject]" {
        format!("{}\n\n{}", subject, body)
    } else {
        body
    };
    
    let message_request = MessageRequest {
        case_id: None,
        message: message_text,
        sender_id: sender,
        channel: MessageChannel::Email,
    };
    
    let channel_url = format!("{}/api/v1/message", state.config.service_url("channel"));
    let _response = state
        .http_client
        .post::<MessageRequest, MessageResponse>(&channel_url, &message_request)
        .await?;
    
    Ok(())
}

async fn mark_message_as_read(message_id: &str, oauth_token: &str) -> anyhow::Result<()> {
    let graph_url = format!("https://graph.microsoft.com/v1.0/me/messages/{}", message_id);
    
    let client = reqwest::Client::new();
    let update_body = serde_json::json!({
        "isRead": true
    });
    
    let response = client
        .patch(&graph_url)
        .header("Authorization", format!("Bearer {}", oauth_token))
        .header("Content-Type", "application/json")
        .json(&update_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!("Failed to mark message as read, status {}: {}", status, error_text));
    }
    Ok(())
}

/// Background task that periodically fetches emails
async fn email_polling_task(state: Arc<AppState>) {
    let poll_interval = 60; // Default 60 seconds polling interval
    
    info!("Starting email polling every {} seconds", poll_interval);
    
    let mut interval = tokio::time::interval(Duration::from_secs(poll_interval));
    
    loop {
        interval.tick().await;
        
        if let Err(e) = fetch_emails(&state).await {
            error!("Email fetching failed: {}", e);
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from `.env` if present.
    dotenv::dotenv().ok();

    // Initialise configuration.  The service is named `email-collector-service`
    // and defaults to port 8006.  Additional options (such as
    // CHANNEL_SERVICE_URL) can be supplied via the environment.
    let config = ServiceConfig::from_env("email-collector-service", 8006);

    // Set up tracing/structured logging.  Respect RUST_LOG if provided.
    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .init();

    // Initialize email configuration from environment variables
    let email_config = EmailConfig::from_env();
    
    if email_config.is_some() {
        info!("Email fetching enabled with Microsoft Graph API configuration");
    } else {
        info!("Email fetching disabled - no email configuration found");
        info!("To enable email fetching, set: IMAP_USERNAME");
    }

    let state = AppState {
        config: config.clone(),
        http_client: HttpClient::new(),
        email_config: Arc::new(Mutex::new(email_config)),
    };

    let state_arc = Arc::new(state);

    // Start email polling task if email configuration is available
    {
        let email_config_guard = state_arc.email_config.lock().await;
        if email_config_guard.is_some() {
            let polling_state = state_arc.clone();
            tokio::spawn(async move {
                email_polling_task(polling_state).await;
            });
        }
    }

    // Build the router.  Expose a health endpoint and an email ingestion
    // endpoint.  Attach CORS and tracing layers for better observability.
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/email", post(handle_incoming_email))
        .route("/api/v1/oauth/token", post(handle_oauth_token))
        .with_state(state_arc)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );

    // Bind and serve the HTTP server.
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("Email Collector Service listening on port {}", config.port);

    axum::serve(listener, app).await?;
    Ok(())
}

/// Simple health check endpoint.  Returns a JSON payload including the
/// service name and version.
#[instrument]
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse::new("email-collector-service"))
}

/// Handler for incoming email webhooks.  Constructs a [`MessageRequest`]
/// with the email contents and dispatches it to the channel service.
#[instrument(skip(state))]
async fn handle_incoming_email(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<IncomingEmail>,
) -> ServiceResult<Json<MessageResponse>> {
    info!("Received incoming email: sender={}, subject={:?}", payload.sender, payload.subject);

    // Combine subject and body into a single message.  If a subject is
    // provided, prefix it to the body separated by a newline.
    let message = match payload.subject {
        Some(ref subject) if !subject.is_empty() => format!("{}\n\n{}", subject, payload.body),
        _ => payload.body.clone(),
    };

    // Construct the message request.  Pass through the optional case_id if
    // provided by the caller.  This allows email replies to an existing
    // case to be threaded correctly.
    let message_request = MessageRequest {
        case_id: payload.case_id,
        message,
        sender_id: payload.sender,
        channel: MessageChannel::Email,
    };

    // Determine the URL for the channel service.  The `service_url` helper
    // resolves service names to full URLs, allowing overrides via
    // environment variables (see ServiceConfig::service_url in shared/common).
    let channel_url = format!(
        "{}/api/v1/message",
        state.config.service_url("channel")
    );

    // Forward the request to the channel service.  If the downstream call
    // fails, propagate the error as a ServiceError to produce an
    // appropriate HTTP response.
    let response = state
        .http_client
        .post::<MessageRequest, MessageResponse>(&channel_url, &message_request)
        .await
        .map_err(|e| common::ServiceError::HttpClient(e))?;

    info!("Channel service responded for email: case_id={}", response.case_id);
    Ok(Json(response))
}

/// Handle OAuth token from dashboard service
#[instrument(skip(state))]
async fn handle_oauth_token(
    State(state): State<Arc<AppState>>,
    Json(token_request): Json<OAuthTokenRequest>,
) -> ServiceResult<Json<OAuthTokenResponse>> {
    info!("Received OAuth token from dashboard service");
    
    // Update the email configuration with the OAuth token
    {
        let mut email_config_guard = state.email_config.lock().await;
        if let Some(ref mut config) = email_config_guard.as_mut() {
            config.oauth_token = Some(token_request.access_token.clone());
            info!("OAuth token updated in email configuration for user: {}", config.username);
        } else {
            warn!("No email configuration found to update with OAuth token");
        }
    }
    
    info!("OAuth token received and stored for email authentication");
    
    Ok(Json(OAuthTokenResponse {
        status: "success".to_string(),
        message: "OAuth token received and configured for email access".to_string(),
    }))
}