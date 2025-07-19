# Agentic Task Capture System (Rust) - Refactored

A modular, service-oriented agentic system that captures tasks from natural language input using LLM-based extraction and manages them with a clean, maintainable architecture.

## 🏗️ Architecture

This system follows a **service-oriented design** with clear separation of concerns:

### **Core Services**
- **Task Service** (`src/task.rs`) - Individual task management and validation
- **TaskList Service** (`src/task_list.rs`) - Collection management and persistence  
- **Agent Service** (`src/agent.rs`) - LLM-based task extraction
- **CLI Service** (`src/cli.rs`) - User interaction and command processing
- **Main** (`src/main.rs`) - Application orchestration and configuration

### **Key Benefits**
- ✅ **Modular** - Each service has a single responsibility
- ✅ **Testable** - Independent unit tests for each component
- ✅ **Maintainable** - Clear interfaces and error handling
- ✅ **Extensible** - Easy to add new features or swap implementations
- ✅ **Type-Safe** - Rust's type system prevents many runtime errors

## Features

- **Natural Language Processing**: Uses OpenAI's GPT-3.5-turbo to intelligently extract tasks and attributes from user input
- **Flexible Task Structure**: Supports different task types with custom attributes using `HashMap<String, Value>`
- **JSON Persistence**: Automatic saving and loading of tasks from JSON file
- **Fallback Extraction**: Intelligent keyword-based extraction when no API key available
- **Interactive CLI**: Rich command-line interface with help, statistics, and filtering
- **Environment Configuration**: `.env` file support for easy configuration management
- **Comprehensive Error Handling**: Proper error types and handling throughout the system

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

## Installation

### 1. Install Rust

If you don't have Rust installed, install it using rustup:

```bash
# Install Rust (follow the prompts)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Reload your shell environment
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### 2. Setup the Project

1. Clone or download this project

2. Navigate to the project directory:
   ```bash
   cd tasks
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

## Data Storage

Tasks are automatically saved to `tasks.json` in the current directory. The file contains:

- Task ID (UUID)
- Title and description
- Task type and status
- Creation timestamp
- Custom attributes based on task type

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
src/
├── main.rs          # Application orchestration and configuration
├── task.rs          # Task data structure, validation, and operations
├── task_list.rs     # Task collection management and persistence
├── agent.rs         # LLM-based task extraction and fallback logic
└── cli.rs           # Interactive command-line interface
```

## 🧪 Testing

Each module includes comprehensive unit tests:

```bash
# Run all tests
cargo test

# Run tests for specific modules
cargo test task::
cargo test task_list::
cargo test agent::
cargo test cli::

# Run tests with output
cargo test -- --nocapture
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
