use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Re-export common types
pub use chrono;
pub use serde;
pub use uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: CaseStatus,
    pub priority: Priority,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub assigned_to: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaseStatus {
    Open,
    InProgress,
    Waiting,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub case_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub priority: Priority,
    pub due_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Meeting,
    Shopping,
    Work,
    Personal,
    Research,
    Communication,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
    OnHold,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEntry {
    pub id: Uuid,
    pub case_id: Uuid,
    pub message: String,
    pub sender: MessageSender,
    pub timestamp: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageSender {
    User,
    Agent,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseWorkflow {
    pub id: Uuid,
    pub case_id: Uuid,
    pub current_step: String,
    pub steps: Vec<WorkflowStep>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub name: String,
    pub description: String,
    pub status: StepStatus,
    pub required_actions: Vec<String>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Active,
    Completed,
    Skipped,
}

// API Request/Response models
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageRequest {
    pub case_id: Option<Uuid>,
    pub message: String,
    pub sender_id: String,
    pub channel: MessageChannel,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageChannel {
    Bot,
    Email,
    WebChat,
    API,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub case_id: Uuid,
    pub response: String,
    pub actions_taken: Vec<String>,
    pub tasks_created: Vec<Uuid>,
    pub tasks_updated: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCaseRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub assigned_to: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCaseRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<CaseStatus>,
    pub priority: Option<Priority>,
    pub assigned_to: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub task_type: TaskType,
    pub priority: Priority,
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<Priority>,
    pub due_date: Option<DateTime<Utc>>,
}

// Error types
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}
