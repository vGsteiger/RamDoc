use crate::error::AppError;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Database connection pool wrapper
#[derive(Clone)]
pub struct DbPool {
    conn: Arc<Mutex<Connection>>,
}

impl DbPool {
    /// Get a connection from the pool
    pub fn conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AppError> {
        self.conn.lock().map_err(|_| {
            AppError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some("Database connection pool poisoned".to_string()),
            ))
        })
    }
}

/// Initialize the database with SQLCipher encryption
/// Returns a connection pool handle
pub fn init_db(db_path: &Path, key: &[u8; 32]) -> Result<DbPool, AppError> {
    // Open database with SQLCipher support
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;

    // Set the encryption key (SQLCipher uses raw key mode)
    // The key must be set before any other operations.
    //
    // MED-2: hex::encode() always produces exactly 64 lowercase hex characters
    // for a 32-byte key, so the format!() below is safe against injection.
    // This assertion guards against future refactors that might change the key source.
    let mut key_hex = hex::encode(key);
    debug_assert!(
        key_hex.len() == 64 && key_hex.chars().all(|c| c.is_ascii_hexdigit()),
        "SQLCipher key must be exactly 64 lowercase hex characters"
    );
    conn.execute_batch(&format!("PRAGMA key = \"x'{}'\";", key_hex))?;

    // Zeroize the key hex string
    use zeroize::Zeroize;
    key_hex.zeroize();

    // Verify the key is correct by attempting a simple operation
    // This will fail if the key is wrong or the database is corrupted
    conn.query_row("SELECT count(*) FROM sqlite_master;", [], |_| Ok(()))?;

    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON;", [])?;

    // Run migrations
    run_migrations(&conn)?;

    Ok(DbPool {
        conn: Arc::new(Mutex::new(conn)),
    })
}

/// Run database migrations
fn run_migrations(conn: &Connection) -> Result<(), AppError> {
    // Check current schema version
    let version: i32 = conn.query_row("PRAGMA user_version;", [], |row| row.get(0))?;

    log::info!("Current database schema version: {}", version);

    // Migration 1: Initial schema
    if version < 1 {
        log::info!("Running migration 001: Initial schema");
        conn.execute_batch(include_str!("migrations/001_initial.sql"))?;
        conn.execute("PRAGMA user_version = 1;", [])?;
    }

    // Migration 2: Append-only audit log triggers (CRIT-5)
    if version < 2 {
        log::info!("Running migration 002: Append-only audit log");
        conn.execute_batch(include_str!("migrations/002_audit_append_only.sql"))?;
        conn.execute("PRAGMA user_version = 2;", [])?;
    }

    // Migration 3: document_embeddings table for semantic search
    if version < 3 {
        log::info!("Running migration 003: Document embeddings");
        conn.execute_batch(include_str!("migrations/003_embeddings.sql"))?;
        conn.execute("PRAGMA user_version = 3;", [])?;
    }

    // Migration 4: Chat sessions and messages tables
    if version < 4 {
        log::info!("Running migration 004: Chat sessions");
        conn.execute_batch(include_str!("migrations/004_chat.sql"))?;
        conn.execute("PRAGMA user_version = 4;", [])?;
    }

    // Migration 5: Literature management and document chunks for RAG
    if version < 5 {
        log::info!("Running migration 005: Literature and document chunks");
        conn.execute_batch(include_str!("migrations/005_literature.sql"))?;
        conn.execute("PRAGMA user_version = 5;", [])?;
    }

    // Migration 6: Email drafts table
    if version < 6 {
        log::info!("Running migration 006: Email drafts");
        conn.execute_batch(include_str!("migrations/006_emails.sql"))?;
        conn.execute("PRAGMA user_version = 6;", [])?;
    }

    // Migration 7: Treatment plans, goals, and interventions
    if version < 7 {
        log::info!("Running migration 007: Treatment plans");
        conn.execute_batch(include_str!("migrations/007_treatment_plans.sql"))?;
        conn.execute("PRAGMA user_version = 7;", [])?;
    }

    // Migration 8: Outcome scores table for standardized questionnaires
    if version < 8 {
        log::info!("Running migration 008: Outcome scores");
        conn.execute_batch(include_str!("migrations/008_outcome_scores.sql"))?;
        conn.execute("PRAGMA user_version = 8;", [])?;
    }

    // Migration 9: Add scheduled_time to sessions table for calendar view
    if version < 9 {
        log::info!("Running migration 009: Sessions scheduled_time");
        conn.execute_batch(include_str!("migrations/009_sessions_scheduled_time.sql"))?;
        conn.execute("PRAGMA user_version = 9;", [])?;
    }

    // Migration 10: Clinical summary column on sessions
    if version < 10 {
        log::info!("Running migration 010: Clinical summary");
        conn.execute_batch(include_str!("migrations/010_clinical_summary.sql"))?;
        conn.execute("PRAGMA user_version = 10;", [])?;
    }

    log::info!("Database migrations complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_db_init() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let key = crate::crypto::generate_key();

        let pool = init_db(&db_path, &key).unwrap();
        let conn = pool.conn().unwrap();

        // Verify foreign keys are enabled
        let fk_enabled: i32 = conn
            .query_row("PRAGMA foreign_keys;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk_enabled, 1);
    }

    #[test]
    fn test_db_wrong_key() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let key1 = crate::crypto::generate_key();
        let key2 = crate::crypto::generate_key();

        // Create database with key1
        init_db(&db_path, &key1).unwrap();

        // Try to open with wrong key
        let result = init_db(&db_path, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_db_reopen_with_correct_key() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let key = crate::crypto::generate_key();

        // Create database
        let pool1 = init_db(&db_path, &key).unwrap();
        drop(pool1);

        // Reopen with same key
        let pool2 = init_db(&db_path, &key).unwrap();
        let conn = pool2.conn().unwrap();

        // Verify we can query and that all migrations have run
        let version: i32 = conn
            .query_row("PRAGMA user_version;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 10);
    }
}
