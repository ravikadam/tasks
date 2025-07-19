use crate::agent::AgentService;
use crate::task::{Task, TaskCreationData, TaskStatus};
use crate::task_list::{TaskListService, TaskListStats};
use std::io::{self, Write};

/// CLI command enumeration
#[derive(Debug, Clone)]
pub enum CliCommand {
    ProcessInput(String),
    ListTasks,
    ListTasksByType(String),
    ListTasksByStatus(String),
    CompleteTask(String),
    ShowStats,
    Help,
    Quit,
}

/// CLI service trait for user interaction
pub trait CliService {
    fn start(&mut self) -> Result<(), CliError>;
    fn parse_command(&self, input: &str) -> CliCommand;
    fn display_welcome(&self);
    fn display_help(&self);
    fn display_tasks(&self, tasks: &[&Task]);
    fn display_stats(&self, stats: &TaskListStats);
    fn prompt_input(&self) -> Result<String, CliError>;
}

/// CLI service errors
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Agent error: {0}")]
    AgentError(String),
    #[error("TaskList error: {0}")]
    TaskListError(String),
}

/// CLI implementation for task management
pub struct TaskCli<A, T>
where
    A: AgentService,
    T: TaskListService,
{
    agent: A,
    task_list: T,
}

impl<A, T> TaskCli<A, T>
where
    A: AgentService,
    T: TaskListService,
{
    /// Create a new CLI with the given agent and task list
    pub fn new(agent: A, task_list: T) -> Self {
        Self { agent, task_list }
    }

    /// Process a natural language input and extract tasks
    async fn process_input(&mut self, input: &str) -> Result<Vec<String>, CliError> {
        let extracted_tasks = self
            .agent
            .extract_tasks(input)
            .await
            .map_err(|e| CliError::AgentError(e.to_string()))?;

        let mut added_task_ids = Vec::new();

        for extracted_task in extracted_tasks {
            let task_data: TaskCreationData = extracted_task.into();
            let task = Task::new(task_data);
            println!("Added task: {} ({})", task.title, task.task_type);
            
            let task_id = task.id.clone();
            self.task_list.add_task(task);
            added_task_ids.push(task_id);
        }

        if !added_task_ids.is_empty() {
            self.task_list
                .save()
                .map_err(|e| CliError::TaskListError(e.to_string()))?;
        }

        Ok(added_task_ids)
    }

    /// Handle a CLI command
    async fn handle_command(&mut self, command: CliCommand) -> Result<bool, CliError> {
        match command {
            CliCommand::ProcessInput(input) => {
                let added_task_ids = self.process_input(&input).await?;
                if !added_task_ids.is_empty() {
                    println!("✅ Added {} task(s) to your list!", added_task_ids.len());
                } else {
                    println!("❌ No tasks were extracted from your input.");
                }
            }
            CliCommand::ListTasks => {
                let tasks: Vec<&Task> = self.task_list.list_tasks().iter().collect();
                self.display_tasks(&tasks);
            }
            CliCommand::ListTasksByType(task_type) => {
                let tasks = self.task_list.filter_by_type(&task_type);
                self.display_tasks(&tasks);
            }
            CliCommand::ListTasksByStatus(status_str) => {
                let status = match status_str.to_lowercase().as_str() {
                    "pending" => TaskStatus::Pending,
                    "completed" => TaskStatus::Completed,
                    "in_progress" | "inprogress" => TaskStatus::InProgress,
                    "cancelled" => TaskStatus::Cancelled,
                    _ => {
                        println!("Invalid status. Use: pending, completed, in_progress, cancelled");
                        return Ok(false);
                    }
                };
                let tasks = self.task_list.filter_by_status(&status);
                self.display_tasks(&tasks);
            }
            CliCommand::CompleteTask(task_id) => {
                match self.task_list.find_task_mut(&task_id) {
                    Some(task) => {
                        let title = task.title.clone();
                        task.status = TaskStatus::Completed;
                        self.task_list
                            .save()
                            .map_err(|e| CliError::TaskListError(e.to_string()))?;
                        println!("Task '{}' marked as complete.", title);
                    }
                    None => {
                        println!("Task with ID '{}' not found.", task_id);
                    }
                }
            }
            CliCommand::ShowStats => {
                let stats = self.task_list.get_stats();
                self.display_stats(&stats);
            }
            CliCommand::Help => {
                self.display_help();
            }
            CliCommand::Quit => {
                println!("Goodbye! 👋");
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl<A, T> CliService for TaskCli<A, T>
where
    A: AgentService,
    T: TaskListService,
{
    fn start(&mut self) -> Result<(), CliError> {
        self.display_welcome();
        
        loop {
            let input = self.prompt_input()?;
            let command = self.parse_command(&input);
            
            let should_quit = tokio::runtime::Runtime::new().unwrap().block_on(async {
                self.handle_command(command).await
            })?;
            
            if should_quit {
                break;
            }
        }

        Ok(())
    }

    fn parse_command(&self, input: &str) -> CliCommand {
        let input = input.trim();

        if input.is_empty() {
            return CliCommand::ProcessInput(input.to_string());
        }

        match input.to_lowercase().as_str() {
            "quit" | "exit" | "q" => CliCommand::Quit,
            "list" | "ls" => CliCommand::ListTasks,
            "stats" | "statistics" => CliCommand::ShowStats,
            "help" | "h" | "?" => CliCommand::Help,
            _ => {
                if input.to_lowercase().starts_with("list ") {
                    let parts: Vec<&str> = input.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let filter = parts[1].trim();
                        // Check if it's a status filter
                        match filter.to_lowercase().as_str() {
                            "pending" | "completed" | "in_progress" | "inprogress" | "cancelled" => {
                                CliCommand::ListTasksByStatus(filter.to_string())
                            }
                            _ => CliCommand::ListTasksByType(filter.to_string()),
                        }
                    } else {
                        CliCommand::ListTasks
                    }
                } else if input.to_lowercase().starts_with("complete ") {
                    let task_id = input[9..].trim();
                    CliCommand::CompleteTask(task_id.to_string())
                } else {
                    CliCommand::ProcessInput(input.to_string())
                }
            }
        }
    }

    fn display_welcome(&self) {
        println!("🤖 Agentic Task Capture System (Rust) - Refactored");
        println!("===================================================");
        println!("Commands:");
        println!("- Type your tasks naturally (e.g., 'I need to call John tomorrow')");
        println!("- 'list' or 'ls' - Show all tasks");
        println!("- 'list [type]' - Show tasks of specific type");
        println!("- 'list [status]' - Show tasks by status (pending, completed, etc.)");
        println!("- 'complete [task_id]' - Mark task as complete");
        println!("- 'stats' - Show task statistics");
        println!("- 'help' - Show this help message");
        println!("- 'quit' - Exit the system");
        println!("===================================================");
    }

    fn display_help(&self) {
        println!("\n📖 Help - Available Commands:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("🎯 Task Creation:");
        println!("  • Type naturally: 'I need to call John tomorrow at 2pm'");
        println!("  • Multiple tasks: 'Buy milk and schedule dentist appointment'");
        println!();
        println!("📋 Task Management:");
        println!("  • list, ls              - Show all tasks");
        println!("  • list [type]           - Filter by type (meeting, work, etc.)");
        println!("  • list [status]         - Filter by status (pending, completed)");
        println!("  • complete [task_id]    - Mark task as complete");
        println!("  • stats                 - Show task statistics");
        println!();
        println!("🔧 System:");
        println!("  • help, h, ?            - Show this help");
        println!("  • quit, exit, q         - Exit application");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("💡 Tip: Task IDs are shown when listing tasks. Copy the full ID for commands.");
    }

    fn display_tasks(&self, tasks: &[&Task]) {
        if tasks.is_empty() {
            println!("No tasks found.");
            return;
        }

        println!("\n--- Task List ({} tasks) ---", tasks.len());
        for task in tasks {
            println!("\n🆔 ID: {}", task.id);
            println!("📝 Title: {}", task.title);
            println!("🏷️  Type: {}", task.task_type);
            println!("📊 Status: {:?}", task.status);
            println!("📄 Description: {}", task.description);
            println!("📅 Created: {}", task.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
            
            if !task.attributes.is_empty() {
                println!("🔧 Attributes:");
                for (key, value) in &task.attributes {
                    println!("   • {}: {}", key, value);
                }
            }
        }
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    fn display_stats(&self, stats: &TaskListStats) {
        println!("\n📊 Task Statistics");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📈 Total Tasks:      {}", stats.total);
        println!("✅ Completed:        {}", stats.completed);
        println!("⏳ Pending:          {}", stats.pending);
        println!("🔄 In Progress:      {}", stats.in_progress);
        println!("❌ Cancelled:        {}", stats.cancelled);
        
        if stats.total > 0 {
            let completion_rate = (stats.completed as f32 / stats.total as f32) * 100.0;
            println!("🎯 Completion Rate:  {:.1}%", completion_rate);
        }
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    fn prompt_input(&self) -> Result<String, CliError> {
        print!("\n💬 Enter your input: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentConfig, TaskAgent};
    use crate::task_list::TaskList;
    use tempfile::NamedTempFile;

    #[test]
    fn test_command_parsing() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_string_lossy().to_string();
        
        let agent = TaskAgent::new(AgentConfig::default());
        let task_list = TaskList::new(file_path);
        let cli = TaskCli::new(agent, task_list);

        // Test basic commands
        assert!(matches!(cli.parse_command("quit"), CliCommand::Quit));
        assert!(matches!(cli.parse_command("list"), CliCommand::ListTasks));
        assert!(matches!(cli.parse_command("help"), CliCommand::Help));
        assert!(matches!(cli.parse_command("stats"), CliCommand::ShowStats));

        // Test parameterized commands
        if let CliCommand::ListTasksByType(task_type) = cli.parse_command("list work") {
            assert_eq!(task_type, "work");
        } else {
            panic!("Expected ListTasksByType command");
        }

        if let CliCommand::CompleteTask(task_id) = cli.parse_command("complete task-123") {
            assert_eq!(task_id, "task-123");
        } else {
            panic!("Expected CompleteTask command");
        }

        // Test natural language input
        if let CliCommand::ProcessInput(input) = cli.parse_command("I need to call John") {
            assert_eq!(input, "I need to call John");
        } else {
            panic!("Expected ProcessInput command");
        }
    }

    #[test]
    fn test_status_filtering() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_string_lossy().to_string();
        
        let agent = TaskAgent::new(AgentConfig::default());
        let task_list = TaskList::new(file_path);
        let cli = TaskCli::new(agent, task_list);

        // Test status commands
        if let CliCommand::ListTasksByStatus(status) = cli.parse_command("list pending") {
            assert_eq!(status, "pending");
        } else {
            panic!("Expected ListTasksByStatus command");
        }

        if let CliCommand::ListTasksByStatus(status) = cli.parse_command("list completed") {
            assert_eq!(status, "completed");
        } else {
            panic!("Expected ListTasksByStatus command");
        }
    }
}
