use chrono::{DateTime, Utc};
// use clap::{Arg, Command}; // Unused for now
use dotenv::dotenv;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    id: String,
    title: String,
    task_type: String,
    description: String,
    created_at: DateTime<Utc>,
    status: String,
    attributes: HashMap<String, Value>,
}

impl Task {
    fn new(
        title: String,
        task_type: String,
        description: String,
        attributes: HashMap<String, Value>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            task_type,
            description,
            created_at: Utc::now(),
            status: "pending".to_string(),
            attributes,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ExtractedTask {
    title: String,
    task_type: String,
    description: String,
    attributes: HashMap<String, Value>,
}

struct TaskAgent {
    tasks: Vec<Task>,
    tasks_file: String,
    client: Option<Client>,
    api_key: Option<String>,
}

impl TaskAgent {
    fn new(api_key: Option<String>, tasks_file: Option<String>) -> Self {
        let tasks_file = tasks_file.unwrap_or_else(|| "tasks.json".to_string());
        let client = if api_key.is_some() {
            Some(Client::new())
        } else {
            None
        };
        
        let mut agent = Self {
            tasks: Vec::new(),
            tasks_file,
            client,
            api_key,
        };
        
        agent.load_tasks();
        agent
    }
    
    fn load_tasks(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.tasks_file) {
            match serde_json::from_str::<Vec<Task>>(&content) {
                Ok(tasks) => {
                    self.tasks = tasks;
                    println!("Loaded {} existing tasks.", self.tasks.len());
                }
                Err(e) => {
                    println!("Error loading tasks: {}", e);
                    self.tasks = Vec::new();
                }
            }
        } else {
            self.tasks = Vec::new();
        }
    }
    
    fn save_tasks(&self) {
        match serde_json::to_string_pretty(&self.tasks) {
            Ok(json_content) => {
                if let Err(e) = fs::write(&self.tasks_file, json_content) {
                    println!("Error saving tasks: {}", e);
                } else {
                    println!("Saved {} tasks to {}", self.tasks.len(), self.tasks_file);
                }
            }
            Err(e) => println!("Error serializing tasks: {}", e),
        }
    }
    
    async fn extract_tasks_with_llm(&self, user_input: &str) -> Vec<ExtractedTask> {
        if let (Some(client), Some(api_key)) = (&self.client, &self.api_key) {
            match self.call_openai_api(client, api_key, user_input).await {
                Ok(tasks) => tasks,
                Err(e) => {
                    println!("Error with LLM extraction: {}", e);
                    self.fallback_task_extraction(user_input)
                }
            }
        } else {
            println!("No OpenAI API key available. Using fallback extraction.");
            self.fallback_task_extraction(user_input)
        }
    }
    
    async fn call_openai_api(
        &self,
        client: &Client,
        api_key: &str,
        user_input: &str,
    ) -> Result<Vec<ExtractedTask>, Box<dyn std::error::Error>> {
        let system_prompt = r#"You are a task extraction AI. Your job is to analyze user input and extract tasks with their attributes.

Return a JSON array of tasks. Each task should have:
- title: Brief task title
- task_type: One of [meeting, shopping, work, personal, reminder, deadline, call, email, travel, health, finance, learning]
- description: Detailed description
- attributes: Object with relevant attributes based on task type

Common attributes by type:
- meeting: date, time, participants, location, agenda
- shopping: items, quantity, store, budget
- work: priority, deadline, assignee, project
- personal: location, reminder_time, category
- reminder: reminder_date, reminder_time
- deadline: due_date, priority
- call: contact_person, phone_number, purpose
- email: recipient, subject, priority
- travel: destination, departure_date, return_date, booking_needed
- health: appointment_date, doctor, type
- finance: amount, category, due_date
- learning: subject, duration, resources

Extract ALL tasks mentioned in the input. If no clear tasks are found, return an empty array.

Examples:
Input: "I need to call John tomorrow at 2pm about the project meeting"
Output: [{"title": "Call John about project meeting", "task_type": "call", "description": "Call John tomorrow at 2pm to discuss the project meeting", "attributes": {"contact_person": "John", "time": "2pm tomorrow", "purpose": "project meeting discussion"}}]

Input: "Buy milk, eggs, and bread from the grocery store, and don't forget to pick up the dry cleaning"
Output: [{"title": "Grocery shopping", "task_type": "shopping", "description": "Buy milk, eggs, and bread from the grocery store", "attributes": {"items": ["milk", "eggs", "bread"], "store": "grocery store"}}, {"title": "Pick up dry cleaning", "task_type": "personal", "description": "Pick up the dry cleaning", "attributes": {"location": "dry cleaning store"}}]

Only return valid JSON array, no other text."#;
        
        let payload = json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_input}
            ],
            "temperature": 0.1
        });
        
        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
        
        let response_json: Value = response.json().await?;
        
        if let Some(content) = response_json["choices"][0]["message"]["content"].as_str() {
            // Try to extract JSON from the response
            let re = Regex::new(r"\[.*\]").unwrap();
            let json_str = if let Some(captures) = re.find(content) {
                captures.as_str()
            } else {
                content
            };
            
            let extracted_tasks: Vec<ExtractedTask> = serde_json::from_str(json_str)?;
            Ok(extracted_tasks)
        } else {
            Ok(Vec::new())
        }
    }
    
    fn fallback_task_extraction(&self, user_input: &str) -> Vec<ExtractedTask> {
        let task_keywords = [
            "need to", "have to", "should", "must", "remember to", "don't forget"
        ];
        
        let mut tasks = Vec::new();
        let sentences: Vec<&str> = user_input.split('.').collect();
        
        for sentence in sentences {
            let sentence = sentence.trim();
            if task_keywords.iter().any(|keyword| sentence.to_lowercase().contains(keyword)) {
                let title = if sentence.len() > 50 {
                    format!("{}...", &sentence[..50])
                } else {
                    sentence.to_string()
                };
                
                tasks.push(ExtractedTask {
                    title,
                    task_type: "personal".to_string(),
                    description: sentence.to_string(),
                    attributes: HashMap::new(),
                });
            }
        }
        
        if tasks.is_empty() && !user_input.trim().is_empty() {
            let title = if user_input.len() > 50 {
                format!("{}...", &user_input[..50])
            } else {
                user_input.to_string()
            };
            
            tasks.push(ExtractedTask {
                title,
                task_type: "personal".to_string(),
                description: user_input.to_string(),
                attributes: HashMap::new(),
            });
        }
        
        tasks
    }
    
    fn add_task(&mut self, extracted_task: ExtractedTask) -> &Task {
        let task = Task::new(
            extracted_task.title,
            extracted_task.task_type,
            extracted_task.description,
            extracted_task.attributes,
        );
        
        self.tasks.push(task);
        self.tasks.last().unwrap()
    }
    
    async fn process_input(&mut self, user_input: &str) -> Vec<String> {
        println!("\nProcessing input: '{}'", user_input);
        
        let extracted_tasks = self.extract_tasks_with_llm(user_input).await;
        let mut added_task_ids = Vec::new();
        
        for extracted_task in extracted_tasks {
            let task = self.add_task(extracted_task);
            println!("Added task: {} ({})", task.title, task.task_type);
            added_task_ids.push(task.id.clone());
        }
        
        if !added_task_ids.is_empty() {
            self.save_tasks();
        }
        
        added_task_ids
    }
    
    fn list_tasks(&self, task_type: Option<&str>, status: Option<&str>) {
        let mut filtered_tasks: Vec<&Task> = self.tasks.iter().collect();
        
        if let Some(t_type) = task_type {
            filtered_tasks.retain(|task| task.task_type == t_type);
        }
        
        if let Some(t_status) = status {
            filtered_tasks.retain(|task| task.status == t_status);
        }
        
        if filtered_tasks.is_empty() {
            println!("No tasks found.");
            return;
        }
        
        println!("\n--- Task List ({} tasks) ---", filtered_tasks.len());
        for task in filtered_tasks {
            println!("\nID: {}", task.id);
            println!("Title: {}", task.title);
            println!("Type: {}", task.task_type);
            println!("Status: {}", task.status);
            println!("Description: {}", task.description);
            println!("Created: {}", task.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
            if !task.attributes.is_empty() {
                println!("Attributes: {}", serde_json::to_string_pretty(&task.attributes).unwrap_or_else(|_| "Error serializing attributes".to_string()));
            }
        }
    }
    
    fn mark_complete(&mut self, task_id: &str) {
        let mut task_title = None;
        
        // First, find and update the task
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = "completed".to_string();
            task_title = Some(task.title.clone());
        }
        
        // Then save and print
        if let Some(title) = task_title {
            self.save_tasks();
            println!("Task '{}' marked as complete.", title);
        } else {
            println!("Task with ID '{}' not found.", task_id);
        }
    }
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv().ok();
    
    println!("🤖 Agentic Task Capture System (Rust)");
    println!("======================================");
    println!("Commands:");
    println!("- Type your tasks naturally (e.g., 'I need to call John tomorrow')");
    println!("- 'list' - Show all tasks");
    println!("- 'list [type]' - Show tasks of specific type");
    println!("- 'complete [task_id]' - Mark task as complete");
    println!("- 'quit' - Exit the system");
    println!("======================================");
    
    // Get OpenAI API key from environment variable (now supports .env file)
    let api_key = std::env::var("OPENAI_API_KEY").ok();
    if api_key.is_none() {
        println!("Warning: No OpenAI API key found. Set OPENAI_API_KEY environment variable for LLM features.");
        println!("Fallback extraction will be used instead.");
    }
    
    let mut agent = TaskAgent::new(api_key, None);
    
    loop {
        print!("\n💬 Enter your input: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();
                
                if input.is_empty() {
                    continue;
                }
                
                if input.to_lowercase() == "quit" {
                    println!("Goodbye! 👋");
                    break;
                } else if input.to_lowercase() == "list" {
                    agent.list_tasks(None, None);
                } else if input.to_lowercase().starts_with("list ") {
                    let task_type = input[5..].trim();
                    agent.list_tasks(Some(task_type), None);
                } else if input.to_lowercase().starts_with("complete ") {
                    let task_id = input[9..].trim();
                    agent.mark_complete(task_id);
                } else {
                    // Process as task input
                    let added_task_ids = agent.process_input(input).await;
                    if !added_task_ids.is_empty() {
                        println!("✅ Added {} task(s) to your list!", added_task_ids.len());
                    } else {
                        println!("❌ No tasks were extracted from your input.");
                    }
                }
            }
            Err(e) => {
                println!("Error reading input: {}", e);
                break;
            }
        }
    }
}
