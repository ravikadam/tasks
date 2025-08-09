# Multi-User Agentic Task Management System (Rust) - Microservices Architecture

A distributed, microservices-based **multi-user** agentic system that captures tasks from natural language input using LLM-based extraction. Built with Docker, PostgreSQL, user authentication, and service-to-service HTTP communication for scalability and maintainability.

## 🎯 **Multi-User Features**

- **🔐 User Authentication**: Secure registration and login with bcrypt password hashing
- **👥 Multi-Tenant Architecture**: Complete user data isolation and user-specific operations
- **🍪 Session Management**: HTTP-only cookies with 24-hour expiration
- **🎨 Modern UI**: Tailwind CSS responsive interface with login/registration pages
- **⚙️ User Configuration**: Account management and email connection interface
- **🔒 Data Security**: All tasks, cases, and conversations are user-specific with foreign key constraints

## 🏗️ Microservices Architecture

This system follows a **microservices architecture** with 6 independent services communicating via HTTP APIs:

```mermaid
graph TB
    User[👤 User/Bot/Email] --> Channel[🚪 Channel Service<br/>Port 8001<br/>Entry Point]
    
    Channel --> Case[📋 Case Management<br/>Port 8002<br/>Workflow & History]
    Channel --> AI[🤖 AI Agent Service<br/>Port 8004<br/>LLM Processing]
    
    Case --> Task[📝 Task Management<br/>Port 8003<br/>CRUD Operations]
    Case --> Persist[💾 Persistence Service<br/>Port 8005<br/>Database Layer]
    
    Task --> Persist
    AI --> Case
    AI --> Task
    AI --> OpenAI[🧠 OpenAI API<br/>GPT-3.5-turbo]
    
    Persist --> DB[(🗄️ PostgreSQL<br/>Port 5432<br/>Database)]
    
    subgraph "🐳 Docker Containers"
        Channel
        Case
        Task
        AI
        Persist
        DB
    end
    
    subgraph "🌐 External APIs"
        OpenAI
    end
    
    classDef service fill:#e1f5fe,stroke:#01579b,stroke-width:2px
    classDef database fill:#f3e5f5,stroke:#4a148c,stroke-width:2px
    classDef external fill:#fff3e0,stroke:#e65100,stroke-width:2px
    classDef user fill:#e8f5e8,stroke:#1b5e20,stroke-width:2px
    
    class Channel,Case,Task,AI,Persist service
    class DB database
    class OpenAI external
```

## 🔧 Services Overview

### 1. **Channel Service** (Port 8001)
- **Purpose**: Entry point for all external communications
- **Features**: Handles messages from bots, emails, web chat, and API calls
- **Multi-User**: Routes user-specific messages to AI Agent Service for processing

### 2. **AI Agent Service** (Port 8004)
- **Purpose**: LLM-powered intelligent task extraction and processing
- **Features**: Uses OpenAI GPT-3.5-turbo to analyze natural language and extract actionable tasks
- **Multi-User**: Creates user-specific cases and tasks with proper user isolation

### 3. **Case Management Service** (Port 8002)
- **Purpose**: Manages case workflows and conversation history
- **Features**: CRUD operations for cases, status tracking, conversation entries
- **Multi-User**: All cases are linked to specific users via `user_id` foreign keys

### 4. **Task Management Service** (Port 8003)
- **Purpose**: Handles task lifecycle and operations
- **Features**: CRUD operations for tasks, status updates, priority management
- **Multi-User**: All tasks are user-specific and isolated per user account

### 5. **Persistence Service** (Port 8005)
- **Purpose**: Database abstraction layer with authentication
- **Features**: PostgreSQL operations, user management, session handling, bcrypt password hashing
- **Multi-User**: Manages user accounts, sessions, and enforces data isolation

### 6. **Dashboard Service** (Port 8006)
- **Purpose**: Multi-user web interface with authentication
- **Features**: User login/registration, session management, user-specific task display, account configuration
- **Multi-User**: Complete authentication flow with secure session cookies and user-specific data views and filtering

### **4. AI Agent Service** (Port 8004)
- **Purpose**: LLM integration and task extraction
- **Endpoints**: `/extract`, `/process`, `/health`
- **Responsibilities**:
  - Process natural language input via OpenAI API
  - Extract structured task data from unstructured text
  - Provide fallback keyword-based extraction
  - Orchestrate multi-service workflows

### **5. Persistence Service** (Port 8005)
- **Purpose**: Database abstraction layer
- **Endpoints**: `/cases`, `/tasks`, `/conversations`, `/health`
- **Responsibilities**:
  - PostgreSQL database operations
  - Data validation and integrity
  - Query optimization and caching
  - Database migrations and schema management

### **6. Dashboard Service** (Port 8006)
- **Purpose**: Web UI for viewing pending tasks
- **Endpoints**: `/`, `/health`
- **Responsibilities**:
  - Fetch tasks from Task Management Service
  - Render pending tasks in simple HTML

## 🔄 Service Communication Flow

1. **User Input** → Channel Service receives request
2. **Case Creation** → Channel Service calls Case Management to create/retrieve case
3. **AI Processing** → Channel Service calls AI Agent Service for task extraction
4. **Task Creation** → AI Agent calls Task Management to create tasks
5. **Data Persistence** → All services use Persistence Service for database operations
6. **Response** → Channel Service returns structured response to user

## ✨ Architecture Benefits

- 🔄 **Scalable** - Each service can be scaled independently
- 🛡️ **Resilient** - Service failures don't bring down the entire system
- 🧪 **Testable** - Each service can be tested in isolation
- 🔧 **Maintainable** - Clear service boundaries and responsibilities
- 🚀 **Deployable** - Independent deployment and versioning
- 📊 **Observable** - Each service has health checks and logging
- 🔒 **Secure** - Service-to-service authentication and authorization

## 🚀 Quick Start with Docker

### Prerequisites
- Docker Desktop installed and running
- Git (to clone the repository)

### 1. Clone and Setup
```bash
# Clone the repository
git clone <your-repo-url>
cd tasks

# Copy environment template
cp .env.example .env

# Edit .env and add your OpenAI API key (optional)
# OPENAI_API_KEY=sk-your-actual-key-here
```

### 2. Start All Services
```bash
# Build and start all microservices
docker-compose up -d

# Check service status
docker ps

# View logs
docker-compose logs -f
```

### 3. Verify Services
```bash
# Check health endpoints
curl http://localhost:8001/health  # Channel Service
curl http://localhost:8002/health  # Case Management
curl http://localhost:8003/health  # Task Management
curl http://localhost:8004/health  # AI Agent Service
curl http://localhost:8005/health  # Persistence Service
curl http://localhost:8006/health  # Dashboard Service
```

### 4. View Pending Tasks
Open http://localhost:8006 in your browser to see the dashboard listing all pending tasks.

### 5. Test the System
```bash
# Create tasks via Channel Service API
curl -X POST http://localhost:8001/api/v1/message \
  -H "Content-Type: application/json" \
  -d '{"message": "I need to call John tomorrow and buy groceries", "sender_id": "user123", "channel": "API"}'

# Example successful response:
{
  "case_id": "15693dce-e42f-42d2-863f-fd02cd1719a4",
  "response": "I see you need to call John tomorrow and buy groceries. Would you like assistance with anything else?",
  "actions_taken": [
    "Created new case",
    "Added conversation entry", 
    "Created task: Call John Tomorrow",
    "Created task: Buy Groceries"
  ],
  "tasks_created": [
    "3b02d548-1839-4ffd-afcd-591feef31a2f",
    "aa313aea-2bf1-42e0-9018-6ae2bf2d6a5e"
  ],
  "tasks_updated": []
}
```

## 📡 API Documentation

### Message API Endpoint

**POST** `/api/v1/message`

#### Request Format
```json
{
  "message": "string (required)",     // The natural language message
  "sender_id": "string (required)",   // Unique identifier for the sender
  "channel": "enum (required)",       // One of: "Bot", "Email", "WebChat", "API"
  "case_id": "uuid (optional)"        // Associate with existing case
}
```

#### Response Format
```json
{
  "case_id": "uuid",                  // Case ID (new or existing)
  "response": "string",               // AI-generated response
  "actions_taken": ["string"],        // List of actions performed
  "tasks_created": ["uuid"],          // UUIDs of newly created tasks
  "tasks_updated": ["uuid"]           // UUIDs of updated tasks
}
```

#### Channel Types
- **Bot**: Messages from chatbots or automated systems
- **Email**: Messages received via email
- **WebChat**: Messages from web chat interfaces
- **API**: Direct API calls

#### Error Responses
```json
// 422 Unprocessable Entity (missing required fields)
{
  "error": {
    "code": 422,
    "message": "Validation error: missing required field 'sender_id'"
  }
}

// 502 Bad Gateway (service unavailable)
{
  "error": {
    "code": 502,
    "message": "External service error"
  }
}
```

## 🌟 Features

- **🤖 Natural Language Processing**: Uses OpenAI GPT-3.5-turbo for intelligent task extraction
- **🏗️ Microservices Architecture**: 5 independent, scalable services
- **🐳 Docker Containerization**: Easy deployment and development
- **🗄️ PostgreSQL Database**: Robust data persistence and querying
- **🔄 Service Communication**: HTTP-based inter-service communication
- **🛡️ Health Monitoring**: Built-in health checks for all services
- **📊 Structured Logging**: Comprehensive logging across all services
- **🔧 Environment Configuration**: Flexible configuration management
- **🧪 Fallback Processing**: Keyword-based extraction when OpenAI unavailable

## Task Types Supported

- **Meeting**: date, time, participants, location, agenda
- **Shopping**: items, quantity, store, budget
- **Work**: priority, deadline, assignee, project
- **Personal**: location, reminder_time, category
- **Reminder**: reminder_date, reminder_time
- **Deadline**: due_date, priority
- **Call**: contact_person, phone_number, purpose
- **Email**: recipient, subject, priority
- **Travel**: destination, departure_date, return_date, booking_needed
- **Health**: appointment_date, doctor, type
- **Finance**: amount, category, due_date
- **Learning**: subject, duration, resources

## 🐳 Docker Deployment

### Service Ports
- **Channel Service**: `localhost:8001`
- **Case Management**: `localhost:8002`
- **Task Management**: `localhost:8003`
- **AI Agent Service**: `localhost:8004`
- **Persistence Service**: `localhost:8005`
- **Dashboard Service**: `localhost:8006`
- **PostgreSQL Database**: `localhost:5432`

### Docker Commands
```bash
# Start all services
docker-compose up -d

# Stop all services
docker-compose down

# Rebuild and restart
docker-compose up -d --build

# View logs for specific service
docker-compose logs -f channel-service
docker-compose logs -f case-management-service
docker-compose logs -f task-management-service
docker-compose logs -f ai-agent-service
docker-compose logs -f persistence-service

# Scale a specific service
docker-compose up -d --scale task-management-service=3
```

### Development Setup

1. **Prerequisites**:
   - Docker Desktop
   - Git
   - (Optional) Rust toolchain for local development

2. **Clone and Setup**:
   ```bash
   git clone <repository-url>
   cd tasks
   cp .env.example .env
   # Edit .env with your configuration
   ```

3. Copy the environment template and configure your API key:
   ```bash
   # Copy the example environment file
   cp .env.example .env
   
   # Edit .env and add your OpenAI API key
   # OPENAI_API_KEY=sk-your-actual-key-here
   ```

4. Build the project:
   ```bash
   cargo build --release
   ```

## 🚀 Usage

### Setting up OpenAI API (Optional)

For advanced LLM-based task extraction, set your OpenAI API key in `.env`:

```bash
# Copy the example file
cp .env.example .env

# Edit .env and add your API key
OPENAI_API_KEY=sk-your-actual-key-here

# Optional: Customize other settings
OPENAI_MODEL=gpt-3.5-turbo
OPENAI_TEMPERATURE=0.1
TASKS_FILE=my_tasks.json
```

Without an API key, the system will use intelligent fallback keyword-based extraction.

### Running the Application

```bash
cargo run
```

### Enhanced Commands

The refactored CLI supports more commands:

- **Natural Language Input**: Type your tasks naturally
  - Example: "I need to call John tomorrow at 2pm about the project meeting"
  - Example: "Buy milk, eggs, and bread from the grocery store"

- **Task Management**:
  - `list` or `ls` - Show all tasks
  - `list [type]` - Filter by task type (e.g., `list meeting`)
  - `list [status]` - Filter by status (`list pending`, `list completed`)
  - `complete [task_id]` - Mark a task as complete
  - `stats` - Show task statistics and completion rates

- **System Commands**:
  - `help` - Show detailed help information
  - `quit` - Exit the system

### Example Session

```
🤖 Agentic Task Capture System (Rust) - Refactored
===================================================
Commands:
- Type your tasks naturally (e.g., 'I need to call John tomorrow')
- 'list' or 'ls' - Show all tasks
- 'list [type]' - Show tasks of specific type
- 'list [status]' - Show tasks by status (pending, completed, etc.)
- 'complete [task_id]' - Mark task as complete
- 'stats' - Show task statistics
- 'help' - Show this help message
- 'quit' - Exit the system
===================================================

💬 Enter your input: I need to call Sarah about the meeting and buy groceries

Processing input: 'I need to call Sarah about the meeting and buy groceries'
Added task: Call Sarah about the meeting (call)
Added task: Buy groceries (shopping)
Saved 2 tasks to tasks.json
✅ Added 2 task(s) to your list!

💬 Enter your input: stats

📊 Task Statistics
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📈 Total Tasks:      2
✅ Completed:        0
⏳ Pending:          2
🔄 In Progress:      0
❌ Cancelled:        0
🎯 Completion Rate:  0.0%
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## 🗄️ PostgreSQL Data Persistence

### Database Schema

The system uses **PostgreSQL** for permanent data storage with the following tables:

#### **Cases Table**
```sql
CREATE TABLE cases (
    id UUID PRIMARY KEY,
    title VARCHAR NOT NULL,
    description TEXT,
    status VARCHAR NOT NULL,           -- "Open", "InProgress", "Closed"
    priority VARCHAR NOT NULL,         -- "Low", "Medium", "High", "Critical"
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    assigned_to VARCHAR,
    metadata JSONB NOT NULL DEFAULT '{}'
);
```

#### **Tasks Table**
```sql
CREATE TABLE tasks (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    title VARCHAR NOT NULL,
    description TEXT,
    task_type VARCHAR NOT NULL,        -- "Work", "Meeting", "Shopping", etc.
    status VARCHAR NOT NULL,           -- "Pending", "InProgress", "Completed"
    priority VARCHAR NOT NULL,         -- "Low", "Medium", "High", "Critical"
    due_date TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    metadata JSONB NOT NULL DEFAULT '{}'
);
```

#### **Conversation Entries Table**
```sql
CREATE TABLE conversation_entries (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    sender VARCHAR NOT NULL,           -- "User", "Agent"
    timestamp TIMESTAMPTZ NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'
);
```

### Database Connection

- **Host**: `localhost:5432` (via Docker)
- **Database**: `task_agent`
- **Username**: `postgres`
- **Password**: `postgres`
- **Connection String**: `postgresql://postgres:postgres@postgres:5432/task_agent`

### Automatic Migrations

Database tables are created automatically when the persistence service starts. No manual migration is required.

### Verifying Data Persistence

```bash
# Connect to PostgreSQL container
docker exec -it tasks-postgres-1 psql -U postgres -d task_agent

# Check cases
SELECT id, title, status, priority FROM cases;

# Check tasks
SELECT id, case_id, title, task_type, status FROM tasks;

# Check conversation history
SELECT id, case_id, message, sender FROM conversation_entries;
```

## Dependencies

- `serde` - Serialization/deserialization
- `serde_json` - JSON handling
- `tokio` - Async runtime
- `reqwest` - HTTP client for OpenAI API
- `chrono` - Date/time handling
- `uuid` - Unique ID generation
- `regex` - Pattern matching
- `clap` - Command line parsing (for future CLI enhancements)

## 📁 Project Structure

```
.
├── Cargo.toml                    # Workspace configuration
├── docker-compose.yml            # Multi-service Docker configuration
├── README.md                     # This documentation
├── .env.example                  # Environment template
├── migrations/                   # Database schema migrations
│   └── *.sql
├── shared/                       # Shared libraries
│   ├── models/                   # Common data structures
│   └── common/                   # Utility functions
└── services/                     # Microservices
    ├── channel-service/          # Entry point service
    │   ├── src/main.rs
    │   ├── Cargo.toml
    │   └── Dockerfile
    ├── case-management-service/  # Case lifecycle management
    │   ├── src/main.rs
    │   ├── Cargo.toml
    │   └── Dockerfile
    ├── task-management-service/  # Task CRUD operations
    │   ├── src/main.rs
    │   ├── Cargo.toml
    │   └── Dockerfile
    ├── ai-agent-service/         # LLM integration
    │   ├── src/main.rs
    │   ├── Cargo.toml
    │   └── Dockerfile
    ├── persistence-service/      # Database layer
    │   ├── src/main.rs
    │   ├── Cargo.toml
    │   └── Dockerfile
    └── dashboard-service/        # HTML dashboard for pending tasks
        ├── src/main.rs
        ├── Cargo.toml
        └── Dockerfile
```

## 🧪 Testing

### Service-Level Testing
```bash
# Test individual services locally (requires Rust toolchain)
cd services/channel-service && cargo test
cd services/case-management-service && cargo test
cd services/task-management-service && cargo test
cd services/ai-agent-service && cargo test
cd services/persistence-service && cargo test

# Test shared libraries
cd shared/models && cargo test
cd shared/common && cargo test
```

### Integration Testing
```bash
# Start services for integration testing
docker-compose up -d

# Run integration tests (example)
curl -X POST http://localhost:8001/bot \
  -H "Content-Type: application/json" \
  -d '{"message": "Test task creation"}'

# Check health endpoints
for port in 8001 8002 8003 8004 8005 8006; do
  echo "Testing port $port:"
  curl -s http://localhost:$port/health || echo "Service on port $port not responding"
done
```

## ⚙️ Configuration

Environment variables (`.env` file):

| Variable | Default | Description |
|----------|---------|-------------|
| `OPENAI_API_KEY` | None | OpenAI API key for LLM extraction |
| `OPENAI_MODEL` | `gpt-3.5-turbo` | OpenAI model to use |
| `OPENAI_TEMPERATURE` | `0.1` | LLM response creativity (0.0-1.0) |
| `TASKS_FILE` | `tasks.json` | JSON file path for task persistence |

## 🔮 Future Enhancements

The modular architecture enables easy extension:

- **Speech-to-text integration** - Add voice input capability
- **Web interface** - Leverage existing services with a web frontend
- **Database storage** - Replace JSON with PostgreSQL/SQLite
- **Task scheduling** - Add time-based task execution and reminders
- **Team collaboration** - Multi-user task sharing
- **Plugin system** - Custom task type handlers
- **Mobile app** - Native mobile interface using the same core services
- **Task dependencies** - Support for task relationships and dependencies
- **Export capabilities** - Export to other formats (CSV, XML, etc.)
- **Task editing** - In-place task modification capabilities
