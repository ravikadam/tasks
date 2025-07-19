use crate::task::TaskCreationData;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Configuration for the agent service
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub fallback_enabled: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: "gpt-3.5-turbo".to_string(),
            temperature: 0.1,
            fallback_enabled: true,
        }
    }
}

/// Extracted task data from user input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedTaskData {
    pub title: String,
    pub task_type: String,
    pub description: String,
    pub attributes: HashMap<String, Value>,
}

impl From<ExtractedTaskData> for TaskCreationData {
    fn from(extracted: ExtractedTaskData) -> Self {
        Self {
            title: extracted.title,
            task_type: extracted.task_type,
            description: extracted.description,
            attributes: extracted.attributes,
        }
    }
}

/// Agent service trait for task extraction
#[async_trait::async_trait]
pub trait AgentService {
    async fn extract_tasks(&self, input: &str) -> Result<Vec<ExtractedTaskData>, AgentError>;
    fn set_config(&mut self, config: AgentConfig);
    fn get_config(&self) -> &AgentConfig;
}

/// Agent service errors
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("LLM API error: {0}")]
    LlmApiError(String),
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("No API key provided")]
    NoApiKey,
    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),
}

/// TaskAgent handles LLM-based task extraction
pub struct TaskAgent {
    config: AgentConfig,
    client: Option<Client>,
}

// Implement Send for TaskAgent
unsafe impl Send for TaskAgent {}

impl TaskAgent {
    /// Create a new TaskAgent with the given configuration
    pub fn new(config: AgentConfig) -> Self {
        let client = if config.api_key.is_some() {
            Some(Client::new())
        } else {
            None
        };

        Self { config, client }
    }

    /// Extract tasks using LLM API
    async fn extract_with_llm(&self, input: &str) -> Result<Vec<ExtractedTaskData>, AgentError> {
        let client = self.client.as_ref().ok_or(AgentError::NoApiKey)?;
        let api_key = self.config.api_key.as_ref().ok_or(AgentError::NoApiKey)?;

        let system_prompt = self.get_system_prompt();
        
        let payload = json!({
            "model": self.config.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": input}
            ],
            "temperature": self.config.temperature
        });

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let response_json: Value = response.json().await?;

        if let Some(error) = response_json.get("error") {
            return Err(AgentError::LlmApiError(error.to_string()));
        }

        if let Some(content) = response_json["choices"][0]["message"]["content"].as_str() {
            self.parse_llm_response(content)
        } else {
            Err(AgentError::ExtractionFailed("No content in response".to_string()))
        }
    }

    /// Parse LLM response and extract JSON
    fn parse_llm_response(&self, content: &str) -> Result<Vec<ExtractedTaskData>, AgentError> {
        // Try to extract JSON from the response
        let re = Regex::new(r"\[.*\]").unwrap();
        let json_str = if let Some(captures) = re.find(content) {
            captures.as_str()
        } else {
            content
        };

        let extracted_tasks: Vec<ExtractedTaskData> = serde_json::from_str(json_str)?;
        Ok(extracted_tasks)
    }

    /// Fallback extraction using keyword matching
    fn extract_with_fallback(&self, input: &str) -> Vec<ExtractedTaskData> {
        let task_keywords = [
            "need to", "have to", "should", "must", "remember to", "don't forget",
            "schedule", "plan to", "going to", "will", "want to"
        ];

        let mut tasks = Vec::new();
        let sentences: Vec<&str> = input.split('.').collect();

        for sentence in sentences {
            let sentence = sentence.trim();
            if task_keywords.iter().any(|keyword| sentence.to_lowercase().contains(keyword)) {
                let title = if sentence.len() > 50 {
                    format!("{}...", &sentence[..50])
                } else {
                    sentence.to_string()
                };

                tasks.push(ExtractedTaskData {
                    title,
                    task_type: self.infer_task_type(sentence),
                    description: sentence.to_string(),
                    attributes: HashMap::new(),
                });
            }
        }

        // If no keywords found, treat the entire input as a task
        if tasks.is_empty() && !input.trim().is_empty() {
            let title = if input.len() > 50 {
                format!("{}...", &input[..50])
            } else {
                input.to_string()
            };

            tasks.push(ExtractedTaskData {
                title,
                task_type: self.infer_task_type(input),
                description: input.to_string(),
                attributes: HashMap::new(),
            });
        }

        tasks
    }

    /// Infer task type from text content
    fn infer_task_type(&self, text: &str) -> String {
        let text_lower = text.to_lowercase();
        
        if text_lower.contains("meeting") || text_lower.contains("call") || text_lower.contains("conference") {
            "meeting".to_string()
        } else if text_lower.contains("buy") || text_lower.contains("shop") || text_lower.contains("purchase") {
            "shopping".to_string()
        } else if text_lower.contains("email") || text_lower.contains("mail") || text_lower.contains("respond") {
            "email".to_string()
        } else if text_lower.contains("work") || text_lower.contains("project") || text_lower.contains("task") {
            "work".to_string()
        } else if text_lower.contains("doctor") || text_lower.contains("appointment") || text_lower.contains("health") {
            "health".to_string()
        } else if text_lower.contains("travel") || text_lower.contains("trip") || text_lower.contains("flight") {
            "travel".to_string()
        } else if text_lower.contains("learn") || text_lower.contains("study") || text_lower.contains("course") {
            "learning".to_string()
        } else if text_lower.contains("deadline") || text_lower.contains("due") {
            "deadline".to_string()
        } else if text_lower.contains("remind") || text_lower.contains("remember") {
            "reminder".to_string()
        } else {
            "personal".to_string()
        }
    }

    /// Get the system prompt for LLM
    fn get_system_prompt(&self) -> &str {
        r#"You are a task extraction AI. Your job is to analyze user input and extract tasks with their attributes.

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

Only return valid JSON array, no other text."#
    }
}

#[async_trait::async_trait]
impl AgentService for TaskAgent {
    async fn extract_tasks(&self, input: &str) -> Result<Vec<ExtractedTaskData>, AgentError> {
        println!("Processing input: '{}'", input);

        // Try LLM extraction first if API key is available
        if self.config.api_key.is_some() {
            match self.extract_with_llm(input).await {
                Ok(tasks) => return Ok(tasks),
                Err(e) => {
                    println!("LLM extraction failed: {}", e);
                    if !self.config.fallback_enabled {
                        return Err(e);
                    }
                }
            }
        }

        // Use fallback extraction
        if self.config.fallback_enabled {
            println!("Using fallback extraction");
            Ok(self.extract_with_fallback(input))
        } else {
            Err(AgentError::NoApiKey)
        }
    }

    fn set_config(&mut self, config: AgentConfig) {
        let needs_new_client = config.api_key != self.config.api_key;
        self.config = config;
        
        if needs_new_client {
            self.client = if self.config.api_key.is_some() {
                Some(Client::new())
            } else {
                None
            };
        }
    }

    fn get_config(&self) -> &AgentConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_type_inference() {
        let agent = TaskAgent::new(AgentConfig::default());
        
        assert_eq!(agent.infer_task_type("I need to attend a meeting"), "meeting");
        assert_eq!(agent.infer_task_type("Buy groceries from the store"), "shopping");
        assert_eq!(agent.infer_task_type("Respond to email from client"), "email");
        assert_eq!(agent.infer_task_type("Complete work project"), "work");
        assert_eq!(agent.infer_task_type("Random personal task"), "personal");
    }

    #[tokio::test]
    async fn test_fallback_extraction() {
        let agent = TaskAgent::new(AgentConfig::default());
        
        let result = agent.extract_tasks("I need to call John tomorrow").await;
        assert!(result.is_ok());
        
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title, "I need to call John tomorrow");
        assert_eq!(tasks[0].task_type, "meeting");
    }

    #[test]
    fn test_agent_config() {
        let mut config = AgentConfig::default();
        config.api_key = Some("test-key".to_string());
        config.temperature = 0.5;
        
        let mut agent = TaskAgent::new(config.clone());
        assert_eq!(agent.get_config().temperature, 0.5);
        
        config.temperature = 0.8;
        agent.set_config(config);
        assert_eq!(agent.get_config().temperature, 0.8);
    }
}
