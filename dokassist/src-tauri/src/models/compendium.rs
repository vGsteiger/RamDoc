use crate::error::AppError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompendiumEntry {
    pub id: String,
    pub file_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub id: String,
    pub file_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub token_count: Option<i32>,
    pub created_at: String,
}

/// Create a compendium entry
pub fn create_compendium_entry(
    conn: &Connection,
    file_id: &str,
    title: &str,
    description: Option<&str>,
    category: Option<&str>,
    priority: i32,
) -> Result<CompendiumEntry, AppError> {
    let id = uuid::Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO compendium_entries (id, file_id, title, description, category, priority)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, file_id, title, description, category, priority],
    )?;

    get_compendium_entry(conn, &id)
}

/// Get a compendium entry by ID
pub fn get_compendium_entry(conn: &Connection, id: &str) -> Result<CompendiumEntry, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, file_id, title, description, category, priority, created_at, updated_at
         FROM compendium_entries WHERE id = ?1",
    )?;

    let entry = stmt
        .query_row([id], |row| {
            Ok(CompendiumEntry {
                id: row.get(0)?,
                file_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                category: row.get(4)?,
                priority: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Compendium entry with id {} not found", id))
            }
            _ => e.into(),
        })?;

    Ok(entry)
}

/// List all compendium entries
pub fn list_compendium_entries(conn: &Connection) -> Result<Vec<CompendiumEntry>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, file_id, title, description, category, priority, created_at, updated_at
         FROM compendium_entries ORDER BY priority DESC, created_at DESC",
    )?;

    let entries = stmt
        .query_map([], |row| {
            Ok(CompendiumEntry {
                id: row.get(0)?,
                file_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                category: row.get(4)?,
                priority: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(entries)
}

/// Delete a compendium entry
pub fn delete_compendium_entry(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM compendium_entries WHERE id = ?1", [id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "Compendium entry with id {} not found",
            id
        )));
    }

    Ok(())
}

/// Create a document chunk
pub fn create_document_chunk(
    conn: &Connection,
    file_id: &str,
    chunk_index: i32,
    content: &str,
    token_count: Option<i32>,
) -> Result<DocumentChunk, AppError> {
    let id = uuid::Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO document_chunks (id, file_id, chunk_index, content, token_count)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, file_id, chunk_index, content, token_count],
    )?;

    get_document_chunk(conn, &id)
}

/// Get a document chunk by ID
pub fn get_document_chunk(conn: &Connection, id: &str) -> Result<DocumentChunk, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, file_id, chunk_index, content, token_count, created_at
         FROM document_chunks WHERE id = ?1",
    )?;

    let chunk = stmt
        .query_row([id], |row| {
            Ok(DocumentChunk {
                id: row.get(0)?,
                file_id: row.get(1)?,
                chunk_index: row.get(2)?,
                content: row.get(3)?,
                token_count: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Document chunk with id {} not found", id))
            }
            _ => e.into(),
        })?;

    Ok(chunk)
}

/// List all chunks for a file
pub fn list_chunks_for_file(
    conn: &Connection,
    file_id: &str,
) -> Result<Vec<DocumentChunk>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, file_id, chunk_index, content, token_count, created_at
         FROM document_chunks WHERE file_id = ?1 ORDER BY chunk_index ASC",
    )?;

    let chunks = stmt
        .query_map([file_id], |row| {
            Ok(DocumentChunk {
                id: row.get(0)?,
                file_id: row.get(1)?,
                chunk_index: row.get(2)?,
                content: row.get(3)?,
                token_count: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(chunks)
}

/// Delete all chunks for a file
pub fn delete_chunks_for_file(conn: &Connection, file_id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM document_chunks WHERE file_id = ?1", [file_id])?;
    Ok(())
}

/// Search compendium entries using full-text search
/// Returns relevant chunks from compendium documents
pub fn search_compendium(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<(DocumentChunk, CompendiumEntry)>, AppError> {
    // Use FTS5 search to find relevant document chunks
    // Join with compendium_entries to get metadata
    let mut stmt = conn.prepare(
        r#"
        SELECT
            dc.id, dc.file_id, dc.chunk_index, dc.content, dc.token_count, dc.created_at,
            ce.id, ce.file_id, ce.title, ce.description, ce.category, ce.priority,
            ce.created_at, ce.updated_at
        FROM document_chunks dc
        INNER JOIN compendium_entries ce ON dc.file_id = ce.file_id
        INNER JOIN files f ON dc.file_id = f.id
        WHERE f.is_compendium = 1
        AND dc.content LIKE '%' || ?1 || '%'
        ORDER BY ce.priority DESC, dc.chunk_index ASC
        LIMIT ?2
        "#,
    )?;

    let results = stmt
        .query_map([query, &limit.to_string()], |row| {
            let chunk = DocumentChunk {
                id: row.get(0)?,
                file_id: row.get(1)?,
                chunk_index: row.get(2)?,
                content: row.get(3)?,
                token_count: row.get(4)?,
                created_at: row.get(5)?,
            };

            let entry = CompendiumEntry {
                id: row.get(6)?,
                file_id: row.get(7)?,
                title: row.get(8)?,
                description: row.get(9)?,
                category: row.get(10)?,
                priority: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            };

            Ok((chunk, entry))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;
    use tempfile::TempDir;

    fn setup_test_db() -> (TempDir, crate::database::DbPool) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let key = [42u8; 32];
        let pool = database::init_db(&db_path, &key).unwrap();
        (temp_dir, pool)
    }

    #[test]
    fn test_create_and_get_compendium_entry() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        // First create a file record
        let file_id = uuid::Uuid::now_v7().to_string();
        conn.execute(
            "INSERT INTO files (id, patient_id, filename, vault_path, mime_type, size_bytes, is_compendium)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
            rusqlite::params![file_id, "patient-1", "medical_reference.pdf", "/vault/test.enc", "application/pdf", 1000],
        )
        .unwrap();

        let entry = create_compendium_entry(
            &conn,
            &file_id,
            "Medical Reference",
            Some("A comprehensive medical guide"),
            Some("medicine"),
            10,
        )
        .unwrap();

        assert_eq!(entry.title, "Medical Reference");
        assert_eq!(entry.priority, 10);

        let retrieved = get_compendium_entry(&conn, &entry.id).unwrap();
        assert_eq!(retrieved.id, entry.id);
    }

    #[test]
    fn test_list_compendium_entries() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        // Create multiple entries
        for i in 0..3 {
            let file_id = uuid::Uuid::now_v7().to_string();
            conn.execute(
                "INSERT INTO files (id, patient_id, filename, vault_path, mime_type, size_bytes, is_compendium)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
                rusqlite::params![file_id, "patient-1", format!("ref_{}.pdf", i), format!("/vault/test_{}.enc", i), "application/pdf", 1000],
            )
            .unwrap();

            create_compendium_entry(&conn, &file_id, &format!("Reference {}", i), None, None, i)
                .unwrap();
        }

        let entries = list_compendium_entries(&conn).unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_create_and_list_document_chunks() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        let file_id = uuid::Uuid::now_v7().to_string();
        conn.execute(
            "INSERT INTO files (id, patient_id, filename, vault_path, mime_type, size_bytes, is_compendium)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
            rusqlite::params![file_id, "patient-1", "document.pdf", "/vault/doc.enc", "application/pdf", 2000],
        )
        .unwrap();

        // Create multiple chunks
        for i in 0..3 {
            create_document_chunk(&conn, &file_id, i, &format!("Chunk {} content", i), Some(100))
                .unwrap();
        }

        let chunks = list_chunks_for_file(&conn, &file_id).unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[1].chunk_index, 1);
        assert_eq!(chunks[2].chunk_index, 2);
    }
}
