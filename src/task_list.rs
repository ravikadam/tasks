use crate::task::{Task, TaskStatus};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// TaskList manages a collection of tasks with persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskList {
    tasks: Vec<Task>,
    file_path: String,
}

// Implement Send for TaskList
unsafe impl Send for TaskList {}

/// TaskList service trait defining operations on task collections
pub trait TaskListService {
    fn new(file_path: String) -> Self;
    fn add_task(&mut self, task: Task) -> &Task;
    fn remove_task(&mut self, task_id: &str) -> Option<Task>;
    fn find_task(&self, task_id: &str) -> Option<&Task>;
    fn find_task_mut(&mut self, task_id: &str) -> Option<&mut Task>;
    fn list_tasks(&self) -> &Vec<Task>;
    fn filter_by_type(&self, task_type: &str) -> Vec<&Task>;
    fn filter_by_status(&self, status: &TaskStatus) -> Vec<&Task>;
    fn count(&self) -> usize;
    fn save(&self) -> Result<(), TaskListError>;
    fn load(&mut self) -> Result<(), TaskListError>;
    fn clear(&mut self);
    fn get_stats(&self) -> TaskListStats;
}

/// TaskList operation errors
#[derive(Debug, thiserror::Error)]
pub enum TaskListError {
    #[error("Failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),
    #[error("Failed to parse JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),
    #[error("Task not found with ID: {0}")]
    TaskNotFound(String),
}

impl TaskList {
    /// Create a new TaskList with the specified file path
    pub fn with_file_path(file_path: String) -> Self {
        let mut task_list = Self {
            tasks: Vec::new(),
            file_path,
        };
        
        // Try to load existing tasks
        if let Err(e) = task_list.load() {
            println!("Warning: Could not load existing tasks: {}", e);
        }
        
        task_list
    }

    /// Get tasks that match multiple criteria
    pub fn filter_tasks<F>(&self, predicate: F) -> Vec<&Task>
    where
        F: Fn(&Task) -> bool,
    {
        self.tasks.iter().filter(|task| predicate(task)).collect()
    }

    /// Update task status by ID
    pub fn update_task_status(&mut self, task_id: &str, status: TaskStatus) -> Result<(), TaskListError> {
        match self.find_task_mut(task_id) {
            Some(task) => {
                task.status = status;
                Ok(())
            }
            None => Err(TaskListError::TaskNotFound(task_id.to_string())),
        }
    }

    /// Get statistics about the task list
    pub fn get_stats(&self) -> TaskListStats {
        let total = self.tasks.len();
        let completed = self.filter_by_status(&TaskStatus::Completed).len();
        let pending = self.filter_by_status(&TaskStatus::Pending).len();
        let in_progress = self.filter_by_status(&TaskStatus::InProgress).len();
        let cancelled = self.filter_by_status(&TaskStatus::Cancelled).len();

        TaskListStats {
            total,
            completed,
            pending,
            in_progress,
            cancelled,
        }
    }
}

/// Statistics about a task list
#[derive(Debug, Clone)]
pub struct TaskListStats {
    pub total: usize,
    pub completed: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub cancelled: usize,
}

impl TaskListService for TaskList {
    fn new(file_path: String) -> Self {
        Self::with_file_path(file_path)
    }

    fn add_task(&mut self, task: Task) -> &Task {
        self.tasks.push(task);
        self.tasks.last().unwrap()
    }

    fn remove_task(&mut self, task_id: &str) -> Option<Task> {
        if let Some(pos) = self.tasks.iter().position(|task| task.id == task_id) {
            Some(self.tasks.remove(pos))
        } else {
            None
        }
    }

    fn find_task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == task_id)
    }

    fn find_task_mut(&mut self, task_id: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|task| task.id == task_id)
    }

    fn list_tasks(&self) -> &Vec<Task> {
        &self.tasks
    }

    fn filter_by_type(&self, task_type: &str) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|task| task.task_type == task_type)
            .collect()
    }

    fn filter_by_status(&self, status: &TaskStatus) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|task| &task.status == status)
            .collect()
    }

    fn count(&self) -> usize {
        self.tasks.len()
    }

    fn save(&self) -> Result<(), TaskListError> {
        let json_content = serde_json::to_string_pretty(&self.tasks)?;
        fs::write(&self.file_path, json_content)?;
        println!("Saved {} tasks to {}", self.tasks.len(), self.file_path);
        Ok(())
    }

    fn load(&mut self) -> Result<(), TaskListError> {
        if Path::new(&self.file_path).exists() {
            let content = fs::read_to_string(&self.file_path)?;
            self.tasks = serde_json::from_str(&content)?;
            println!("Loaded {} existing tasks.", self.tasks.len());
        } else {
            self.tasks = Vec::new();
        }
        Ok(())
    }

    fn clear(&mut self) {
        self.tasks.clear();
    }

    fn get_stats(&self) -> TaskListStats {
        TaskList::get_stats(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{TaskCreationData, TaskService};
    use std::collections::HashMap;
    use tempfile::NamedTempFile;

    #[test]
    fn test_task_list_operations() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_string_lossy().to_string();
        
        let mut task_list = TaskList::new(file_path);
        
        let task_data = TaskCreationData {
            title: "Test Task".to_string(),
            task_type: "work".to_string(),
            description: "A test task".to_string(),
            attributes: HashMap::new(),
        };
        
        let task = Task::create_task(task_data);
        let task_id = task.id.clone();
        
        // Test add task
        task_list.add_task(task);
        assert_eq!(task_list.count(), 1);
        
        // Test find task
        assert!(task_list.find_task(&task_id).is_some());
        
        // Test remove task
        let removed_task = task_list.remove_task(&task_id);
        assert!(removed_task.is_some());
        assert_eq!(task_list.count(), 0);
    }

    #[test]
    fn test_task_list_filtering() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_string_lossy().to_string();
        
        let mut task_list = TaskList::new(file_path);
        
        // Add tasks of different types
        let work_task = Task::create_task(TaskCreationData {
            title: "Work Task".to_string(),
            task_type: "work".to_string(),
            description: "Work description".to_string(),
            attributes: HashMap::new(),
        });
        
        let meeting_task = Task::create_task(TaskCreationData {
            title: "Meeting Task".to_string(),
            task_type: "meeting".to_string(),
            description: "Meeting description".to_string(),
            attributes: HashMap::new(),
        });
        
        task_list.add_task(work_task);
        task_list.add_task(meeting_task);
        
        // Test filtering by type
        let work_tasks = task_list.filter_by_type("work");
        assert_eq!(work_tasks.len(), 1);
        assert_eq!(work_tasks[0].title, "Work Task");
        
        let meeting_tasks = task_list.filter_by_type("meeting");
        assert_eq!(meeting_tasks.len(), 1);
        assert_eq!(meeting_tasks[0].title, "Meeting Task");
    }

    #[test]
    fn test_task_list_stats() {
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_string_lossy().to_string();
        
        let mut task_list = TaskList::new(file_path);
        
        let mut task1 = Task::create_task(TaskCreationData {
            title: "Task 1".to_string(),
            task_type: "work".to_string(),
            description: "Description 1".to_string(),
            attributes: HashMap::new(),
        });
        
        let task2 = Task::create_task(TaskCreationData {
            title: "Task 2".to_string(),
            task_type: "work".to_string(),
            description: "Description 2".to_string(),
            attributes: HashMap::new(),
        });
        
        task1.status = TaskStatus::Completed;
        
        task_list.add_task(task1);
        task_list.add_task(task2);
        
        let stats = task_list.get_stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.pending, 1);
    }
}
