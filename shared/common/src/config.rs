use std::env;

#[derive(Clone, Debug)]
pub struct ServiceConfig {
    pub service_name: String,
    pub port: u16,
    pub database_url: Option<String>,
    pub openai_api_key: Option<String>,
    pub log_level: String,
}

impl ServiceConfig {
    pub fn from_env(service_name: &str, default_port: u16) -> Self {
        Self {
            service_name: service_name.to_string(),
            port: env::var("PORT")
                .unwrap_or_else(|_| default_port.to_string())
                .parse()
                .unwrap_or(default_port),
            database_url: env::var("DATABASE_URL").ok(),
            openai_api_key: env::var("OPENAI_API_KEY").ok(),
            log_level: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        }
    }

    pub fn service_url(&self, service: &str) -> String {
        match service {
            "channel" => env::var("CHANNEL_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8001".to_string()),
            "case-management" => env::var("CASE_MANAGEMENT_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8002".to_string()),
            "task-management" => env::var("TASK_MANAGEMENT_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8003".to_string()),
            "ai-agent" => env::var("AI_AGENT_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8004".to_string()),
            "persistence" => env::var("PERSISTENCE_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8005".to_string()),
            _ => format!("http://localhost:8000"),
        }
    }
}
