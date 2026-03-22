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
    pub size_bytes: u64,
    pub created_at: String,
}

/// Create a new file record in the database
pub fn create_file_record(
    conn: &Connection,
    patient_id: &str,
    filename: &str,
    vault_path: &str,
    mime_type: &str,
    size_bytes: u64,
) -> Result<FileRecord, AppError> {
    let id = uuid::Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO files (id, patient_id, filename, vault_path, mime_type, size_bytes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            id,
            patient_id,
            filename,
            vault_path,
            mime_type,
            size_bytes as i64
        ],
    )?;

    get_file_record(conn, &id)
}

/// Get a file record by ID
pub fn get_file_record(conn: &Connection, id: &str) -> Result<FileRecord, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, patient_id, filename, vault_path, mime_type, size_bytes, created_at
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
                size_bytes: row.get::<_, i64>(5)? as u64,
                created_at: row.get(6)?,
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
        "SELECT id, patient_id, filename, vault_path, mime_type, size_bytes, created_at
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
                size_bytes: row.get::<_, i64>(5)? as u64,
                created_at: row.get(6)?,
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
        "SELECT id, patient_id, filename, vault_path, mime_type, size_bytes, created_at
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
                size_bytes: row.get::<_, i64>(5)? as u64,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(records)
}

/// Persist extracted text and optional document type for an existing file record.
/// Called by `process_file` after LLM/PDF extraction completes.
pub fn update_file_text(
    conn: &Connection,
    file_id: &str,
    extracted_text: &str,
    document_type: Option<&str>,
) -> Result<(), AppError> {
    let rows = conn.execute(
        "UPDATE files SET extracted_text = ?1, document_type = ?2 WHERE id = ?3",
        rusqlite::params![extracted_text, document_type, file_id],
    )?;

    if rows == 0 {
        return Err(AppError::NotFound(format!(
            "File record with id {} not found",
            file_id
        )));
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_db;
    use tempfile::tempdir;

    fn open_test_db() -> (tempfile::TempDir, crate::database::DbPool) {
        let dir = tempdir().unwrap();
        let key = crate::crypto::generate_key();
        let pool = init_db(&dir.path().join("test.db"), &key).unwrap();
        (dir, pool)
    }

    fn insert_patient(conn: &Connection) {
        conn.execute(
            "INSERT INTO patients (id, first_name, last_name, date_of_birth, ahv_number)
             VALUES ('p1', 'Anna', 'Test', '1985-01-01', '756.1234.5678.97')",
            [],
        )
        .unwrap();
    }

    fn make_file(conn: &Connection) -> FileRecord {
        create_file_record(
            conn,
            "p1",
            "report.pdf",
            "vault/abc.enc",
            "application/pdf",
            12345,
        )
        .unwrap()
    }

    #[test]
    fn test_create_and_get_file_record() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let f = make_file(&conn);
        assert_eq!(f.filename, "report.pdf");
        assert_eq!(f.mime_type, "application/pdf");
        assert_eq!(f.size_bytes, 12345u64);
        let f2 = get_file_record(&conn, &f.id).unwrap();
        assert_eq!(f.id, f2.id);
        assert_eq!(f2.size_bytes, 12345u64);
    }

    #[test]
    fn test_get_file_record_by_vault_path() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let f = make_file(&conn);
        let f2 = get_file_record_by_vault_path(&conn, "vault/abc.enc").unwrap();
        assert_eq!(f.id, f2.id);
        assert_eq!(f2.filename, "report.pdf");
    }

    #[test]
    fn test_list_files_for_patient() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        for i in 0..3u64 {
            create_file_record(
                &conn,
                "p1",
                &format!("file{}.pdf", i),
                &format!("vault/f{}.enc", i),
                "application/pdf",
                i * 100,
            )
            .unwrap();
        }
        let list = list_files_for_patient(&conn, "p1").unwrap();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_update_file_text() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let f = make_file(&conn);
        update_file_text(&conn, &f.id, "extracted content", Some("referral")).unwrap();
        let text: (String, String) = conn
            .query_row(
                "SELECT extracted_text, document_type FROM files WHERE id = ?1",
                [&f.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(text.0, "extracted content");
        assert_eq!(text.1, "referral");
    }

    #[test]
    fn test_update_file_text_unknown_id() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        assert!(matches!(
            update_file_text(&conn, "no-such-id", "text", None),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn test_delete_file_record() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let f = make_file(&conn);
        delete_file_record(&conn, &f.id).unwrap();
        assert!(matches!(
            get_file_record(&conn, &f.id),
            Err(AppError::NotFound(_))
        ));
    }
}
