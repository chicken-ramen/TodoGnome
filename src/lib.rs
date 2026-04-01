use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TodoError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Event store error: {0}")]
    EventStore(String),
}

pub type Result<T> = std::result::Result<T, TodoError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stakeholder {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub due_date: Option<DateTime<Utc>>,
    pub stakeholders: Vec<Stakeholder>,
    pub completed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source_url: Option<String>,
    pub tags: Vec<String>,
}

impl TodoItem {
    pub fn new(
        title: String,
        description: Option<String>,
        priority: Priority,
        due_date: Option<DateTime<Utc>>,
        stakeholders: Vec<Stakeholder>,
        source_url: Option<String>,
        tags: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            description,
            priority,
            due_date,
            stakeholders,
            completed: false,
            created_at: now,
            updated_at: now,
            source_url,
            tags,
        }
    }
    
    pub fn mark_completed(&mut self) {
        self.completed = true;
        self.updated_at = Utc::now();
    }
    
    pub fn is_due_today(&self) -> bool {
        match self.due_date {
            Some(due) => {
                let today = Utc::now().date_naive();
                due.date_naive() == today
            }
            None => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TodoEvent {
    TodoAdded(TodoItem),
    TodoCompleted { id: Uuid, completed_at: DateTime<Utc> },
    TodoUpdated { id: Uuid, updates: HashMap<String, serde_json::Value> },
    TodoDeleted { id: Uuid, deleted_at: DateTime<Utc> },
}

impl TodoEvent {
    pub fn id(&self) -> Uuid {
        match self {
            TodoEvent::TodoAdded(item) => item.id,
            TodoEvent::TodoCompleted { id, .. } => *id,
            TodoEvent::TodoUpdated { id, .. } => *id,
            TodoEvent::TodoDeleted { id, .. } => *id,
        }
    }
    
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            TodoEvent::TodoAdded(item) => item.created_at,
            TodoEvent::TodoCompleted { completed_at, .. } => *completed_at,
            TodoEvent::TodoUpdated { .. } => Utc::now(),
            TodoEvent::TodoDeleted { deleted_at, .. } => *deleted_at,
        }
    }
}

pub struct EventStore {
    data_dir: String,
}

impl EventStore {
    pub fn new(data_dir: impl AsRef<str>) -> Self {
        Self {
            data_dir: data_dir.as_ref().to_string(),
        }
    }
    
    pub fn ensure_data_dir(&self) -> Result<()> {
        let path = Path::new(&self.data_dir);
        if !path.exists() {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }
    
    pub fn append_event(&self, event: &TodoEvent) -> Result<()> {
        self.ensure_data_dir()?;
        
        let file_path = Path::new(&self.data_dir).join(format!("events_{}.json", event.timestamp().timestamp()));
        let content = serde_json::to_string_pretty(event)?;
        fs::write(file_path, content)?;
        Ok(())
    }
    
    pub fn load_all_events(&self) -> Result<Vec<TodoEvent>> {
        self.ensure_data_dir()?;
        
        let mut events = Vec::new();
        let path = Path::new(&self.data_dir);
        
        if !path.exists() {
            return Ok(events);
        }
        
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().map(|ext| ext == "json").unwrap_or(false) {
                let content = fs::read_to_string(&path)?;
                let event: TodoEvent = serde_json::from_str(&content)?;
                events.push(event);
            }
        }
        
        events.sort_by_key(|e| e.timestamp());
        Ok(events)
    }
    
    pub fn rebuild_state(&self) -> Result<Vec<TodoItem>> {
        let events = self.load_all_events()?;
        let mut items = HashMap::new();
        
        for event in events {
            match event {
                TodoEvent::TodoAdded(item) => {
                    items.insert(item.id, item);
                }
                TodoEvent::TodoCompleted { id, .. } => {
                    if let Some(item) = items.get_mut(&id) {
                        item.mark_completed();
                    }
                }
                TodoEvent::TodoUpdated { id, updates } => {
                    // For simplicity, we'll just reload all events to get latest state
                    // In production, you'd apply updates to the specific fields
                }
                TodoEvent::TodoDeleted { id, .. } => {
                    items.remove(&id);
                }
            }
        }
        
        Ok(items.into_values().collect())
    }
}