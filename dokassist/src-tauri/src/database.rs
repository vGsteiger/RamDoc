use crate::audit::{self, ChainCheckpoint, MAC_SIZE};
#[cfg(target_os = "macos")]
use crate::constants::{AUDIT_CHECKPOINT_A_ACCOUNT, AUDIT_CHECKPOINT_B_ACCOUNT, KEYCHAIN_SERVICE};
use crate::error::AppError;
use rusqlite::{Connection, OpenFlags};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use zeroize::Zeroize;

pub const LATEST_SCHEMA_VERSION: i32 = 16;

trait CheckpointStore: Send + Sync {
    fn read(&self) -> Result<Option<ChainCheckpoint>, AppError>;
    fn write(&self, checkpoint: &ChainCheckpoint) -> Result<(), AppError>;
    fn reset(&self) -> Result<(), AppError>;
}

#[cfg(target_os = "macos")]
struct KeychainCheckpointStore;

#[cfg(target_os = "macos")]
impl CheckpointStore for KeychainCheckpointStore {
    fn read(&self) -> Result<Option<ChainCheckpoint>, AppError> {
        let mut checkpoints = Vec::new();
        for account in [AUDIT_CHECKPOINT_A_ACCOUNT, AUDIT_CHECKPOINT_B_ACCOUNT] {
            match crate::keychain::retrieve_key(KEYCHAIN_SERVICE, account) {
                Ok(encoded) => checkpoints.push(ChainCheckpoint::from_bytes(&encoded)?),
                Err(AppError::KeychainItemMissing) => {}
                Err(err) => return Err(err),
            }
        }
        checkpoints.sort_by_key(|checkpoint| checkpoint.sequence);
        if checkpoints.len() == 2
            && checkpoints[0].sequence == checkpoints[1].sequence
            && checkpoints[0].mac != checkpoints[1].mac
        {
            return Err(AppError::AuditIntegrity(
                "Keychain audit checkpoint slots disagree".to_string(),
            ));
        }
        Ok(checkpoints.pop())
    }

    fn write(&self, checkpoint: &ChainCheckpoint) -> Result<(), AppError> {
        let account = if checkpoint.sequence % 2 == 0 {
            AUDIT_CHECKPOINT_A_ACCOUNT
        } else {
            AUDIT_CHECKPOINT_B_ACCOUNT
        };
        crate::keychain::store_key(KEYCHAIN_SERVICE, account, &checkpoint.to_bytes())
    }

    fn reset(&self) -> Result<(), AppError> {
        for account in [AUDIT_CHECKPOINT_A_ACCOUNT, AUDIT_CHECKPOINT_B_ACCOUNT] {
            match crate::keychain::delete_key(KEYCHAIN_SERVICE, account) {
                Ok(()) | Err(AppError::KeychainItemMissing) => {}
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointMode {
    /// Refuse to open when the Keychain checkpoint is missing or does not occur
    /// in the verified chain.
    Strict,
    /// Mnemonic recovery or backup restore: establish a new checkpoint.
    Reanchor,
    /// The user has seen a checkpoint mismatch at unlock and re-authenticated to
    /// accept it: establish a new checkpoint. Behaves like `Strict` when there is
    /// nothing to accept.
    AcknowledgedReanchor,
}

enum CheckpointStatus {
    /// The stored checkpoint (if any) occurs in the verified chain.
    Anchored(Option<ChainCheckpoint>),
    /// The stored checkpoint is missing, conflicting, or not in the chain.
    Discrepancy(String),
}

/// Compare the stored checkpoint against the verified chain.
///
/// Only integrity findings count as a discrepancy. Keychain access failures are
/// propagated so they are never mistaken for tampering and never re-anchor.
fn checkpoint_status(
    conn: &Connection,
    store: &dyn CheckpointStore,
    migrated_audit_chain: bool,
) -> Result<CheckpointStatus, AppError> {
    match store.read() {
        Ok(Some(checkpoint)) => match audit::verify_checkpoint(conn, &checkpoint) {
            Ok(()) => Ok(CheckpointStatus::Anchored(Some(checkpoint))),
            Err(AppError::AuditIntegrity(message)) => Ok(CheckpointStatus::Discrepancy(message)),
            Err(err) => Err(err),
        },
        // The chain was created by this open's migration; there is no prior
        // anchor to compare against.
        Ok(None) if migrated_audit_chain => Ok(CheckpointStatus::Anchored(None)),
        Ok(None) => Ok(CheckpointStatus::Discrepancy(
            "Keychain audit checkpoint is missing".to_string(),
        )),
        Err(AppError::AuditIntegrity(message)) => Ok(CheckpointStatus::Discrepancy(message)),
        Err(err) => Err(err),
    }
}

/// Append a permanent record of the discrepancy, then anchor the new head.
///
/// The entry is written before the Keychain is touched: if the process dies in
/// between, the old checkpoint still mismatches and the next unlock asks again.
fn record_and_reanchor(
    conn: &Connection,
    audit_key: &[u8; MAC_SIZE],
    store: &dyn CheckpointStore,
    head: &ChainCheckpoint,
    discrepancy: Option<&str>,
    authorization: &str,
) -> Result<ChainCheckpoint, AppError> {
    let head = match discrepancy {
        Some(discrepancy) => {
            let details = format!(
                "{discrepancy}; database audit head was sequence {}; new checkpoint established by {authorization}",
                head.sequence
            );
            log::warn!("Re-anchoring audit checkpoint: {details}");
            audit::log(
                conn,
                audit::AuditAction::CheckpointReanchored,
                "audit_log",
                None,
                Some(&details),
            )?;
            audit::verify_chain(conn, audit_key)?
        }
        None => head.clone(),
    };
    store.reset()?;
    store.write(&head)?;
    Ok(head)
}

struct DbInner {
    conn: Mutex<Connection>,
    audit_key: [u8; MAC_SIZE],
    checkpoint_store: Option<Arc<dyn CheckpointStore>>,
    checkpoint: Mutex<ChainCheckpoint>,
    checkpoint_error: Mutex<Option<String>>,
}

impl DbInner {
    fn sync_checkpoint(&self, conn: &Connection) -> Result<(), AppError> {
        let Some(store) = &self.checkpoint_store else {
            return Ok(());
        };
        let current = self
            .checkpoint
            .lock()
            .map_err(|_| AppError::AuditIntegrity("checkpoint mutex poisoned".to_string()))?
            .clone();
        let head = audit::verify_chain_from_checkpoint(conn, &self.audit_key, &current)?;
        if head != current {
            store.write(&head)?;
            *self
                .checkpoint
                .lock()
                .map_err(|_| AppError::AuditIntegrity("checkpoint mutex poisoned".to_string()))? =
                head;
        }
        Ok(())
    }
}

impl Drop for DbInner {
    fn drop(&mut self) {
        self.audit_key.zeroize();
    }
}

/// Database connection pool wrapper
#[derive(Clone)]
pub struct DbPool {
    inner: Arc<DbInner>,
}

impl DbPool {
    /// Get a connection from the pool
    pub fn conn(&self) -> Result<DbConnection<'_>, AppError> {
        if let Some(message) = self
            .inner
            .checkpoint_error
            .lock()
            .map_err(|_| AppError::AuditIntegrity("checkpoint error mutex poisoned".to_string()))?
            .clone()
        {
            return Err(AppError::AuditIntegrity(message));
        }
        let guard = self.inner.conn.lock().map_err(|_| {
            AppError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some("Database connection pool poisoned".to_string()),
            ))
        })?;
        Ok(DbConnection {
            guard,
            inner: &self.inner,
        })
    }
}

pub struct DbConnection<'a> {
    guard: MutexGuard<'a, Connection>,
    inner: &'a DbInner,
}

impl Deref for DbConnection<'_> {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl DerefMut for DbConnection<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl Drop for DbConnection<'_> {
    fn drop(&mut self) {
        if self.guard.is_autocommit() {
            if let Err(err) = self.inner.sync_checkpoint(&self.guard) {
                log::error!("Failed to advance audit Keychain checkpoint: {err}");
                if let Ok(mut slot) = self.inner.checkpoint_error.lock() {
                    *slot = Some(err.to_string());
                }
            }
        }
    }
}

/// Initialize the database with SQLCipher encryption
/// Returns a connection pool handle
pub fn init_db(db_path: &Path, key: &[u8; 32]) -> Result<DbPool, AppError> {
    let audit_key = audit::derive_mac_key(key, key);
    init_db_internal(db_path, key, audit_key, None, CheckpointMode::Strict)
}

#[cfg(target_os = "macos")]
pub fn init_db_with_audit_checkpoint(
    db_path: &Path,
    db_key: &[u8; 32],
    fs_key: &[u8; 32],
    mode: CheckpointMode,
) -> Result<DbPool, AppError> {
    init_db_internal(
        db_path,
        db_key,
        audit::derive_mac_key(db_key, fs_key),
        Some(Arc::new(KeychainCheckpointStore)),
        mode,
    )
}

/// Set the SQLCipher raw encryption key on a connection.
///
/// Must be called before any other operation on the connection.
///
/// MED-2: `hex::encode()` always produces exactly 64 lowercase hex characters
/// for a 32-byte key, so the `format!()` below is safe against injection. The
/// check is enforced in release builds too, so a future refactor that changes
/// the key source cannot silently pass an unescaped value into the pragma.
pub(crate) fn apply_sqlcipher_key(conn: &Connection, key: &[u8; 32]) -> Result<(), AppError> {
    let mut key_hex = hex::encode(key);
    if key_hex.len() != 64 || !key_hex.chars().all(|c| c.is_ascii_hexdigit()) {
        key_hex.zeroize();
        return Err(AppError::Crypto(
            "SQLCipher key must be exactly 64 hex characters".to_string(),
        ));
    }

    let result = conn.execute_batch(&format!("PRAGMA key = \"x'{}'\";", key_hex));

    // Zeroize the key hex string
    key_hex.zeroize();

    result.map_err(AppError::from)
}

fn init_db_internal(
    db_path: &Path,
    key: &[u8; 32],
    audit_key: [u8; MAC_SIZE],
    checkpoint_store: Option<Arc<dyn CheckpointStore>>,
    checkpoint_mode: CheckpointMode,
) -> Result<DbPool, AppError> {
    // Open database with SQLCipher support
    let conn = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;

    // Set the encryption key (SQLCipher uses raw key mode)
    apply_sqlcipher_key(&conn, key)?;

    // Verify the key is correct by attempting a simple operation
    // This will fail if the key is wrong or the database is corrupted
    conn.query_row("SELECT count(*) FROM sqlite_master;", [], |_| Ok(()))?;

    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON;", [])?;

    audit::register_mac_function(&conn, audit_key)?;
    // Prevent application-defined functions from being invoked by database schema
    // objects supplied by someone who possesses only the SQLCipher key.
    conn.execute_batch("PRAGMA trusted_schema = OFF;")?;

    // Run migrations, then reinstall trusted triggers before verification.
    let migrated_audit_chain = run_migrations(&conn, &audit_key)?;
    audit::install_integrity_triggers(&conn)?;
    let mut head = audit::verify_chain(&conn, &audit_key)?;

    if let Some(store) = &checkpoint_store {
        head = match checkpoint_mode {
            CheckpointMode::Reanchor => {
                // A Keychain read failure must not block an authorized recovery;
                // it only means the previous anchor cannot be described.
                let discrepancy =
                    match checkpoint_status(&conn, store.as_ref(), migrated_audit_chain) {
                        Ok(CheckpointStatus::Anchored(_)) => None,
                        Ok(CheckpointStatus::Discrepancy(message)) => Some(message),
                        Err(err) => Some(format!("previous checkpoint unreadable: {err}")),
                    };
                record_and_reanchor(
                    &conn,
                    &audit_key,
                    store.as_ref(),
                    &head,
                    discrepancy.as_deref(),
                    "recovery phrase or backup restore",
                )?
            }
            CheckpointMode::Strict | CheckpointMode::AcknowledgedReanchor => {
                match checkpoint_status(&conn, store.as_ref(), migrated_audit_chain)? {
                    CheckpointStatus::Anchored(stored) => {
                        if stored.as_ref() != Some(&head) {
                            store.write(&head)?;
                        }
                        head
                    }
                    CheckpointStatus::Discrepancy(message)
                        if checkpoint_mode == CheckpointMode::Strict =>
                    {
                        return Err(AppError::AuditCheckpointMismatch(message));
                    }
                    CheckpointStatus::Discrepancy(message) => record_and_reanchor(
                        &conn,
                        &audit_key,
                        store.as_ref(),
                        &head,
                        Some(&message),
                        "user confirmation at unlock",
                    )?,
                }
            }
        };
    }

    Ok(DbPool {
        inner: Arc::new(DbInner {
            conn: Mutex::new(conn),
            audit_key,
            checkpoint_store,
            checkpoint: Mutex::new(head),
            checkpoint_error: Mutex::new(None),
        }),
    })
}

/// Run database migrations
fn run_migrations(conn: &Connection, audit_key: &[u8; MAC_SIZE]) -> Result<bool, AppError> {
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

    // Migration 11: Model registry for multi-model management
    if version < 11 {
        log::info!("Running migration 011: Model registry");
        conn.execute_batch(include_str!("migrations/011_model_registry.sql"))?;
        conn.execute("PRAGMA user_version = 11;", [])?;
    }

    // Migration 12: Letters table for formal letter drafting
    if version < 12 {
        log::info!("Running migration 012: Letters");
        conn.execute_batch(include_str!("migrations/012_letters.sql"))?;
        conn.execute("PRAGMA user_version = 12;", [])?;
    }

    // Migration 13: Practice settings and onboarding state
    if version < 13 {
        log::info!("Running migration 013: Practice settings");
        conn.execute_batch(include_str!("migrations/013_practice_settings.sql"))?;
        conn.execute("PRAGMA user_version = 13;", [])?;
    }

    let migrated_audit_chain = version < 14;
    if migrated_audit_chain {
        log::info!("Running migration 014: Audit HMAC chain");
        audit::migrate_to_hmac_chain(conn, audit_key)?;
    }

    // Migration 15: Provenance-bearing evidence index for patient-history RAG
    if version < 15 {
        log::info!("Running migration 015: Evidence provenance");
        conn.execute_batch(include_str!("migrations/015_evidence_provenance.sql"))?;
        conn.execute("PRAGMA user_version = 15;", [])?;
    }

    // Migration 16: Durable, reversible versions for chat-generated drafts.
    if version < 16 {
        log::info!("Running migration 016: Chat draft versions");
        conn.execute_batch(include_str!("migrations/016_chat_draft_versions.sql"))?;
        conn.execute("PRAGMA user_version = 16;", [])?;
    }

    log::info!("Database migrations complete");
    Ok(migrated_audit_chain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[derive(Default)]
    struct MemoryCheckpointStore {
        checkpoint: Mutex<Option<ChainCheckpoint>>,
    }

    impl MemoryCheckpointStore {
        fn current(&self) -> Option<ChainCheckpoint> {
            self.checkpoint.lock().unwrap().clone()
        }
    }

    impl CheckpointStore for MemoryCheckpointStore {
        fn read(&self) -> Result<Option<ChainCheckpoint>, AppError> {
            Ok(self.current())
        }

        fn write(&self, checkpoint: &ChainCheckpoint) -> Result<(), AppError> {
            *self.checkpoint.lock().unwrap() = Some(checkpoint.clone());
            Ok(())
        }

        fn reset(&self) -> Result<(), AppError> {
            *self.checkpoint.lock().unwrap() = None;
            Ok(())
        }
    }

    fn init_with_memory_checkpoint(
        db_path: &Path,
        key: &[u8; 32],
        store: Arc<MemoryCheckpointStore>,
        mode: CheckpointMode,
    ) -> Result<DbPool, AppError> {
        init_db_internal(
            db_path,
            key,
            audit::derive_mac_key(key, key),
            Some(store),
            mode,
        )
    }

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
        assert_eq!(version, LATEST_SCHEMA_VERSION);
    }

    #[test]
    fn checkpoint_advances_only_after_transaction_commit() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let key = crate::crypto::generate_key();
        let store = Arc::new(MemoryCheckpointStore::default());
        let pool =
            init_with_memory_checkpoint(&db_path, &key, Arc::clone(&store), CheckpointMode::Strict)
                .unwrap();

        {
            let conn = pool.conn().unwrap();
            {
                let tx = conn.unchecked_transaction().unwrap();
                audit::log(
                    &tx,
                    audit::AuditAction::Create,
                    "patient",
                    Some("rolled-back"),
                    None,
                )
                .unwrap();
            }
        }
        assert_eq!(store.current().unwrap().sequence, 0);

        {
            let conn = pool.conn().unwrap();
            let tx = conn.unchecked_transaction().unwrap();
            audit::log(
                &tx,
                audit::AuditAction::Create,
                "patient",
                Some("committed"),
                None,
            )
            .unwrap();
            tx.commit().unwrap();
        }
        assert_eq!(store.current().unwrap().sequence, 1);
    }

    #[test]
    fn keychain_style_checkpoint_detects_tail_truncation_on_reopen() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let key = crate::crypto::generate_key();
        let store = Arc::new(MemoryCheckpointStore::default());

        {
            let pool = init_with_memory_checkpoint(
                &db_path,
                &key,
                Arc::clone(&store),
                CheckpointMode::Strict,
            )
            .unwrap();
            let conn = pool.conn().unwrap();
            audit::log(
                &conn,
                audit::AuditAction::Create,
                "patient",
                Some("p-1"),
                None,
            )
            .unwrap();
            audit::log(
                &conn,
                audit::AuditAction::View,
                "patient",
                Some("p-1"),
                None,
            )
            .unwrap();
        }
        assert_eq!(store.current().unwrap().sequence, 2);

        // Simulate a database-key attacker removing the trigger and newest row.
        {
            let pool = init_db(&db_path, &key).unwrap();
            let conn = pool.conn().unwrap();
            conn.execute_batch("DROP TRIGGER audit_log_no_delete;")
                .unwrap();
            conn.execute("DELETE FROM audit_log WHERE sequence = 2", [])
                .unwrap();
        }

        assert!(matches!(
            init_with_memory_checkpoint(&db_path, &key, Arc::clone(&store), CheckpointMode::Strict,),
            Err(AppError::AuditCheckpointMismatch(_))
        ));

        // An explicitly authorized recovery/restore may establish a new baseline,
        // but only after the remaining HMAC chain verifies. The discrepancy it
        // re-anchored over is appended as sequence 2.
        let pool = init_with_memory_checkpoint(
            &db_path,
            &key,
            Arc::clone(&store),
            CheckpointMode::Reanchor,
        )
        .unwrap();
        assert_eq!(store.current().unwrap().sequence, 2);
        let (action, details) = last_audit_entry(&pool);
        assert_eq!(action, "checkpoint_reanchored");
        assert!(details.contains("recovery phrase or backup restore"));
    }

    /// Builds a database whose audit head (sequence 1) is behind a Keychain
    /// checkpoint at sequence 2, as after restoring an older copy of the file.
    fn database_older_than_checkpoint(
        dir: &tempfile::TempDir,
    ) -> (std::path::PathBuf, [u8; 32], Arc<MemoryCheckpointStore>) {
        let db_path = dir.path().join("test.db");
        let snapshot = dir.path().join("snapshot.db");
        let key = crate::crypto::generate_key();
        let store = Arc::new(MemoryCheckpointStore::default());
        {
            let pool = init_with_memory_checkpoint(
                &db_path,
                &key,
                Arc::clone(&store),
                CheckpointMode::Strict,
            )
            .unwrap();
            let conn = pool.conn().unwrap();
            audit::log(
                &conn,
                audit::AuditAction::Create,
                "patient",
                Some("p-1"),
                None,
            )
            .unwrap();
        }
        std::fs::copy(&db_path, &snapshot).unwrap();
        {
            let pool = init_with_memory_checkpoint(
                &db_path,
                &key,
                Arc::clone(&store),
                CheckpointMode::Strict,
            )
            .unwrap();
            let conn = pool.conn().unwrap();
            audit::log(
                &conn,
                audit::AuditAction::View,
                "patient",
                Some("p-1"),
                None,
            )
            .unwrap();
        }
        assert_eq!(store.current().unwrap().sequence, 2);
        std::fs::copy(&snapshot, &db_path).unwrap();
        (db_path, key, store)
    }

    fn last_audit_entry(pool: &DbPool) -> (String, String) {
        pool.conn()
            .unwrap()
            .query_row(
                "SELECT action, COALESCE(details, '') FROM audit_log ORDER BY sequence DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap()
    }

    #[test]
    fn restored_older_database_reports_mismatch_then_unlocks_after_acknowledgement() {
        let dir = tempdir().unwrap();
        let (db_path, key, store) = database_older_than_checkpoint(&dir);

        assert!(matches!(
            init_with_memory_checkpoint(&db_path, &key, Arc::clone(&store), CheckpointMode::Strict),
            Err(AppError::AuditCheckpointMismatch(_))
        ));
        // The refused open must not move the anchor.
        assert_eq!(store.current().unwrap().sequence, 2);

        let pool = init_with_memory_checkpoint(
            &db_path,
            &key,
            Arc::clone(&store),
            CheckpointMode::AcknowledgedReanchor,
        )
        .unwrap();
        let (action, details) = last_audit_entry(&pool);
        assert_eq!(action, "checkpoint_reanchored");
        assert!(details.contains("checkpoint sequence 2"), "{details}");
        assert!(details.contains("audit head was sequence 1"), "{details}");
        assert!(details.contains("user confirmation"), "{details}");
        assert_eq!(store.current().unwrap().sequence, 2);
        drop(pool);

        // The new anchor is the chain head including the recorded discrepancy,
        // so the next ordinary unlock succeeds.
        init_with_memory_checkpoint(&db_path, &key, Arc::clone(&store), CheckpointMode::Strict)
            .unwrap();
    }

    #[test]
    fn missing_checkpoint_on_existing_chain_can_be_acknowledged() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let key = crate::crypto::generate_key();
        let store = Arc::new(MemoryCheckpointStore::default());
        init_with_memory_checkpoint(&db_path, &key, Arc::clone(&store), CheckpointMode::Strict)
            .unwrap();
        store.reset().unwrap();

        assert!(matches!(
            init_with_memory_checkpoint(&db_path, &key, Arc::clone(&store), CheckpointMode::Strict),
            Err(AppError::AuditCheckpointMismatch(_))
        ));
        let pool = init_with_memory_checkpoint(
            &db_path,
            &key,
            Arc::clone(&store),
            CheckpointMode::AcknowledgedReanchor,
        )
        .unwrap();
        let (action, details) = last_audit_entry(&pool);
        assert_eq!(action, "checkpoint_reanchored");
        assert!(details.contains("missing"), "{details}");
        assert_eq!(store.current().unwrap().sequence, 1);
    }

    #[test]
    fn acknowledgement_without_a_mismatch_records_nothing() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let key = crate::crypto::generate_key();
        let store = Arc::new(MemoryCheckpointStore::default());
        init_with_memory_checkpoint(&db_path, &key, Arc::clone(&store), CheckpointMode::Strict)
            .unwrap();

        let pool = init_with_memory_checkpoint(
            &db_path,
            &key,
            Arc::clone(&store),
            CheckpointMode::AcknowledgedReanchor,
        )
        .unwrap();
        let count: i64 = pool
            .conn()
            .unwrap()
            .query_row("SELECT count(*) FROM audit_log", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
        assert_eq!(store.current().unwrap().sequence, 0);
    }

    #[test]
    fn acknowledgement_cannot_accept_a_chain_that_does_not_verify() {
        let dir = tempdir().unwrap();
        let (db_path, key, store) = database_older_than_checkpoint(&dir);
        {
            let pool = init_db(&db_path, &key).unwrap();
            let conn = pool.conn().unwrap();
            conn.execute_batch("DROP TRIGGER audit_log_no_update;")
                .unwrap();
            conn.execute(
                "UPDATE audit_log SET entity_id = 'forged' WHERE sequence = 1",
                [],
            )
            .unwrap();
        }

        assert!(matches!(
            init_with_memory_checkpoint(
                &db_path,
                &key,
                Arc::clone(&store),
                CheckpointMode::AcknowledgedReanchor,
            ),
            Err(AppError::AuditIntegrity(_))
        ));
        assert_eq!(store.current().unwrap().sequence, 2);
    }

    /// A store whose reads fail the way a denied Keychain prompt does.
    struct DeniedCheckpointStore;

    impl CheckpointStore for DeniedCheckpointStore {
        fn read(&self) -> Result<Option<ChainCheckpoint>, AppError> {
            Err(AppError::KeychainAccessDenied("denied".to_string()))
        }

        fn write(&self, _checkpoint: &ChainCheckpoint) -> Result<(), AppError> {
            panic!("a denied read must never lead to a checkpoint write");
        }

        fn reset(&self) -> Result<(), AppError> {
            panic!("a denied read must never lead to a checkpoint reset");
        }
    }

    #[test]
    fn keychain_access_denial_is_not_treated_as_a_checkpoint_mismatch() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let key = crate::crypto::generate_key();
        init_with_memory_checkpoint(
            &db_path,
            &key,
            Arc::new(MemoryCheckpointStore::default()),
            CheckpointMode::Strict,
        )
        .unwrap();

        for mode in [CheckpointMode::Strict, CheckpointMode::AcknowledgedReanchor] {
            let result = init_db_internal(
                &db_path,
                &key,
                audit::derive_mac_key(&key, &key),
                Some(Arc::new(DeniedCheckpointStore)),
                mode,
            );
            assert!(
                matches!(result, Err(AppError::KeychainAccessDenied(_))),
                "mode {mode:?}"
            );
        }
    }
}
