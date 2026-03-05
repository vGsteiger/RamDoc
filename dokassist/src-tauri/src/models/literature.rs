//! Literature model for managing general reference documents (compendium)
//! These documents are not tied to specific patients and are used for
//! retrieval-augmented generation (RAG) in chat and report generation.

use crate::error::AppError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Literature {
    pub id: String,
    pub filename: String,
    pub vault_path: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateLiterature {
    pub filename: String,
    pub vault_path: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLiterature {
    pub description: Option<String>,
}

/// Create a new literature document record
pub fn create_literature(
    conn: &Connection,
    data: CreateLiterature,
) -> Result<Literature, AppError> {
    let id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO literature (id, filename, vault_path, mime_type, size_bytes, description, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        rusqlite::params![
            id,
            data.filename,
            data.vault_path,
            data.mime_type,
            data.size_bytes,
            data.description,
            now,
            now
        ],
    )?;

    get_literature(conn, &id)
}

/// Get a literature document by ID
pub fn get_literature(conn: &Connection, id: &str) -> Result<Literature, AppError> {
    let literature = conn.query_row(
        r#"
        SELECT id, filename, vault_path, mime_type, size_bytes, description, created_at, updated_at
        FROM literature
        WHERE id = ?1
        "#,
        [id],
        |row| {
            Ok(Literature {
                id: row.get(0)?,
                filename: row.get(1)?,
                vault_path: row.get(2)?,
                mime_type: row.get(3)?,
                size_bytes: row.get(4)?,
                description: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )?;

    Ok(literature)
}

/// List all literature documents
pub fn list_literature(
    conn: &Connection,
    limit: u32,
    offset: u32,
) -> Result<Vec<Literature>, AppError> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, filename, vault_path, mime_type, size_bytes, description, created_at, updated_at
        FROM literature
        ORDER BY created_at DESC
        LIMIT ?1 OFFSET ?2
        "#,
    )?;

    let literature = stmt
        .query_map(rusqlite::params![limit, offset], |row| {
            Ok(Literature {
                id: row.get(0)?,
                filename: row.get(1)?,
                vault_path: row.get(2)?,
                mime_type: row.get(3)?,
                size_bytes: row.get(4)?,
                description: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(literature)
}

/// Update a literature document's metadata
pub fn update_literature(
    conn: &Connection,
    id: &str,
    data: UpdateLiterature,
) -> Result<Literature, AppError> {
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        r#"
        UPDATE literature
        SET description = COALESCE(?1, description),
            updated_at = ?2
        WHERE id = ?3
        "#,
        rusqlite::params![data.description, now, id],
    )?;

    get_literature(conn, id)
}

/// Delete a literature document (also cascades to chunks and embeddings)
pub fn delete_literature(conn: &Connection, id: &str) -> Result<(), AppError> {
    let affected = conn.execute("DELETE FROM literature WHERE id = ?1", [id])?;

    if affected == 0 {
        return Err(AppError::NotFound(format!(
            "Literature document not found: {}",
            id
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    fn setup_test_db() -> (tempfile::TempDir, crate::database::DbPool) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let key = [42u8; 32];
        let pool = database::init_db(&db_path, &key).unwrap();
        (temp_dir, pool)
    }

    #[test]
    fn test_create_and_get_literature() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        let create = CreateLiterature {
            filename: "medication_guidelines.pdf".to_string(),
            vault_path: "vault/lit-123.enc".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 1024000,
            description: Some("Swiss medication guidelines".to_string()),
        };

        let literature = create_literature(&conn, create).unwrap();
        assert_eq!(literature.filename, "medication_guidelines.pdf");
        assert_eq!(literature.mime_type, "application/pdf");
        assert_eq!(literature.size_bytes, 1024000);
        assert_eq!(
            literature.description,
            Some("Swiss medication guidelines".to_string())
        );

        let fetched = get_literature(&conn, &literature.id).unwrap();
        assert_eq!(fetched.id, literature.id);
        assert_eq!(fetched.filename, literature.filename);
    }

    #[test]
    fn test_list_literature() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        // Create multiple literature documents
        for i in 0..3 {
            let create = CreateLiterature {
                filename: format!("doc_{}.pdf", i),
                vault_path: format!("vault/lit-{}.enc", i),
                mime_type: "application/pdf".to_string(),
                size_bytes: 1000 * (i as i64 + 1),
                description: Some(format!("Document {}", i)),
            };
            create_literature(&conn, create).unwrap();
        }

        let all_lit = list_literature(&conn, 10, 0).unwrap();
        assert_eq!(all_lit.len(), 3);

        // Test pagination
        let first = list_literature(&conn, 1, 0).unwrap();
        assert_eq!(first.len(), 1);

        let second = list_literature(&conn, 1, 1).unwrap();
        assert_eq!(second.len(), 1);
        assert_ne!(first[0].id, second[0].id);
    }

    #[test]
    fn test_update_literature() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        let create = CreateLiterature {
            filename: "guidelines.pdf".to_string(),
            vault_path: "vault/lit-update.enc".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 500000,
            description: Some("Old description".to_string()),
        };

        let literature = create_literature(&conn, create).unwrap();
        let original_created_at = literature.created_at.clone();

        let update = UpdateLiterature {
            description: Some("New description".to_string()),
        };

        let updated = update_literature(&conn, &literature.id, update).unwrap();
        assert_eq!(updated.description, Some("New description".to_string()));
        assert_eq!(updated.created_at, original_created_at);
        assert_ne!(updated.updated_at, updated.created_at);
    }

    #[test]
    fn test_delete_literature() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        let create = CreateLiterature {
            filename: "to_delete.pdf".to_string(),
            vault_path: "vault/lit-delete.enc".to_string(),
            mime_type: "application/pdf".to_string(),
            size_bytes: 100000,
            description: None,
        };

        let literature = create_literature(&conn, create).unwrap();
        let id = literature.id.clone();

        delete_literature(&conn, &id).unwrap();

        let result = get_literature(&conn, &id);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_nonexistent() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        let result = delete_literature(&conn, "nonexistent-id");
        assert!(result.is_err());
    }
}
