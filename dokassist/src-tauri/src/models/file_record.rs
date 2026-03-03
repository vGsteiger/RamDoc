use crate::error::AppError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: String,
    pub patient_id: String,
    pub filename: String,
    pub vault_path: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub document_type: Option<String>,
    pub extracted_text: Option<String>,
    pub metadata_json: Option<String>,
    pub is_compendium: bool,
    pub created_at: String,
}

/// Create a new file record in the database
pub fn create_file_record(
    conn: &Connection,
    patient_id: &str,
    filename: &str,
    vault_path: &str,
    mime_type: &str,
    size_bytes: i64,
) -> Result<FileRecord, AppError> {
    let id = uuid::Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO files (id, patient_id, filename, vault_path, mime_type, size_bytes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, patient_id, filename, vault_path, mime_type, size_bytes],
    )?;

    get_file_record(conn, &id)
}

/// Get a file record by ID
pub fn get_file_record(conn: &Connection, id: &str) -> Result<FileRecord, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, patient_id, filename, vault_path, mime_type, size_bytes,
                document_type, extracted_text, metadata_json, is_compendium, created_at
         FROM files WHERE id = ?1",
    )?;

    let record = stmt
        .query_row([id], |row| {
            Ok(FileRecord {
                id: row.get(0)?,
                patient_id: row.get(1)?,
                filename: row.get(2)?,
                vault_path: row.get(3)?,
                mime_type: row.get(4)?,
                size_bytes: row.get(5)?,
                document_type: row.get(6)?,
                extracted_text: row.get(7)?,
                metadata_json: row.get(8)?,
                is_compendium: row.get::<_, i32>(9)? != 0,
                created_at: row.get(10)?,
            })
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("File record with id {} not found", id))
            }
            _ => e.into(),
        })?;

    Ok(record)
}

/// Get a file record by vault path
pub fn get_file_record_by_vault_path(
    conn: &Connection,
    vault_path: &str,
) -> Result<FileRecord, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, patient_id, filename, vault_path, mime_type, size_bytes,
                document_type, extracted_text, metadata_json, is_compendium, created_at
         FROM files WHERE vault_path = ?1",
    )?;

    let record = stmt
        .query_row([vault_path], |row| {
            Ok(FileRecord {
                id: row.get(0)?,
                patient_id: row.get(1)?,
                filename: row.get(2)?,
                vault_path: row.get(3)?,
                mime_type: row.get(4)?,
                size_bytes: row.get(5)?,
                document_type: row.get(6)?,
                extracted_text: row.get(7)?,
                metadata_json: row.get(8)?,
                is_compendium: row.get::<_, i32>(9)? != 0,
                created_at: row.get(10)?,
            })
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!(
                "File record with vault path {} not found",
                vault_path
            )),
            _ => e.into(),
        })?;

    Ok(record)
}

/// List all files for a patient
pub fn list_files_for_patient(
    conn: &Connection,
    patient_id: &str,
) -> Result<Vec<FileRecord>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, patient_id, filename, vault_path, mime_type, size_bytes,
                document_type, extracted_text, metadata_json, is_compendium, created_at
         FROM files WHERE patient_id = ?1 ORDER BY created_at DESC",
    )?;

    let records = stmt
        .query_map([patient_id], |row| {
            Ok(FileRecord {
                id: row.get(0)?,
                patient_id: row.get(1)?,
                filename: row.get(2)?,
                vault_path: row.get(3)?,
                mime_type: row.get(4)?,
                size_bytes: row.get(5)?,
                document_type: row.get(6)?,
                extracted_text: row.get(7)?,
                metadata_json: row.get(8)?,
                is_compendium: row.get::<_, i32>(9)? != 0,
                created_at: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(records)
}

/// Delete a file record from the database
pub fn delete_file_record(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM files WHERE id = ?1", [id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "File record with id {} not found",
            id
        )));
    }

    Ok(())
}
