pub mod kanban;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use kanban::KanbanState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Blocked,
}

impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::Todo
    }
}

impl TaskStatus {
    pub fn label(&self) -> &str {
        match self {
            TaskStatus::Todo => "Todo",
            TaskStatus::InProgress => "In Progress",
            TaskStatus::Done => "Done",
            TaskStatus::Blocked => "Blocked",
        }
    }

    pub fn all() -> &'static [TaskStatus] {
        &[
            TaskStatus::Todo,
            TaskStatus::InProgress,
            TaskStatus::Done,
            TaskStatus::Blocked,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    #[serde(default = "new_uuid")]
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(default)]
    pub status: TaskStatus,
    pub priority: Option<u8>,
    pub due: Option<String>,
    pub tags: Option<Vec<String>>,
    #[serde(skip)]
    pub note_path: String,
}

fn new_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}
