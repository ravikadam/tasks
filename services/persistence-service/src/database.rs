use sqlx::{PgPool, Row, postgres::PgRow};
use models::{
    Case, Task, ConversationEntry, CaseWorkflow, WorkflowStep,
    UpdateCaseRequest, UpdateTaskRequest,
    CaseStatus, TaskStatus, Priority, TaskType, MessageSender, StepStatus
};
use common::{ServiceResult, ServiceError};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    // Case operations
    pub async fn create_case(&self, case: Case) -> ServiceResult<Case> {
        let row = sqlx::query(
            r#"
            INSERT INTO cases (id, title, description, status, priority, created_at, updated_at, assigned_to, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#
        )
        .bind(case.id)
        .bind(&case.title)
        .bind(&case.description)
        .bind(serde_json::to_string(&case.status)?)
        .bind(serde_json::to_string(&case.priority)?)
        .bind(case.created_at)
        .bind(case.updated_at)
        .bind(&case.assigned_to)
        .bind(&case.metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(Case {
            id: row.id,
            title: row.title,
            description: row.description,
            status: serde_json::from_str(&row.status)?,
            priority: serde_json::from_str(&row.priority)?,
            created_at: row.created_at,
            updated_at: row.updated_at,
            assigned_to: row.assigned_to,
            metadata: row.metadata,
        })
    }

    pub async fn get_case(&self, id: Uuid) -> ServiceResult<Case> {
        let row = sqlx::query(
            "SELECT id, title, description, status, priority, created_at, updated_at, assigned_to, metadata FROM cases WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("Case with id {} not found", id)))?;

        Ok(Case {
            id: row.id,
            title: row.title,
            description: row.description,
            status: serde_json::from_str(&row.status)?,
            priority: serde_json::from_str(&row.priority)?,
            created_at: row.created_at,
            updated_at: row.updated_at,
            assigned_to: row.assigned_to,
            metadata: row.metadata,
        })
    }

    pub async fn update_case(&self, id: Uuid, request: UpdateCaseRequest) -> ServiceResult<Case> {
        let mut query = "UPDATE cases SET updated_at = NOW()".to_string();
        let mut params = Vec::new();
        let mut param_count = 1;

        if let Some(title) = &request.title {
            query.push_str(&format!(", title = ${}", param_count));
            params.push(title.clone());
            param_count += 1;
        }

        if let Some(description) = &request.description {
            query.push_str(&format!(", description = ${}", param_count));
            params.push(description.clone());
            param_count += 1;
        }

        if let Some(status) = &request.status {
            query.push_str(&format!(", status = ${}", param_count));
            params.push(serde_json::to_string(status)?);
            param_count += 1;
        }

        if let Some(priority) = &request.priority {
            query.push_str(&format!(", priority = ${}", param_count));
            params.push(serde_json::to_string(priority)?);
            param_count += 1;
        }

        if let Some(assigned_to) = &request.assigned_to {
            query.push_str(&format!(", assigned_to = ${}", param_count));
            params.push(assigned_to.clone());
            param_count += 1;
        }

        query.push_str(&format!(" WHERE id = ${} RETURNING *", param_count));

        // For simplicity, we'll fetch the updated case separately
        // In a production system, you'd want to use a proper query builder
        let _ = sqlx::query(&query)
            .bind(id)
            .execute(&self.pool)
            .await?;

        self.get_case(id).await
    }

    // Task operations
    pub async fn create_task(&self, task: Task) -> ServiceResult<Task> {
        let row = sqlx::query!(
            r#"
            INSERT INTO tasks (id, case_id, title, description, task_type, status, priority, due_date, created_at, updated_at, completed_at, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
            task.id,
            task.case_id,
            task.title,
            task.description,
            serde_json::to_string(&task.task_type)?,
            serde_json::to_string(&task.status)?,
            serde_json::to_string(&task.priority)?,
            task.due_date,
            task.created_at,
            task.updated_at,
            task.completed_at,
            task.metadata
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Task {
            id: row.id,
            case_id: row.case_id,
            title: row.title,
            description: row.description,
            task_type: serde_json::from_str(&row.task_type)?,
            status: serde_json::from_str(&row.status)?,
            priority: serde_json::from_str(&row.priority)?,
            due_date: row.due_date,
            created_at: row.created_at,
            updated_at: row.updated_at,
            completed_at: row.completed_at,
            metadata: row.metadata,
        })
    }

    pub async fn get_task(&self, id: Uuid) -> ServiceResult<Task> {
        let row = sqlx::query!(
            "SELECT * FROM tasks WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("Task with id {} not found", id)))?;

        Ok(Task {
            id: row.id,
            case_id: row.case_id,
            title: row.title,
            description: row.description,
            task_type: serde_json::from_str(&row.task_type)?,
            status: serde_json::from_str(&row.status)?,
            priority: serde_json::from_str(&row.priority)?,
            due_date: row.due_date,
            created_at: row.created_at,
            updated_at: row.updated_at,
            completed_at: row.completed_at,
            metadata: row.metadata,
        })
    }

    pub async fn update_task(&self, id: Uuid, request: UpdateTaskRequest) -> ServiceResult<Task> {
        // Similar to update_case, simplified for brevity
        let mut completed_at = None;
        if let Some(TaskStatus::Completed) = &request.status {
            completed_at = Some(Utc::now());
        }

        let _ = sqlx::query!(
            r#"
            UPDATE tasks 
            SET updated_at = NOW(),
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                status = COALESCE($4, status),
                priority = COALESCE($5, priority),
                due_date = COALESCE($6, due_date),
                completed_at = COALESCE($7, completed_at)
            WHERE id = $1
            "#,
            id,
            request.title,
            request.description,
            request.status.as_ref().map(|s| serde_json::to_string(s)).transpose()?,
            request.priority.as_ref().map(|p| serde_json::to_string(p)).transpose()?,
            request.due_date,
            completed_at
        )
        .execute(&self.pool)
        .await?;

        self.get_task(id).await
    }

    pub async fn delete_task(&self, id: Uuid) -> ServiceResult<()> {
        let result = sqlx::query!(
            "DELETE FROM tasks WHERE id = $1",
            id
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(ServiceError::NotFound(format!("Task with id {} not found", id)));
        }

        Ok(())
    }

    pub async fn get_tasks_for_case(&self, case_id: Uuid) -> ServiceResult<Vec<Task>> {
        let rows = sqlx::query!(
            "SELECT * FROM tasks WHERE case_id = $1 ORDER BY created_at DESC",
            case_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(Task {
                id: row.id,
                case_id: row.case_id,
                title: row.title,
                description: row.description,
                task_type: serde_json::from_str(&row.task_type)?,
                status: serde_json::from_str(&row.status)?,
                priority: serde_json::from_str(&row.priority)?,
                due_date: row.due_date,
                created_at: row.created_at,
                updated_at: row.updated_at,
                completed_at: row.completed_at,
                metadata: row.metadata,
            });
        }

        Ok(tasks)
    }

    // Conversation operations
    pub async fn get_conversation_history(&self, case_id: Uuid) -> ServiceResult<Vec<ConversationEntry>> {
        let rows = sqlx::query!(
            "SELECT * FROM conversation_entries WHERE case_id = $1 ORDER BY timestamp ASC",
            case_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(ConversationEntry {
                id: row.id,
                case_id: row.case_id,
                message: row.message,
                sender: serde_json::from_str(&row.sender)?,
                timestamp: row.timestamp,
                metadata: row.metadata,
            });
        }

        Ok(entries)
    }

    pub async fn add_conversation_entry(&self, entry: ConversationEntry) -> ServiceResult<ConversationEntry> {
        let row = sqlx::query!(
            r#"
            INSERT INTO conversation_entries (id, case_id, message, sender, timestamp, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
            entry.id,
            entry.case_id,
            entry.message,
            serde_json::to_string(&entry.sender)?,
            entry.timestamp,
            entry.metadata
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(ConversationEntry {
            id: row.id,
            case_id: row.case_id,
            message: row.message,
            sender: serde_json::from_str(&row.sender)?,
            timestamp: row.timestamp,
            metadata: row.metadata,
        })
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
