use sqlx::{PgPool, Row};
use models::{
    Case, Task, ConversationEntry, CaseWorkflow, WorkflowStep,
    UpdateCaseRequest, UpdateTaskRequest, StepStatus
};
use common::{ServiceResult, ServiceError};
use uuid::Uuid;
use chrono::Utc;

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Run migrations manually, executing each statement separately
        
        // Create cases table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS cases (
                id UUID PRIMARY KEY,
                title VARCHAR NOT NULL,
                description TEXT,
                status VARCHAR NOT NULL,
                priority VARCHAR NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                assigned_to VARCHAR,
                metadata JSONB NOT NULL DEFAULT '{}'
            )
        "#)
        .execute(&self.pool)
        .await?;

        // Create tasks table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id UUID PRIMARY KEY,
                case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
                title VARCHAR NOT NULL,
                description TEXT,
                task_type VARCHAR NOT NULL,
                status VARCHAR NOT NULL,
                priority VARCHAR NOT NULL,
                due_date TIMESTAMPTZ,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                completed_at TIMESTAMPTZ,
                metadata JSONB NOT NULL DEFAULT '{}'
            )
        "#)
        .execute(&self.pool)
        .await?;

        // Create conversation entries table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS conversation_entries (
                id UUID PRIMARY KEY,
                case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
                message TEXT NOT NULL,
                sender VARCHAR NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL,
                metadata JSONB NOT NULL DEFAULT '{}'
            )
        "#)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Case operations
    pub async fn create_case(&self, case: Case) -> ServiceResult<Case> {
        sqlx::query(
            r#"
            INSERT INTO cases (id, title, description, status, priority, created_at, updated_at, assigned_to, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#
        )
        .bind(case.id)
        .bind(&case.title)
        .bind(&case.description)
        .bind(serde_json::to_string(&case.status).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?)
        .bind(serde_json::to_string(&case.priority).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?)
        .bind(case.created_at)
        .bind(case.updated_at)
        .bind(&case.assigned_to)
        .bind(&case.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        Ok(case)
    }

    pub async fn get_case(&self, id: Uuid) -> ServiceResult<Case> {
        let row = sqlx::query(
            "SELECT id, title, description, status, priority, created_at, updated_at, assigned_to, metadata FROM cases WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| ServiceError::NotFound(format!("Case with id {} not found", id)))?;

        Ok(Case {
            id: row.get("id"),
            title: row.get("title"),
            description: row.get("description"),
            status: serde_json::from_str(&row.get::<String, _>("status")).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
            priority: serde_json::from_str(&row.get::<String, _>("priority")).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            assigned_to: row.get("assigned_to"),
            metadata: row.get("metadata"),
        })
    }

    pub async fn update_case(&self, id: Uuid, request: UpdateCaseRequest) -> ServiceResult<Case> {
        // Get current case first
        let mut case = self.get_case(id).await?;
        
        // Update fields
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

        // Update in database
        sqlx::query(
            r#"
            UPDATE cases 
            SET title = $2, description = $3, status = $4, priority = $5, updated_at = $6, assigned_to = $7
            WHERE id = $1
            "#
        )
        .bind(id)
        .bind(&case.title)
        .bind(&case.description)
        .bind(serde_json::to_string(&case.status).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?)
        .bind(serde_json::to_string(&case.priority).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?)
        .bind(case.updated_at)
        .bind(&case.assigned_to)
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        Ok(case)
    }

    // Task operations
    pub async fn create_task(&self, task: Task) -> ServiceResult<Task> {
        sqlx::query(
            r#"
            INSERT INTO tasks (id, case_id, title, description, task_type, status, priority, due_date, created_at, updated_at, completed_at, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(task.id)
        .bind(task.case_id)
        .bind(&task.title)
        .bind(&task.description)
        .bind(serde_json::to_string(&task.task_type).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?)
        .bind(serde_json::to_string(&task.status).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?)
        .bind(serde_json::to_string(&task.priority).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?)
        .bind(task.due_date)
        .bind(task.created_at)
        .bind(task.updated_at)
        .bind(task.completed_at)
        .bind(&task.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        Ok(task)
    }

    pub async fn get_task(&self, id: Uuid) -> ServiceResult<Task> {
        let row = sqlx::query("SELECT * FROM tasks WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?
            .ok_or_else(|| ServiceError::NotFound(format!("Task with id {} not found", id)))?;

        Ok(Task {
            id: row.get("id"),
            case_id: row.get("case_id"),
            title: row.get("title"),
            description: row.get("description"),
            task_type: serde_json::from_str(&row.get::<String, _>("task_type")).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
            status: serde_json::from_str(&row.get::<String, _>("status")).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
            priority: serde_json::from_str(&row.get::<String, _>("priority")).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
            due_date: row.get("due_date"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            completed_at: row.get("completed_at"),
            metadata: row.get("metadata"),
        })
    }

    pub async fn update_task(&self, id: Uuid, request: UpdateTaskRequest) -> ServiceResult<Task> {
        let mut task = self.get_task(id).await?;
        
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

        sqlx::query(
            r#"
            UPDATE tasks 
            SET title = $2, description = $3, status = $4, priority = $5, due_date = $6, updated_at = $7, completed_at = $8
            WHERE id = $1
            "#
        )
        .bind(id)
        .bind(&task.title)
        .bind(&task.description)
        .bind(serde_json::to_string(&task.status).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?)
        .bind(serde_json::to_string(&task.priority).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?)
        .bind(task.due_date)
        .bind(task.updated_at)
        .bind(task.completed_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        Ok(task)
    }

    pub async fn delete_task(&self, id: Uuid) -> ServiceResult<()> {
        let result = sqlx::query("DELETE FROM tasks WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ServiceError::NotFound(format!("Task with id {} not found", id)));
        }

        Ok(())
    }

    pub async fn get_tasks_for_case(&self, case_id: Uuid) -> ServiceResult<Vec<Task>> {
        let rows = sqlx::query("SELECT * FROM tasks WHERE case_id = $1 ORDER BY created_at DESC")
            .bind(case_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(Task {
                id: row.get("id"),
                case_id: row.get("case_id"),
                title: row.get("title"),
                description: row.get("description"),
                task_type: serde_json::from_str(&row.get::<String, _>("task_type")).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
                status: serde_json::from_str(&row.get::<String, _>("status")).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
                priority: serde_json::from_str(&row.get::<String, _>("priority")).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
                due_date: row.get("due_date"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                completed_at: row.get("completed_at"),
                metadata: row.get("metadata"),
            });
        }

        Ok(tasks)
    }

    // Conversation operations
    pub async fn get_conversation_history(&self, case_id: Uuid) -> ServiceResult<Vec<ConversationEntry>> {
        let rows = sqlx::query("SELECT * FROM conversation_entries WHERE case_id = $1 ORDER BY timestamp ASC")
            .bind(case_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(ConversationEntry {
                id: row.get("id"),
                case_id: row.get("case_id"),
                message: row.get("message"),
                sender: serde_json::from_str(&row.get::<String, _>("sender")).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
                timestamp: row.get("timestamp"),
                metadata: row.get("metadata"),
            });
        }

        Ok(entries)
    }

    pub async fn add_conversation_entry(&self, entry: ConversationEntry) -> ServiceResult<ConversationEntry> {
        sqlx::query(
            r#"
            INSERT INTO conversation_entries (id, case_id, message, sender, timestamp, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#
        )
        .bind(entry.id)
        .bind(entry.case_id)
        .bind(&entry.message)
        .bind(serde_json::to_string(&entry.sender).map_err(|e| ServiceError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?)
        .bind(entry.timestamp)
        .bind(&entry.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        Ok(entry)
    }

    // Workflow operations (simplified)
    pub async fn get_case_workflow(&self, case_id: Uuid) -> ServiceResult<CaseWorkflow> {
        // For now, return a default workflow - in production, this would be stored in DB
        Ok(CaseWorkflow {
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
        })
    }

    pub async fn update_case_workflow(&self, workflow: CaseWorkflow) -> ServiceResult<CaseWorkflow> {
        // For now, just return the workflow - in production, this would update the DB
        Ok(workflow)
    }
}
