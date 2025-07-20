use models::{
    Case, Task, ConversationEntry, CaseWorkflow, WorkflowStep,
    UpdateCaseRequest, UpdateTaskRequest, StepStatus
};
use common::{ServiceResult, ServiceError};
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Simple in-memory database for demonstration
#[derive(Clone)]
pub struct Database {
    cases: Arc<Mutex<HashMap<Uuid, Case>>>,
    tasks: Arc<Mutex<HashMap<Uuid, Task>>>,
    conversations: Arc<Mutex<HashMap<Uuid, Vec<ConversationEntry>>>>,
    workflows: Arc<Mutex<HashMap<Uuid, CaseWorkflow>>>,
}

impl Database {
    pub async fn new(_database_url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            cases: Arc::new(Mutex::new(HashMap::new())),
            tasks: Arc::new(Mutex::new(HashMap::new())),
            conversations: Arc::new(Mutex::new(HashMap::new())),
            workflows: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub async fn migrate(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // No-op for in-memory database
        Ok(())
    }

    // Case operations
    pub async fn create_case(&self, case: Case) -> ServiceResult<Case> {
        let mut cases = self.cases.lock().unwrap();
        cases.insert(case.id, case.clone());
        Ok(case)
    }

    pub async fn get_case(&self, id: Uuid) -> ServiceResult<Case> {
        let cases = self.cases.lock().unwrap();
        cases.get(&id)
            .cloned()
            .ok_or_else(|| ServiceError::NotFound(format!("Case with id {} not found", id)))
    }

    pub async fn update_case(&self, id: Uuid, request: UpdateCaseRequest) -> ServiceResult<Case> {
        let mut cases = self.cases.lock().unwrap();
        let case = cases.get_mut(&id)
            .ok_or_else(|| ServiceError::NotFound(format!("Case with id {} not found", id)))?;

        if let Some(title) = request.title {
            case.title = title;
        }
        if let Some(description) = request.description {
            case.description = Some(description);
        }
        if let Some(status) = request.status {
            case.status = status;
        }
        if let Some(priority) = request.priority {
            case.priority = priority;
        }
        if let Some(assigned_to) = request.assigned_to {
            case.assigned_to = Some(assigned_to);
        }
        case.updated_at = Utc::now();

        Ok(case.clone())
    }

    // Task operations
    pub async fn create_task(&self, task: Task) -> ServiceResult<Task> {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.insert(task.id, task.clone());
        Ok(task)
    }

    pub async fn get_task(&self, id: Uuid) -> ServiceResult<Task> {
        let tasks = self.tasks.lock().unwrap();
        tasks.get(&id)
            .cloned()
            .ok_or_else(|| ServiceError::NotFound(format!("Task with id {} not found", id)))
    }

    pub async fn update_task(&self, id: Uuid, request: UpdateTaskRequest) -> ServiceResult<Task> {
        let mut tasks = self.tasks.lock().unwrap();
        let task = tasks.get_mut(&id)
            .ok_or_else(|| ServiceError::NotFound(format!("Task with id {} not found", id)))?;

        if let Some(title) = request.title {
            task.title = title;
        }
        if let Some(description) = request.description {
            task.description = Some(description);
        }
        if let Some(status) = request.status {
            task.status = status;
            if matches!(task.status, models::TaskStatus::Completed) {
                task.completed_at = Some(Utc::now());
            }
        }
        if let Some(priority) = request.priority {
            task.priority = priority;
        }
        if let Some(due_date) = request.due_date {
            task.due_date = Some(due_date);
        }
        task.updated_at = Utc::now();

        Ok(task.clone())
    }

    pub async fn delete_task(&self, id: Uuid) -> ServiceResult<()> {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.remove(&id)
            .ok_or_else(|| ServiceError::NotFound(format!("Task with id {} not found", id)))?;
        Ok(())
    }

    pub async fn get_tasks_for_case(&self, case_id: Uuid) -> ServiceResult<Vec<Task>> {
        let tasks = self.tasks.lock().unwrap();
        let case_tasks: Vec<Task> = tasks.values()
            .filter(|task| task.case_id == case_id)
            .cloned()
            .collect();
        Ok(case_tasks)
    }

    // Conversation operations
    pub async fn get_conversation_history(&self, case_id: Uuid) -> ServiceResult<Vec<ConversationEntry>> {
        let conversations = self.conversations.lock().unwrap();
        Ok(conversations.get(&case_id).cloned().unwrap_or_default())
    }

    pub async fn add_conversation_entry(&self, entry: ConversationEntry) -> ServiceResult<ConversationEntry> {
        let mut conversations = self.conversations.lock().unwrap();
        conversations.entry(entry.case_id)
            .or_insert_with(Vec::new)
            .push(entry.clone());
        Ok(entry)
    }

    // Workflow operations
    pub async fn get_case_workflow(&self, case_id: Uuid) -> ServiceResult<CaseWorkflow> {
        let workflows = self.workflows.lock().unwrap();
        if let Some(workflow) = workflows.get(&case_id) {
            Ok(workflow.clone())
        } else {
            // Create default workflow
            let workflow = CaseWorkflow {
                id: Uuid::new_v4(),
                case_id,
                current_step: "initial".to_string(),
                steps: vec![
                    WorkflowStep {
                        name: "initial".to_string(),
                        description: "Case created".to_string(),
                        status: StepStatus::Completed,
                        required_actions: vec![],
                        completed_at: Some(Utc::now()),
                    },
                    WorkflowStep {
                        name: "processing".to_string(),
                        description: "Processing tasks".to_string(),
                        status: StepStatus::Active,
                        required_actions: vec!["Complete tasks".to_string()],
                        completed_at: None,
                    },
                ],
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            Ok(workflow)
        }
    }

    pub async fn update_case_workflow(&self, workflow: CaseWorkflow) -> ServiceResult<CaseWorkflow> {
        let mut workflows = self.workflows.lock().unwrap();
        workflows.insert(workflow.case_id, workflow.clone());
        Ok(workflow)
    }
}
