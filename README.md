# Agentic Task Capture System (Rust)

A simple agentic system that captures tasks from natural language input using LLM-based extraction and manages them in a JSON-based task list.

## Features

- **Natural Language Processing**: Uses OpenAI's GPT-3.5-turbo to intelligently extract tasks and attributes from user input
- **Flexible Task Structure**: Supports different task types with custom attributes
- **JSON Persistence**: Saves and loads tasks from a JSON file
- **Fallback Extraction**: Works without API key using keyword-based extraction
- **Task Management**: Add, list, filter, and mark tasks as complete

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

## Usage

### Setting up OpenAI API (Optional)

For advanced LLM-based task extraction, set your OpenAI API key:

```bash
export OPENAI_API_KEY="your-api-key-here"
```

Without an API key, the system will use fallback keyword-based extraction.

### Running the Application

```bash
cargo run
```

### Commands

- **Natural Language Input**: Type your tasks naturally
  - Example: "I need to call John tomorrow at 2pm about the project meeting"
  - Example: "Buy milk, eggs, and bread from the grocery store"

- **List Tasks**: `list` - Show all tasks
- **Filter Tasks**: `list [type]` - Show tasks of specific type (e.g., `list meeting`)
- **Complete Task**: `complete [task_id]` - Mark a task as complete
- **Quit**: `quit` - Exit the system

### Example Session

```
🤖 Agentic Task Capture System (Rust)
======================================
Commands:
- Type your tasks naturally (e.g., 'I need to call John tomorrow')
- 'list' - Show all tasks
- 'list [type]' - Show tasks of specific type
- 'complete [task_id]' - Mark task as complete
- 'quit' - Exit the system
======================================

💬 Enter your input: I need to call Sarah about the meeting and buy groceries

Processing input: 'I need to call Sarah about the meeting and buy groceries'
Added task: Call Sarah about the meeting (call)
Added task: Buy groceries (shopping)
Saved 2 tasks to tasks.json
✅ Added 2 task(s) to your list!

💬 Enter your input: list

--- Task List (2 tasks) ---

ID: a1b2c3d4-e5f6-7890-abcd-ef1234567890
Title: Call Sarah about the meeting
Type: call
Status: pending
Description: Call Sarah about the meeting
Created: 2025-07-19 04:08:52 UTC
Attributes: {
  "contact_person": "Sarah",
  "purpose": "meeting discussion"
}

ID: b2c3d4e5-f6g7-8901-bcde-f23456789012
Title: Buy groceries
Type: shopping
Status: pending
Description: Buy groceries
Created: 2025-07-19 04:08:52 UTC
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

## Architecture

The system consists of:

1. **Task Structure**: Flexible data model with type-specific attributes
2. **LLM Integration**: OpenAI API integration with intelligent prompt engineering
3. **Fallback Extraction**: Keyword-based extraction when LLM is unavailable
4. **Persistence Layer**: JSON file-based storage
5. **Interactive CLI**: User-friendly command-line interface

## Future Enhancements

- Speech-to-text integration
- Task scheduling and reminders
- Task dependencies and relationships
- Export to other formats (CSV, XML)
- Web interface
- Database storage options
- Task editing capabilities
