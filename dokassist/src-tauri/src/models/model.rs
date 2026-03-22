use crate::error::AppError;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub downloaded_at: String,
    pub last_used: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskModel {
    pub task_type: String,
    pub model_id: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Task types that can have specific models assigned
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    Summary,
    Letter,
    Report,
    Default,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::Summary => "summary",
            TaskType::Letter => "letter",
            TaskType::Report => "report",
            TaskType::Default => "default",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, AppError> {
        match s {
            "summary" => Ok(TaskType::Summary),
            "letter" => Ok(TaskType::Letter),
            "report" => Ok(TaskType::Report),
            "default" => Ok(TaskType::Default),
            _ => Err(AppError::Validation(format!("Unknown task type: {}", s))),
        }
    }
}

impl Model {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            filename: row.get(2)?,
            sha256: row.get(3)?,
            size_bytes: row.get(4)?,
            downloaded_at: row.get(5)?,
            last_used: row.get(6)?,
            is_default: row.get::<_, i32>(7)? != 0,
        })
    }
}

impl TaskModel {
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            task_type: row.get(0)?,
            model_id: row.get(1)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
        })
    }
}

/// Create a new model record after successful download
pub fn create_model(
    conn: &Connection,
    id: &str,
    name: &str,
    filename: &str,
    sha256: &str,
    size_bytes: i64,
) -> Result<Model, AppError> {
    conn.execute(
        "INSERT INTO models (id, name, filename, sha256, size_bytes) VALUES (?, ?, ?, ?, ?)",
        params![id, name, filename, sha256, size_bytes],
    )?;

    get_model(conn, id)
}

/// Get a model by ID
pub fn get_model(conn: &Connection, id: &str) -> Result<Model, AppError> {
    conn.query_row(
        "SELECT id, name, filename, sha256, size_bytes, downloaded_at, last_used, is_default
         FROM models WHERE id = ?",
        params![id],
        Model::from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Model not found: {}", id))
        }
        _ => AppError::Database(e),
    })
}

/// Get a model by filename
pub fn get_model_by_filename(conn: &Connection, filename: &str) -> Result<Model, AppError> {
    conn.query_row(
        "SELECT id, name, filename, sha256, size_bytes, downloaded_at, last_used, is_default
         FROM models WHERE filename = ?",
        params![filename],
        Model::from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            AppError::NotFound(format!("Model not found: {}", filename))
        }
        _ => AppError::Database(e),
    })
}

/// List all models
pub fn list_models(conn: &Connection) -> Result<Vec<Model>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, filename, sha256, size_bytes, downloaded_at, last_used, is_default
         FROM models ORDER BY last_used DESC, downloaded_at DESC",
    )?;

    let models = stmt
        .query_map([], Model::from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(models)
}

/// Delete a model by ID
pub fn delete_model(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM models WHERE id = ?", params![id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("Model not found: {}", id)));
    }

    Ok(())
}

/// Set a model as the default
pub fn set_default_model(conn: &Connection, id: &str) -> Result<(), AppError> {
    // Verify the model exists
    get_model(conn, id)?;

    // Clear existing default
    conn.execute("UPDATE models SET is_default = 0", [])?;

    // Set new default
    conn.execute("UPDATE models SET is_default = 1 WHERE id = ?", params![id])?;

    Ok(())
}

/// Get the default model
pub fn get_default_model(conn: &Connection) -> Result<Option<Model>, AppError> {
    match conn.query_row(
        "SELECT id, name, filename, sha256, size_bytes, downloaded_at, last_used, is_default
         FROM models WHERE is_default = 1",
        [],
        Model::from_row,
    ) {
        Ok(model) => Ok(Some(model)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Database(e)),
    }
}

/// Update last_used timestamp for a model
pub fn update_model_last_used(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE models SET last_used = datetime('now') WHERE id = ?",
        params![id],
    )?;
    Ok(())
}

/// Set the model for a specific task type
pub fn set_task_model(
    conn: &Connection,
    task_type: TaskType,
    model_id: &str,
) -> Result<(), AppError> {
    // Verify the model exists
    get_model(conn, model_id)?;

    let task_str = task_type.as_str();

    // Upsert the task model assignment
    conn.execute(
        "INSERT INTO task_models (task_type, model_id) VALUES (?, ?)
         ON CONFLICT(task_type) DO UPDATE SET model_id = ?, updated_at = datetime('now')",
        params![task_str, model_id, model_id],
    )?;

    Ok(())
}

/// Get the model assigned to a specific task type
pub fn get_task_model(
    conn: &Connection,
    task_type: TaskType,
) -> Result<Option<Model>, AppError> {
    let task_str = task_type.as_str();

    match conn.query_row(
        "SELECT m.id, m.name, m.filename, m.sha256, m.size_bytes, m.downloaded_at, m.last_used, m.is_default
         FROM models m
         JOIN task_models tm ON m.id = tm.model_id
         WHERE tm.task_type = ?",
        params![task_str],
        Model::from_row,
    ) {
        Ok(model) => Ok(Some(model)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Database(e)),
    }
}

/// Get all task model assignments
pub fn list_task_models(conn: &Connection) -> Result<Vec<TaskModel>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT task_type, model_id, created_at, updated_at
         FROM task_models ORDER BY task_type",
    )?;

    let task_models = stmt
        .query_map([], TaskModel::from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(task_models)
}

/// Clear the model assignment for a specific task type
pub fn clear_task_model(conn: &Connection, task_type: TaskType) -> Result<(), AppError> {
    let task_str = task_type.as_str();
    conn.execute(
        "DELETE FROM task_models WHERE task_type = ?",
        params![task_str],
    )?;
    Ok(())
}
