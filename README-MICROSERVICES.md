# Agentic Task Capture System - Microservices Architecture

This is a comprehensive refactor of the original Rust agentic task capture system into a modern microservices architecture with HTTP listeners, case management, and PostgreSQL persistence.

## Architecture Overview

The system consists of 5 microservices that communicate via HTTP APIs:

### 1. Channel Service (Port 8001)
- **Purpose**: Entry point for all user interactions
- **Endpoints**:
  - `POST /api/v1/message` - Process bot messages
  - `POST /api/v1/email` - Process email interactions
  - `GET /health` - Health check
- **Responsibilities**: Route user interactions to AI Agent Service

### 2. Case Management Service (Port 8002)
- **Purpose**: Manage case lifecycle, state, and workflow
- **Endpoints**:
  - `POST /api/v1/cases` - Create new case
  - `GET /api/v1/cases/{id}` - Get case details
  - `PUT /api/v1/cases/{id}/state` - Update case state
  - `GET /api/v1/cases/{id}/history` - Get conversation history
  - `POST /api/v1/cases/{id}/history` - Add conversation entry
  - `GET /api/v1/cases/{id}/workflow` - Get case workflow
  - `PUT /api/v1/cases/{id}/workflow` - Update workflow
- **Responsibilities**: Case state management, conversation history, workflow orchestration

### 3. Task Management Service (Port 8003)
- **Purpose**: Handle tasks and task lists for cases
- **Endpoints**:
  - `GET /api/v1/cases/{case_id}/tasks` - Get tasks for case
  - `POST /api/v1/cases/{case_id}/tasks` - Create task
  - `GET /api/v1/tasks/{id}` - Get task details
  - `PUT /api/v1/tasks/{id}` - Update task
  - `DELETE /api/v1/tasks/{id}` - Delete task
  - `PUT /api/v1/tasks/{id}/complete` - Mark task complete
- **Responsibilities**: Task CRUD operations, task lifecycle management

### 4. AI Agent Service (Port 8004)
- **Purpose**: Process messages, extract tasks, orchestrate updates
- **Endpoints**:
  - `POST /api/v1/process` - Process user input and orchestrate
- **Responsibilities**: 
  - LLM integration (OpenAI GPT-3.5-turbo)
  - Task extraction from natural language
  - Case creation and management orchestration
  - Fallback keyword-based extraction

### 5. Persistence Service (Port 8005)
- **Purpose**: Database operations with PostgreSQL
- **Endpoints**: Internal service-to-service communication only
- **Responsibilities**: 
  - PostgreSQL database operations
  - Data persistence for cases, tasks, conversations, workflows
  - Database migrations and schema management

## Data Models

### Core Entities
- **Case**: Main entity representing a user case/issue
- **Task**: Individual actionable items within a case
- **ConversationEntry**: Messages between user and agent
- **CaseWorkflow**: Workflow steps and lifecycle management

### Task Types
- Meeting, Shopping, Work, Personal, Research, Communication, Other

### Priorities
- Low, Medium, High, Critical

### Status Types
- Case: Open, InProgress, Waiting, Resolved, Closed
- Task: Pending, InProgress, Completed, Cancelled, OnHold

## Setup and Installation

### Prerequisites
- Rust 1.70+
- PostgreSQL 15+
- Docker & Docker Compose (optional)
- OpenAI API Key (optional, fallback available)

### Local Development Setup

1. **Clone and setup workspace**:
   ```bash
   cd /Users/ravikadam/Documents/1learning/tasks
   ```

2. **Setup PostgreSQL**:
   ```bash
   # Using Docker
   docker run --name postgres-task-agent \
     -e POSTGRES_DB=task_agent \
     -e POSTGRES_USER=postgres \
     -e POSTGRES_PASSWORD=postgres \
     -p 5432:5432 -d postgres:15
   ```

3. **Environment Configuration**:
   ```bash
   cp .env.microservices .env
   # Edit .env with your OpenAI API key and database URL
   ```

4. **Build all services**:
   ```bash
   cargo build --workspace
   ```

5. **Run database migrations**:
   ```bash
   cd services/persistence-service
   cargo run # This will run migrations automatically
   ```

6. **Start services** (in separate terminals):
   ```bash
   # Terminal 1 - Persistence Service
   cd services/persistence-service && cargo run

   # Terminal 2 - Case Management Service  
   cd services/case-management-service && cargo run

   # Terminal 3 - Task Management Service
   cd services/task-management-service && cargo run

   # Terminal 4 - AI Agent Service
   cd services/ai-agent-service && cargo run

   # Terminal 5 - Channel Service
   cd services/channel-service && cargo run
   ```

### Docker Deployment

```bash
# Build and start all services
docker-compose up --build

# Start in background
docker-compose up -d --build
```

## API Usage Examples

### Send a Message
```bash
curl -X POST http://localhost:8001/api/v1/message \
  -H "Content-Type: application/json" \
  -d '{
    "message": "I need to schedule a meeting with John tomorrow and buy groceries",
    "sender_id": "user123",
    "channel": "Bot"
  }'
```

### Get Case Tasks
```bash
curl http://localhost:8003/api/v1/cases/{case_id}/tasks
```

### Update Task Status
```bash
curl -X PUT http://localhost:8003/api/v1/tasks/{task_id} \
  -H "Content-Type: application/json" \
  -d '{
    "status": "Completed"
  }'
```

## Service Communication Flow

1. **User Input** → Channel Service
2. **Channel Service** → AI Agent Service
3. **AI Agent Service** → Case Management Service (create/update case)
4. **AI Agent Service** → Task Management Service (create tasks)
5. **All Services** → Persistence Service (data storage)

## Database Schema

### Tables
- `cases` - Case information and metadata
- `tasks` - Task details and relationships
- `conversation_entries` - Chat history
- `case_workflows` - Workflow state and steps

### Key Features
- UUID primary keys
- JSONB metadata fields
- Proper foreign key relationships
- Performance indexes
- Cascade deletes

## Monitoring and Health Checks

Each service exposes a `/health` endpoint:
- http://localhost:8001/health (Channel Service)
- http://localhost:8002/health (Case Management)
- http://localhost:8003/health (Task Management)
- http://localhost:8004/health (AI Agent)
- http://localhost:8005/health (Persistence)

## Development Notes

### Shared Libraries
- `models/` - Common data structures and API models
- `common/` - Shared utilities, error handling, HTTP client

### Key Dependencies
- **axum** - Web framework
- **sqlx** - Database operations
- **tokio** - Async runtime
- **serde** - Serialization
- **tracing** - Logging and observability

### Future Enhancements
- Authentication and authorization
- Rate limiting
- Circuit breakers
- Distributed tracing
- Metrics collection
- Event sourcing
- CQRS patterns
- GraphQL API gateway

## Troubleshooting

### Common Issues
1. **Database Connection**: Ensure PostgreSQL is running and DATABASE_URL is correct
2. **Service Communication**: Check that all services are running on correct ports
3. **OpenAI API**: Verify API key is set, fallback extraction will be used otherwise
4. **Port Conflicts**: Ensure ports 8001-8005 are available

### Logs
Each service logs to stdout with structured logging. Set `RUST_LOG=debug` for detailed logs.

## Migration from Monolith

The original monolithic structure has been preserved in the `src/` directory for reference. The new microservices architecture provides:

- **Better Scalability**: Each service can be scaled independently
- **Technology Flexibility**: Services can use different technologies
- **Fault Isolation**: Failure in one service doesn't affect others
- **Team Autonomy**: Different teams can own different services
- **Deployment Independence**: Services can be deployed separately

This represents a significant architectural evolution from the original task capture system to a production-ready microservices platform.
