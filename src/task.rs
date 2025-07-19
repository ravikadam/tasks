use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a task with flexible attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub task_type: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub status: TaskStatus,
    pub attributes: HashMap<String, Value>,
}

/// Task status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::Pending
    }
}

/// Data structure for creating a new task
#[derive(Debug, Clone)]
pub struct TaskCreationData {
    pub title: String,
    pub task_type: String,
    pub description: String,
    pub attributes: HashMap<String, Value>,
}

/// Task service trait defining operations on individual tasks
pub trait TaskService {
    fn create_task(data: TaskCreationData) -> Task;
    fn update_status(&mut self, status: TaskStatus);
    fn add_attribute(&mut self, key: String, value: Value);
    fn remove_attribute(&mut self, key: &str);
    fn is_completed(&self) -> bool;
    fn validate(&self) -> Result<(), TaskValidationError>;
}

/// Task validation errors
#[derive(Debug, thiserror::Error)]
pub enum TaskValidationError {
    #[error("Task title cannot be empty")]
    EmptyTitle,
    #[error("Task description cannot be empty")]
    EmptyDescription,
    #[error("Invalid task type: {0}")]
    InvalidTaskType(String),
}

impl Task {
    /// Create a new task with the given data
    pub fn new(data: TaskCreationData) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: data.title,
            task_type: data.task_type,
            description: data.description,
            created_at: Utc::now(),
            status: TaskStatus::default(),
            attributes: data.attributes,
        }
    }

    /// Get supported task types
    pub fn supported_types() -> Vec<&'static str> {
        vec![
            "meeting", "shopping", "work", "personal", "reminder", 
            "deadline", "call", "email", "travel", "health", 
            "finance", "learning"
        ]
    }
}

impl TaskService for Task {
    fn create_task(data: TaskCreationData) -> Task {
        Task::new(data)
    }

    fn update_status(&mut self, status: TaskStatus) {
        self.status = status;
    }

    fn add_attribute(&mut self, key: String, value: Value) {
        self.attributes.insert(key, value);
    }

    fn remove_attribute(&mut self, key: &str) {
        self.attributes.remove(key);
    }

    fn is_completed(&self) -> bool {
        self.status == TaskStatus::Completed
    }

    fn validate(&self) -> Result<(), TaskValidationError> {
        if self.title.trim().is_empty() {
            return Err(TaskValidationError::EmptyTitle);
        }

        if self.description.trim().is_empty() {
            return Err(TaskValidationError::EmptyDescription);
        }

        if !Task::supported_types().contains(&self.task_type.as_str()) {
            return Err(TaskValidationError::InvalidTaskType(self.task_type.clone()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_task_creation() {
        let data = TaskCreationData {
            title: "Test Task".to_string(),
            task_type: "work".to_string(),
            description: "A test task".to_string(),
            attributes: HashMap::new(),
        };

        let task = Task::new(data);
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.task_type, "work");
        assert_eq!(task.status, TaskStatus::Pending);
        assert!(!task.id.is_empty());
    }

    #[test]
    fn test_task_validation() {
        let mut task = Task::new(TaskCreationData {
            title: "Valid Task".to_string(),
            task_type: "work".to_string(),
            description: "Valid description".to_string(),
            attributes: HashMap::new(),
        });

        assert!(task.validate().is_ok());

        task.title = "".to_string();
        assert!(matches!(task.validate(), Err(TaskValidationError::EmptyTitle)));
    }

    #[test]
    fn test_task_attributes() {
        let mut task = Task::new(TaskCreationData {
            title: "Test Task".to_string(),
            task_type: "meeting".to_string(),
            description: "Meeting task".to_string(),
            attributes: HashMap::new(),
        });

        task.add_attribute("location".to_string(), json!("Conference Room A"));
        assert_eq!(task.attributes.get("location"), Some(&json!("Conference Room A")));

        task.remove_attribute("location");
        assert!(task.attributes.get("location").is_none());
    }
}
