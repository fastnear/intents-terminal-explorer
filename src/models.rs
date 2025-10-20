use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub color: Color,
    pub created_at: DateTime<Utc>,
    pub todo_count: usize,
    pub completed_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Todo {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub description: String,
    pub status: TodoStatus,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub parent_id: Option<Uuid>, // For subtasks
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Archived,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Color {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Pink,
    Gray,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortMode {
    Priority,
    DueDate,
    Created,
    Alphabetical,
    Status,
}

impl Project {
    pub fn new(name: String, color: Color) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            color,
            created_at: Utc::now(),
            todo_count: 0,
            completed_count: 0,
        }
    }

    pub fn progress_percentage(&self) -> f32 {
        if self.todo_count == 0 {
            0.0
        } else {
            (self.completed_count as f32 / self.todo_count as f32) * 100.0
        }
    }
}

impl Todo {
    pub fn new(project_id: Uuid, title: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            title,
            description: String::new(),
            status: TodoStatus::Pending,
            priority: Priority::Medium,
            tags: Vec::new(),
            due_date: None,
            created_at: Utc::now(),
            completed_at: None,
            parent_id: None,
        }
    }

    pub fn toggle_complete(&mut self) {
        match self.status {
            TodoStatus::Completed => {
                self.status = TodoStatus::Pending;
                self.completed_at = None;
            }
            _ => {
                self.status = TodoStatus::Completed;
                self.completed_at = Some(Utc::now());
            }
        }
    }

    pub fn is_overdue(&self) -> bool {
        if let Some(due) = self.due_date {
            due < Utc::now() && self.status != TodoStatus::Completed
        } else {
            false
        }
    }

    pub fn status_icon(&self) -> &'static str {
        match self.status {
            TodoStatus::Pending => "[ ]",
            TodoStatus::InProgress => "[~]",
            TodoStatus::Completed => "[✓]",
            TodoStatus::Archived => "[×]",
        }
    }

    pub fn priority_icon(&self) -> &'static str {
        match self.priority {
            Priority::Critical => "🔴",
            Priority::High => "🟠",
            Priority::Medium => "🟡",
            Priority::Low => "🟢",
        }
    }
}

impl fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TodoStatus::Pending => write!(f, "Pending"),
            TodoStatus::InProgress => write!(f, "In Progress"),
            TodoStatus::Completed => write!(f, "Completed"),
            TodoStatus::Archived => write!(f, "Archived"),
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Low => write!(f, "Low"),
            Priority::Medium => write!(f, "Medium"),
            Priority::High => write!(f, "High"),
            Priority::Critical => write!(f, "Critical"),
        }
    }
}

impl Color {
    pub fn to_ratatui(&self) -> ratatui::style::Color {
        use ratatui::style::Color as RColor;
        match self {
            Color::Red => RColor::Red,
            Color::Orange => RColor::Rgb(255, 165, 0),
            Color::Yellow => RColor::Yellow,
            Color::Green => RColor::Green,
            Color::Blue => RColor::Blue,
            Color::Purple => RColor::Magenta,
            Color::Pink => RColor::Rgb(255, 192, 203),
            Color::Gray => RColor::Gray,
        }
    }
}

impl SortMode {
    pub fn next(&self) -> Self {
        match self {
            SortMode::Priority => SortMode::DueDate,
            SortMode::DueDate => SortMode::Created,
            SortMode::Created => SortMode::Alphabetical,
            SortMode::Alphabetical => SortMode::Status,
            SortMode::Status => SortMode::Priority,
        }
    }
}

impl fmt::Display for SortMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SortMode::Priority => write!(f, "Priority"),
            SortMode::DueDate => write!(f, "Due Date"),
            SortMode::Created => write!(f, "Created"),
            SortMode::Alphabetical => write!(f, "A-Z"),
            SortMode::Status => write!(f, "Status"),
        }
    }
}