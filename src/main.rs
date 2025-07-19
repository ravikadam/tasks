//! Agentic Task Capture System - Refactored Architecture
//! 
//! This system provides a modular, service-oriented architecture for task management
//! with LLM-based extraction capabilities.

mod agent;
mod cli;
mod task;
mod task_list;

use agent::{AgentConfig, TaskAgent};
use cli::{CliService, TaskCli};
use dotenv::dotenv;
use std::env;
use task_list::{TaskList, TaskListService};

/// Application configuration
#[derive(Debug, Clone)]
struct AppConfig {
    pub tasks_file: String,
    pub agent_config: AgentConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            tasks_file: "tasks.json".to_string(),
            agent_config: AgentConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load configuration from environment variables
    fn from_env() -> Self {
        let api_key = env::var("OPENAI_API_KEY").ok();
        let tasks_file = env::var("TASKS_FILE").unwrap_or_else(|_| "tasks.json".to_string());
        let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
        let temperature = env::var("OPENAI_TEMPERATURE")
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(0.1);
        
        let agent_config = AgentConfig {
            api_key,
            model,
            temperature,
            fallback_enabled: true,
        };
        
        Self {
            tasks_file,
            agent_config,
        }
    }
}

/// Initialize and run the application
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Load configuration
    let config = AppConfig::from_env();
    
    // Display API key status
    if config.agent_config.api_key.is_none() {
        println!("Warning: No OpenAI API key found. Set OPENAI_API_KEY environment variable for LLM features.");
        println!("Fallback extraction will be used instead.");
        println!();
    }
    
    // Initialize services
    let agent = TaskAgent::new(config.agent_config);
    let task_list = TaskList::new(config.tasks_file);
    
    // Create and start CLI
    let mut cli = TaskCli::new(agent, task_list);
    cli.start()?;
    
    Ok(())
}
