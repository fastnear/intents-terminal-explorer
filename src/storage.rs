use crate::models::{Project, Todo, TodoStatus, Priority, Color};
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::Path;
use uuid::Uuid;

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let path = dirs::data_local_dir()
            .unwrap_or_else(|| Path::new(".").to_path_buf())
            .join("ratacat")
            .join("todos.db");

        std::fs::create_dir_all(path.parent().unwrap())?;

        let conn = Connection::open(&path)?;
        let mut storage = Self { conn };
        storage.init_db()?;
        Ok(storage)
    }

    fn init_db(&mut self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS todos (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                priority INTEGER NOT NULL,
                due_date TEXT,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                parent_id TEXT,
                FOREIGN KEY (project_id) REFERENCES projects(id)
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS tags (
                todo_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (todo_id, tag),
                FOREIGN KEY (todo_id) REFERENCES todos(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create default project if none exists
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM projects",
            [],
            |row| row.get(0),
        )?;

        if count == 0 {
            let default_project = Project::new("Inbox".to_string(), Color::Blue);
            self.save_project(&default_project)?;
        }

        Ok(())
    }

    // Project operations
    pub fn save_project(&mut self, project: &Project) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO projects (id, name, color, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                project.id.to_string(),
                project.name,
                serde_json::to_string(&project.color)?,
                project.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn get_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.name, p.color, p.created_at,
                    COUNT(CASE WHEN t.id IS NOT NULL THEN 1 END) as todo_count,
                    COUNT(CASE WHEN t.status = 'Completed' THEN 1 END) as completed_count
             FROM projects p
             LEFT JOIN todos t ON p.id = t.project_id
             GROUP BY p.id, p.name, p.color, p.created_at
             ORDER BY p.created_at"
        )?;

        let projects = stmt.query_map([], |row| {
            Ok(Project {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                name: row.get(1)?,
                color: serde_json::from_str(&row.get::<_, String>(2)?).unwrap(),
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?).unwrap().with_timezone(&Utc),
                todo_count: row.get::<_, i32>(4)? as usize,
                completed_count: row.get::<_, i32>(5)? as usize,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(projects)
    }

    pub fn delete_project(&mut self, id: &Uuid) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM tags WHERE todo_id IN (SELECT id FROM todos WHERE project_id = ?1)",
                   params![id.to_string()])?;
        tx.execute("DELETE FROM todos WHERE project_id = ?1", params![id.to_string()])?;
        tx.execute("DELETE FROM projects WHERE id = ?1", params![id.to_string()])?;
        tx.commit()?;
        Ok(())
    }

    // Todo operations
    pub fn save_todo(&mut self, todo: &Todo) -> Result<()> {
        let tx = self.conn.transaction()?;

        tx.execute(
            "INSERT OR REPLACE INTO todos
             (id, project_id, title, description, status, priority, due_date, created_at, completed_at, parent_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                todo.id.to_string(),
                todo.project_id.to_string(),
                todo.title,
                todo.description,
                format!("{:?}", todo.status),
                todo.priority as i32,
                todo.due_date.map(|d| d.to_rfc3339()),
                todo.created_at.to_rfc3339(),
                todo.completed_at.map(|d| d.to_rfc3339()),
                todo.parent_id.map(|id| id.to_string()),
            ],
        )?;

        // Update tags
        tx.execute("DELETE FROM tags WHERE todo_id = ?1", params![todo.id.to_string()])?;
        for tag in &todo.tags {
            tx.execute(
                "INSERT INTO tags (todo_id, tag) VALUES (?1, ?2)",
                params![todo.id.to_string(), tag],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_todos_by_project(&self, project_id: &Uuid) -> Result<Vec<Todo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, title, description, status, priority,
                    due_date, created_at, completed_at, parent_id
             FROM todos
             WHERE project_id = ?1"
        )?;

        let todos = stmt.query_map(params![project_id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let todo_id = Uuid::parse_str(&id_str).unwrap();

            Ok(Todo {
                id: todo_id,
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                title: row.get(2)?,
                description: row.get(3)?,
                status: match row.get::<_, String>(4)?.as_str() {
                    "Pending" => TodoStatus::Pending,
                    "InProgress" => TodoStatus::InProgress,
                    "Completed" => TodoStatus::Completed,
                    "Archived" => TodoStatus::Archived,
                    _ => TodoStatus::Pending,
                },
                priority: match row.get::<_, i32>(5)? {
                    1 => Priority::Low,
                    2 => Priority::Medium,
                    3 => Priority::High,
                    4 => Priority::Critical,
                    _ => Priority::Medium,
                },
                tags: Vec::new(), // Will be filled below
                due_date: row.get::<_, Option<String>>(6)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|d| d.with_timezone(&Utc)),
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?).unwrap().with_timezone(&Utc),
                completed_at: row.get::<_, Option<String>>(8)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|d| d.with_timezone(&Utc)),
                parent_id: row.get::<_, Option<String>>(9)?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        // Load tags for each todo
        let mut todos_with_tags = Vec::new();
        for mut todo in todos {
            let tags: Vec<String> = self.conn.prepare(
                "SELECT tag FROM tags WHERE todo_id = ?1"
            )?.query_map(params![todo.id.to_string()], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

            todo.tags = tags;
            todos_with_tags.push(todo);
        }

        Ok(todos_with_tags)
    }

    pub fn delete_todo(&mut self, id: &Uuid) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM tags WHERE todo_id = ?1", params![id.to_string()])?;
        tx.execute("DELETE FROM todos WHERE id = ?1", params![id.to_string()])?;
        tx.commit()?;
        Ok(())
    }

    pub fn archive_completed(&mut self, project_id: &Uuid) -> Result<usize> {
        let result = self.conn.execute(
            "UPDATE todos SET status = 'Archived'
             WHERE project_id = ?1 AND status = 'Completed'",
            params![project_id.to_string()],
        )?;
        Ok(result)
    }

    pub fn search_todos(&self, query: &str) -> Result<Vec<Todo>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT t.id, t.project_id, t.title, t.description, t.status,
                    t.priority, t.due_date, t.created_at, t.completed_at, t.parent_id
             FROM todos t
             LEFT JOIN tags tg ON t.id = tg.todo_id
             WHERE t.title LIKE ?1 OR t.description LIKE ?1 OR tg.tag LIKE ?1"
        )?;

        let todos = stmt.query_map(params![pattern], |row| {
            Ok(Todo {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                title: row.get(2)?,
                description: row.get(3)?,
                status: match row.get::<_, String>(4)?.as_str() {
                    "Pending" => TodoStatus::Pending,
                    "InProgress" => TodoStatus::InProgress,
                    "Completed" => TodoStatus::Completed,
                    "Archived" => TodoStatus::Archived,
                    _ => TodoStatus::Pending,
                },
                priority: match row.get::<_, i32>(5)? {
                    1 => Priority::Low,
                    2 => Priority::Medium,
                    3 => Priority::High,
                    4 => Priority::Critical,
                    _ => Priority::Medium,
                },
                tags: Vec::new(),
                due_date: row.get::<_, Option<String>>(6)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|d| d.with_timezone(&Utc)),
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?).unwrap().with_timezone(&Utc),
                completed_at: row.get::<_, Option<String>>(8)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|d| d.with_timezone(&Utc)),
                parent_id: row.get::<_, Option<String>>(9)?
                    .and_then(|s| Uuid::parse_str(&s).ok()),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(todos)
    }
}