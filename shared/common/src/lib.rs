use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use models::ApiError;
use serde_json::json;

pub mod config;
pub mod http_client;

// Common error handling
#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ServiceError::Database(ref e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
            }
            ServiceError::HttpClient(ref e) => {
                tracing::error!("HTTP client error: {:?}", e);
                (StatusCode::BAD_GATEWAY, "External service error")
            }
            ServiceError::Serialization(ref e) => {
                tracing::error!("Serialization error: {:?}", e);
                (StatusCode::BAD_REQUEST, "Invalid data format")
            }
            ServiceError::NotFound(ref message) => {
                (StatusCode::NOT_FOUND, message.as_str())
            }
            ServiceError::BadRequest(ref message) => {
                (StatusCode::BAD_REQUEST, message.as_str())
            }
            ServiceError::Unauthorized(ref message) => {
                (StatusCode::UNAUTHORIZED, message.as_str())
            }
            ServiceError::Internal(ref e) => {
                tracing::error!("Internal error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let body = Json(json!({
            "error": {
                "code": status.as_u16(),
                "message": error_message
            }
        }));

        (status, body).into_response()
    }
}

pub type ServiceResult<T> = Result<T, ServiceError>;

// Health check response
#[derive(serde::Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl HealthResponse {
    pub fn new(service_name: &str) -> Self {
        Self {
            status: "healthy".to_string(),
            service: service_name.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now(),
        }
    }
}
