-- Cases table
CREATE TABLE cases (
    id UUID PRIMARY KEY,
    title VARCHAR NOT NULL,
    description TEXT,
    status VARCHAR NOT NULL,
    priority VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    assigned_to VARCHAR,
    metadata JSONB NOT NULL DEFAULT '{}'
);

-- Tasks table
CREATE TABLE tasks (
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
);

-- Conversation entries table
CREATE TABLE conversation_entries (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    sender VARCHAR NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'
);

-- Case workflows table
CREATE TABLE case_workflows (
    id UUID PRIMARY KEY,
    case_id UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    current_step VARCHAR NOT NULL,
    steps JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

-- Indexes for performance
CREATE INDEX idx_tasks_case_id ON tasks(case_id);
CREATE INDEX idx_conversation_entries_case_id ON conversation_entries(case_id);
CREATE INDEX idx_conversation_entries_timestamp ON conversation_entries(timestamp);
CREATE INDEX idx_case_workflows_case_id ON case_workflows(case_id);
CREATE INDEX idx_cases_status ON cases(status);
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_due_date ON tasks(due_date) WHERE due_date IS NOT NULL;
