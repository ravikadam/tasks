use sqlx::{PgPool, Row};
use models::{
    Case, Task, ConversationEntry, CaseWorkflow, WorkflowStep,
    UpdateCaseRequest, UpdateTaskRequest, StepStatus, TaskStatus,
    User, EmailAccount, UserSession, RegisterRequest, LoginRequest,
    LoginResponse, UserProfile, AddEmailAccountRequest, UpdateUserRequest,
    ChangePasswordRequest, EmailProvider, ImapSettings
};
use common::{ServiceResult, ServiceError};
use uuid::Uuid;
use chrono::Utc;
use bcrypt::{hash, verify, DEFAULT_COST};

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
        
        // Create users table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY,
                email VARCHAR UNIQUE NOT NULL,
                password_hash VARCHAR NOT NULL,
                full_name VARCHAR NOT NULL,
                organization VARCHAR,
                is_active BOOLEAN NOT NULL DEFAULT true,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                last_login TIMESTAMPTZ,
                metadata JSONB NOT NULL DEFAULT '{}'
            )
        "#)
        .execute(&self.pool)
        .await?;

        // Create user_sessions table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS user_sessions (
                id UUID PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                session_token VARCHAR UNIQUE NOT NULL,
                expires_at TIMESTAMPTZ NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                last_accessed TIMESTAMPTZ NOT NULL,
                ip_address VARCHAR,
                user_agent TEXT
            )
        "#)
        .execute(&self.pool)
        .await?;

        // Create email_accounts table
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS email_accounts (
                id UUID PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                email_address VARCHAR NOT NULL,
                provider VARCHAR NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT true,
                oauth_token TEXT,
                oauth_refresh_token TEXT,
                oauth_expires_at TIMESTAMPTZ,
                imap_settings JSONB,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                metadata JSONB NOT NULL DEFAULT '{}'
            )
        "#)
        .execute(&self.pool)
        .await?;

        // Create cases table (updated with user_id)
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS cases (
                id UUID PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
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

        // Create tasks table (updated with user_id)
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id UUID PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
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

        // Create conversation entries table (updated with user_id)
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS conversation_entries (
                id UUID PRIMARY KEY,
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
                message TEXT NOT NULL,
                sender VARCHAR NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL,
                metadata JSONB NOT NULL DEFAULT '{}'
            )
        "#)
        .execute(&self.pool)
        .await?;

        // Add user_id columns to existing tables if they don't exist
        // This handles the case where tables were created before the multi-user migration
        
        // Add user_id to cases table if it doesn't exist
        sqlx::query(r#"
            DO $$ 
            BEGIN 
                IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                              WHERE table_name='cases' AND column_name='user_id') THEN
                    ALTER TABLE cases ADD COLUMN user_id UUID REFERENCES users(id) ON DELETE CASCADE;
                END IF;
            END $$;
        "#)
        .execute(&self.pool)
        .await?;

        // Add user_id to tasks table if it doesn't exist
        sqlx::query(r#"
            DO $$ 
            BEGIN 
                IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                              WHERE table_name='tasks' AND column_name='user_id') THEN
                    ALTER TABLE tasks ADD COLUMN user_id UUID REFERENCES users(id) ON DELETE CASCADE;
                END IF;
            END $$;
        "#)
        .execute(&self.pool)
        .await?;

        // Add user_id to conversation_entries table if it doesn't exist
        sqlx::query(r#"
            DO $$ 
            BEGIN 
                IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                              WHERE table_name='conversation_entries' AND column_name='user_id') THEN
                    ALTER TABLE conversation_entries ADD COLUMN user_id UUID REFERENCES users(id) ON DELETE CASCADE;
                END IF;
            END $$;
        "#)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // User Management Operations
    pub async fn create_user(&self, request: RegisterRequest) -> ServiceResult<User> {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let password_hash = hash(&request.password, DEFAULT_COST)
            .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Password hashing error: {}", e)))?;

        let user = User {
            id: user_id,
            email: request.email.clone(),
            password_hash: password_hash.clone(),
            full_name: request.full_name.clone(),
            organization: request.organization.clone(),
            is_active: true,
            created_at: now,
            updated_at: now,
            last_login: None,
            metadata: serde_json::json!({}),
        };

        sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash, full_name, organization, is_active, created_at, updated_at, last_login, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(&user.full_name)
        .bind(&user.organization)
        .bind(user.is_active)
        .bind(user.created_at)
        .bind(user.updated_at)
        .bind(user.last_login)
        .bind(&user.metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        Ok(user)
    }

    pub async fn authenticate_user(&self, request: LoginRequest) -> ServiceResult<User> {
        let row = sqlx::query(
            "SELECT id, email, password_hash, full_name, organization, is_active, created_at, updated_at, last_login, metadata FROM users WHERE email = $1 AND is_active = true"
        )
        .bind(&request.email)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        let row = row.ok_or_else(|| ServiceError::NotFound("User not found or inactive".to_string()))?;

        let password_hash: String = row.get("password_hash");
        if !verify(&request.password, &password_hash)
            .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Password verification error: {}", e)))? {
            return Err(ServiceError::Unauthorized("Invalid credentials".to_string()));
        }

        let mut user = User {
            id: row.get("id"),
            email: row.get("email"),
            password_hash,
            full_name: row.get("full_name"),
            organization: row.get("organization"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            last_login: row.get("last_login"),
            metadata: row.get("metadata"),
        };

        // Update last login
        let now = Utc::now();
        sqlx::query("UPDATE users SET last_login = $1, updated_at = $2 WHERE id = $3")
            .bind(now)
            .bind(now)
            .bind(user.id)
            .execute(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        user.last_login = Some(now);
        user.updated_at = now;
        Ok(user)
    }

    pub async fn create_session(&self, user_id: Uuid, session_token: String, expires_at: chrono::DateTime<Utc>) -> ServiceResult<UserSession> {
        let session_id = Uuid::new_v4();
        let now = Utc::now();

        let session = UserSession {
            id: session_id,
            user_id,
            session_token: session_token.clone(),
            expires_at,
            created_at: now,
            last_accessed: now,
            ip_address: None,
            user_agent: None,
        };

        sqlx::query(
            r#"
            INSERT INTO user_sessions (id, user_id, session_token, expires_at, created_at, last_accessed, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(session.id)
        .bind(session.user_id)
        .bind(&session.session_token)
        .bind(session.expires_at)
        .bind(session.created_at)
        .bind(session.last_accessed)
        .bind(&session.ip_address)
        .bind(&session.user_agent)
        .execute(&self.pool)
        .await
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        Ok(session)
    }

    pub async fn validate_session(&self, session_token: &str) -> ServiceResult<User> {
        let row = sqlx::query(
            r#"
            SELECT u.id, u.email, u.password_hash, u.full_name, u.organization, u.is_active, 
                   u.created_at, u.updated_at, u.last_login, u.metadata
            FROM users u
            JOIN user_sessions s ON u.id = s.user_id
            WHERE s.session_token = $1 AND s.expires_at > NOW() AND u.is_active = true
            "#
        )
        .bind(session_token)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        let row = row.ok_or_else(|| ServiceError::Unauthorized("Invalid or expired session".to_string()))?;

        // Update last accessed time
        let now = Utc::now();
        sqlx::query("UPDATE user_sessions SET last_accessed = $1 WHERE session_token = $2")
            .bind(now)
            .bind(session_token)
            .execute(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        Ok(User {
            id: row.get("id"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            full_name: row.get("full_name"),
            organization: row.get("organization"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            last_login: row.get("last_login"),
            metadata: row.get("metadata"),
        })
    }

    // Case operations
    pub async fn create_case(&self, case: Case) -> ServiceResult<Case> {
        sqlx::query(
            r#"
            INSERT INTO cases (id, user_id, title, description, status, priority, created_at, updated_at, assigned_to, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(case.id)
        .bind(case.user_id)
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

    pub async fn get_case(&self, id: Uuid, user_id: Uuid) -> ServiceResult<Case> {
        let row = sqlx::query(
            "SELECT id, user_id, title, description, status, priority, created_at, updated_at, assigned_to, metadata FROM cases WHERE id = $1 AND user_id = $2"
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?
        .ok_or_else(|| ServiceError::NotFound(format!("Case with id {} not found", id)))?;

        Ok(Case {
            id: row.get("id"),
            user_id: row.get("user_id"),
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

    pub async fn update_case(&self, id: Uuid, user_id: Uuid, request: UpdateCaseRequest) -> ServiceResult<Case> {
        // Get current case first
        let mut case = self.get_case(id, user_id).await?;
        
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
            user_id: row.get("user_id"),
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
                user_id: row.get("user_id"),
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

    pub async fn get_all_tasks(&self) -> ServiceResult<Vec<Task>> {
        let rows = sqlx::query("SELECT * FROM tasks ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(Task {
                id: row.get("id"),
                user_id: row.get("user_id"),
                case_id: row.get("case_id"),
                title: row.get("title"),
                description: row.get("description"),
                task_type: serde_json::from_str(&row.get::<String, _>("task_type"))
                    .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
                status: serde_json::from_str(&row.get::<String, _>("status"))
                    .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
                priority: serde_json::from_str(&row.get::<String, _>("priority"))
                    .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
                due_date: row.get("due_date"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                completed_at: row.get("completed_at"),
                metadata: row.get("metadata"),
            });
        }

        Ok(tasks)
    }

    pub async fn get_tasks_by_status(&self, status: TaskStatus) -> ServiceResult<Vec<Task>> {
        let status_str = serde_json::to_string(&status)
            .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Serialization error: {}", e)))?;
        let rows = sqlx::query("SELECT * FROM tasks WHERE status = $1 ORDER BY created_at DESC")
            .bind(status_str)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Database error: {}", e)))?;

        let mut tasks = Vec::new();
        for row in rows {
            tasks.push(Task {
                id: row.get("id"),
                user_id: row.get("user_id"),
                case_id: row.get("case_id"),
                title: row.get("title"),
                description: row.get("description"),
                task_type: serde_json::from_str(&row.get::<String, _>("task_type"))
                    .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
                status: serde_json::from_str(&row.get::<String, _>("status"))
                    .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
                priority: serde_json::from_str(&row.get::<String, _>("priority"))
                    .map_err(|e| ServiceError::Internal(anyhow::anyhow!("Deserialization error: {}", e)))?,
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
                user_id: row.get("user_id"),
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
