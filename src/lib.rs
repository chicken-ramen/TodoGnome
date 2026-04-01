use chrono::{DateTime, Utc};
use rusqlite::{Connection, params, OptionalExtension};
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
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
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
    db_path: String,
}

impl EventStore {
    pub fn new(data_dir: impl AsRef<str>) -> Self {
        let db_path = Path::new(data_dir.as_ref()).join("todognome.db").to_string_lossy().to_string();
        Self { db_path }
    }
    
    fn get_connection(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)?;
        Self::init_schema(&conn)?;
        Ok(conn)
    }
    
    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                timestamp DATETIME NOT NULL,
                payload TEXT NOT NULL,
                UNIQUE(event_id)
            )",
            [],
        )?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS current_state (
                todo_id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT,
                priority TEXT NOT NULL,
                due_date DATETIME,
                completed INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                source_url TEXT,
                tags TEXT,
                stakeholders TEXT,
                FOREIGN KEY (todo_id) REFERENCES events(event_id)
            )",
            [],
        )?;
        Ok(())
    }
    
    pub fn append_event(&self, event: &TodoEvent) -> Result<()> {
        let conn = self.get_connection()?;
        let event_id = event.id();
        let event_type = match event {
            TodoEvent::TodoAdded(_) => "todo_added",
            TodoEvent::TodoCompleted { .. } => "todo_completed",
            TodoEvent::TodoUpdated { .. } => "todo_updated",
            TodoEvent::TodoDeleted { .. } => "todo_deleted",
        };
        let timestamp = event.timestamp();
        let payload = serde_json::to_string(event)?;
        
        conn.execute(
            "INSERT INTO events (event_id, event_type, timestamp, payload) VALUES (?1, ?2, ?3, ?4)",
            params![event_id.to_string(), event_type, timestamp.to_rfc3339(), payload],
        )?;
        
        // Update current state materialized view
        match event {
            TodoEvent::TodoAdded(item) => {
                let stakeholders = serde_json::to_string(&item.stakeholders)?;
                let tags = serde_json::to_string(&item.tags)?;
                conn.execute(
                    "INSERT OR REPLACE INTO current_state 
                    (todo_id, title, description, priority, due_date, completed, created_at, updated_at, source_url, tags, stakeholders)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        item.id.to_string(),
                        &item.title,
                        &item.description,
                        serde_json::to_string(&item.priority)?,
                        item.due_date.map(|d| d.to_rfc3339()),
                        if item.completed { 1 } else { 0 },
                        item.created_at.to_rfc3339(),
                        item.updated_at.to_rfc3339(),
                        &item.source_url,
                        tags,
                        stakeholders,
                    ],
                )?;
            }
            TodoEvent::TodoCompleted { id, completed_at } => {
                conn.execute(
                    "UPDATE current_state SET completed = 1, updated_at = ?1 WHERE todo_id = ?2",
                    params![completed_at.to_rfc3339(), id.to_string()],
                )?;
            }
            TodoEvent::TodoDeleted { id, .. } => {
                conn.execute(
                    "DELETE FROM current_state WHERE todo_id = ?1",
                    params![id.to_string()],
                )?;
            }
            TodoEvent::TodoUpdated { id, updates } => {
                // For simplicity, we'll just reload from events for updates
                // In production, you'd apply specific field updates
            }
        }
        
        Ok(())
    }
    
    pub fn load_all_events(&self) -> Result<Vec<TodoEvent>> {
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare("SELECT payload FROM events ORDER BY timestamp")?;
        let events_iter = stmt.query_map([], |row| {
            let payload: String = row.get(0)?;
            Ok(serde_json::from_str::<TodoEvent>(&payload).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?)
        })?;
        
        let mut events = Vec::new();
        for event_result in events_iter {
            events.push(event_result?);
        }
        Ok(events)
    }
    
    pub fn rebuild_state(&self) -> Result<Vec<TodoItem>> {
        // Load from current_state materialized view for efficiency
        let conn = self.get_connection()?;
        let mut stmt = conn.prepare(
            "SELECT todo_id, title, description, priority, due_date, completed, created_at, updated_at, source_url, tags, stakeholders
             FROM current_state ORDER BY created_at DESC"
        )?;
        
        let items_iter = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let id = Uuid::parse_str(&id_str).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let title: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let priority_str: String = row.get(3)?;
            let priority: Priority = serde_json::from_str(&priority_str).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let due_date_str: Option<String> = row.get(4)?;
            let due_date = due_date_str.map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
                .transpose()
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let completed: i64 = row.get(5)?;
            let created_at_str: String = row.get(6)?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let updated_at_str: String = row.get(7)?;
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let source_url: Option<String> = row.get(8)?;
            let tags_str: String = row.get(9)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let stakeholders_str: String = row.get(10)?;
            let stakeholders: Vec<Stakeholder> = serde_json::from_str(&stakeholders_str).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            
            Ok(TodoItem {
                id,
                title,
                description,
                priority,
                due_date,
                stakeholders,
                completed: completed == 1,
                created_at,
                updated_at,
                source_url,
                tags,
            })
        })?;
        
        let mut items = Vec::new();
        for item_result in items_iter {
            items.push(item_result?);
        }
        Ok(items)
    }
    
    pub fn get_today_tasks(&self) -> Result<Vec<TodoItem>> {
        let conn = self.get_connection()?;
        let today = Utc::now().date_naive().to_string();
        let mut stmt = conn.prepare(
            "SELECT todo_id, title, description, priority, due_date, completed, created_at, updated_at, source_url, tags, stakeholders
             FROM current_state 
             WHERE date(due_date) = date(?1) AND completed = 0
             ORDER BY 
                 CASE priority 
                     WHEN '\"Critical\"' THEN 1
                     WHEN '\"High\"' THEN 2
                     WHEN '\"Medium\"' THEN 3
                     WHEN '\"Low\"' THEN 4
                     ELSE 5
                 END,
                 due_date"
        )?;
        
        let items_iter = stmt.query_map(params![today], |row| {
            let id_str: String = row.get(0)?;
            let id = Uuid::parse_str(&id_str).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let title: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let priority_str: String = row.get(3)?;
            let priority: Priority = serde_json::from_str(&priority_str).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let due_date_str: Option<String> = row.get(4)?;
            let due_date = due_date_str.map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
                .transpose()
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let completed: i64 = row.get(5)?;
            let created_at_str: String = row.get(6)?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let updated_at_str: String = row.get(7)?;
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let source_url: Option<String> = row.get(8)?;
            let tags_str: String = row.get(9)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let stakeholders_str: String = row.get(10)?;
            let stakeholders: Vec<Stakeholder> = serde_json::from_str(&stakeholders_str).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            
            Ok(TodoItem {
                id,
                title,
                description,
                priority,
                due_date,
                stakeholders,
                completed: completed == 1,
                created_at,
                updated_at,
                source_url,
                tags,
            })
        })?;
        
        let mut items = Vec::new();
        for item_result in items_iter {
            items.push(item_result?);
        }
        Ok(items)
    }
}