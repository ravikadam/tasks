use serde::{Deserialize, Serialize};
use models::{TaskType, Priority};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use regex::Regex;
use tracing::{info, warn, error};

#[derive(Clone)]
pub struct LLMClient {
    api_key: Option<String>,
    client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AIResponse {
    pub response: String,
    pub tasks: Vec<TaskData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskData {
    pub title: String,
    pub description: Option<String>,
    pub task_type: TaskType,
    pub priority: Priority,
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}



impl LLMClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn process_message(&self, message: &str, case_id: Uuid) -> Result<AIResponse, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(api_key) = &self.api_key {
            self.process_with_openai(message, case_id, api_key).await
        } else {
            warn!("No OpenAI API key available, using fallback extraction");
            Ok(self.fallback_extraction(message))
        }
    }

    async fn process_with_openai(&self, message: &str, case_id: Uuid, api_key: &str) -> Result<AIResponse, Box<dyn std::error::Error + Send + Sync>> {
        let system_prompt = r#"
You are an intelligent task extraction agent. Analyze the user's message and:
1. Extract actionable tasks from the message
2. Classify each task by type (Meeting, Shopping, Work, Personal, Research, Communication, Other)
3. Assign priority (Low, Medium, High, Critical)
4. Suggest due dates if mentioned or implied
5. Provide a helpful response to the user

Respond in JSON format:
{
    "response": "Your helpful response to the user",
    "tasks": [
        {
            "title": "Task title",
            "description": "Optional description",
            "task_type": "Work|Meeting|Shopping|Personal|Research|Communication|Other",
            "priority": "Low|Medium|High|Critical",
            "due_date": "2024-01-01T10:00:00Z" // optional ISO format
        }
    ]
}
"#;

        let request = OpenAIRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![
                OpenAIMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                OpenAIMessage {
                    role: "user".to_string(),
                    content: format!("Case ID: {}\nMessage: {}", case_id, message),
                },
            ],
            temperature: 0.7,
        };

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            error!("OpenAI API error: {}", response.status());
            return Ok(self.fallback_extraction(message));
        }

        let openai_response: OpenAIResponse = response.json().await?;
        
        if let Some(choice) = openai_response.choices.first() {
            match serde_json::from_str::<AIResponse>(&choice.message.content) {
                Ok(ai_response) => {
                    info!("Successfully parsed OpenAI response");
                    Ok(ai_response)
                }
                Err(e) => {
                    warn!("Failed to parse OpenAI response: {}, using fallback", e);
                    Ok(self.fallback_extraction(message))
                }
            }
        } else {
            warn!("No choices in OpenAI response, using fallback");
            Ok(self.fallback_extraction(message))
        }
    }

    fn fallback_extraction(&self, message: &str) -> AIResponse {
        let mut tasks = Vec::new();
        let message_lower = message.to_lowercase();

        // Simple keyword-based task extraction
        let task_patterns = vec![
            (r"(?i)\b(schedule|book|arrange)\s+(.+?)(?:\s+(?:for|on|at)\s+(.+?))?(?:\.|$)", TaskType::Meeting),
            (r"(?i)\b(buy|purchase|get|shop for)\s+(.+?)(?:\.|$)", TaskType::Shopping),
            (r"(?i)\b(call|email|contact|reach out to)\s+(.+?)(?:\.|$)", TaskType::Communication),
            (r"(?i)\b(research|look into|investigate|find out about)\s+(.+?)(?:\.|$)", TaskType::Research),
            (r"(?i)\b(complete|finish|work on|do)\s+(.+?)(?:\.|$)", TaskType::Work),
            (r"(?i)\b(remind me to|need to|have to|must)\s+(.+?)(?:\.|$)", TaskType::Personal),
        ];

        for (pattern, task_type) in task_patterns {
            let re = Regex::new(pattern).unwrap();
            for cap in re.captures_iter(message) {
                if let Some(task_desc) = cap.get(2) {
                    let title = task_desc.as_str().trim().to_string();
                    if !title.is_empty() && title.len() > 2 {
                        let priority = self.determine_priority(&message_lower);
                        
                        tasks.push(TaskData {
                            title: self.clean_task_title(&title),
                            description: Some(message.to_string()),
                            task_type: task_type.clone(),
                            priority,
                            due_date: self.extract_due_date(message),
                        });
                    }
                }
            }
        }

        // If no specific patterns matched, create a general task
        if tasks.is_empty() && message.len() > 10 {
            tasks.push(TaskData {
                title: self.extract_general_task_title(message),
                description: Some(message.to_string()),
                task_type: TaskType::Personal,
                priority: Priority::Medium,
                due_date: None,
            });
        }

        AIResponse {
            response: self.generate_response(&tasks, message),
            tasks,
        }
    }

    fn determine_priority(&self, message: &str) -> Priority {
        if message.contains("urgent") || message.contains("asap") || message.contains("immediately") {
            Priority::Critical
        } else if message.contains("important") || message.contains("priority") {
            Priority::High
        } else if message.contains("when you can") || message.contains("no rush") {
            Priority::Low
        } else {
            Priority::Medium
        }
    }

    fn extract_due_date(&self, message: &str) -> Option<DateTime<Utc>> {
        // Simple date extraction - in a real implementation, use a proper date parsing library
        let today = Utc::now();
        
        if message.to_lowercase().contains("today") {
            Some(today)
        } else if message.to_lowercase().contains("tomorrow") {
            Some(today + chrono::Duration::days(1))
        } else if message.to_lowercase().contains("next week") {
            Some(today + chrono::Duration::weeks(1))
        } else {
            None
        }
    }

    fn clean_task_title(&self, title: &str) -> String {
        title.trim_end_matches(&['.', ',', ';', ':', '!', '?'])
            .trim()
            .to_string()
    }

    fn extract_general_task_title(&self, message: &str) -> String {
        let words: Vec<&str> = message.split_whitespace().take(8).collect();
        let title = words.join(" ");
        
        if title.len() > 60 {
            format!("{}...", &title[..57])
        } else {
            title
        }
    }

    fn generate_response(&self, tasks: &[TaskData], original_message: &str) -> String {
        if tasks.is_empty() {
            "I've noted your message. How can I help you further?".to_string()
        } else if tasks.len() == 1 {
            format!("I've created a task for you: '{}'. Is there anything else you need help with?", tasks[0].title)
        } else {
            format!("I've created {} tasks based on your message. They include: {}. Let me know if you need any adjustments!", 
                tasks.len(),
                tasks.iter().map(|t| t.title.as_str()).collect::<Vec<_>>().join(", ")
            )
        }
    }
}
