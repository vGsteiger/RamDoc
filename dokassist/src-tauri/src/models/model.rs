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
