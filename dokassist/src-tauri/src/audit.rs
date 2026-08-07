use crate::error::AppError;
use ring::hmac;
use rusqlite::functions::{Context, FunctionFlags};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

pub const MAC_SIZE: usize = 32;
pub const GENESIS_MAC: [u8; MAC_SIZE] = [0; MAC_SIZE];
const CHAIN_VERSION: u8 = 1;
const MAC_FUNCTION: &str = "audit_chain_mac_v1";
const AUDIT_KEY_CONTEXT: &[u8] = b"RamDoc audit HMAC key v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainCheckpoint {
    pub sequence: i64,
    pub mac: [u8; MAC_SIZE],
}

impl ChainCheckpoint {
    pub fn genesis() -> Self {
        Self {
            sequence: 0,
            mac: GENESIS_MAC,
        }
    }

    pub fn to_bytes(&self) -> [u8; 41] {
        let mut encoded = [0u8; 41];
        encoded[0] = CHAIN_VERSION;
        encoded[1..9].copy_from_slice(&self.sequence.to_be_bytes());
        encoded[9..].copy_from_slice(&self.mac);
        encoded
    }

    pub fn from_bytes(encoded: &[u8]) -> Result<Self, AppError> {
        if encoded.len() != 41 || encoded[0] != CHAIN_VERSION {
            return Err(AppError::AuditIntegrity(
                "invalid Keychain audit checkpoint encoding".to_string(),
            ));
        }
        let mut sequence_bytes = [0u8; 8];
        sequence_bytes.copy_from_slice(&encoded[1..9]);
        let sequence = i64::from_be_bytes(sequence_bytes);
        if sequence < 0 {
            return Err(AppError::AuditIntegrity(
                "negative Keychain audit checkpoint sequence".to_string(),
            ));
        }
        let mut mac = [0u8; MAC_SIZE];
        mac.copy_from_slice(&encoded[9..]);
        Ok(Self { sequence, mac })
    }
}

#[derive(Debug)]
struct ChainRow {
    id: i64,
    sequence: i64,
    timestamp: String,
    action: String,
    entity_type: String,
    entity_id: Option<String>,
    details: Option<String>,
    previous_mac: Vec<u8>,
    entry_mac: Vec<u8>,
}

/// Derive a domain-separated audit MAC key that requires both master keys.
/// Compromise of either the SQLCipher key or filesystem key alone is therefore
/// insufficient to forge audit entries, while mnemonic recovery remains possible.
pub fn derive_mac_key(db_key: &[u8; 32], fs_key: &[u8; 32]) -> [u8; MAC_SIZE] {
    let key = hmac::Key::new(hmac::HMAC_SHA256, fs_key);
    let mut input = [0u8; AUDIT_KEY_CONTEXT.len() + 32];
    input[..AUDIT_KEY_CONTEXT.len()].copy_from_slice(AUDIT_KEY_CONTEXT);
    input[AUDIT_KEY_CONTEXT.len()..].copy_from_slice(db_key);
    let tag = hmac::sign(&key, &input);
    input.zeroize();
    let mut derived = [0u8; MAC_SIZE];
    derived.copy_from_slice(tag.as_ref());
    derived
}

fn append_bytes(encoded: &mut Vec<u8>, value: &[u8]) {
    encoded.extend_from_slice(&(value.len() as u64).to_be_bytes());
    encoded.extend_from_slice(value);
}

fn append_optional(encoded: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            encoded.push(1);
            append_bytes(encoded, value.as_bytes());
        }
        None => encoded.push(0),
    }
}

#[allow(clippy::too_many_arguments)]
fn encode_entry(
    id: i64,
    sequence: i64,
    timestamp: &str,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    details: Option<&str>,
    previous_mac: &[u8],
) -> Result<Vec<u8>, AppError> {
    if previous_mac.len() != MAC_SIZE {
        return Err(AppError::AuditIntegrity(
            "audit row contains an invalid previous MAC length".to_string(),
        ));
    }

    let mut encoded = Vec::with_capacity(192);
    encoded.push(CHAIN_VERSION);
    encoded.extend_from_slice(&id.to_be_bytes());
    encoded.extend_from_slice(&sequence.to_be_bytes());
    append_bytes(&mut encoded, timestamp.as_bytes());
    append_bytes(&mut encoded, action.as_bytes());
    append_bytes(&mut encoded, entity_type.as_bytes());
    append_optional(&mut encoded, entity_id);
    append_optional(&mut encoded, details);
    encoded.extend_from_slice(previous_mac);

    Ok(encoded)
}

#[allow(clippy::too_many_arguments)]
fn compute_entry_mac(
    mac_key: &[u8; MAC_SIZE],
    id: i64,
    sequence: i64,
    timestamp: &str,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    details: Option<&str>,
    previous_mac: &[u8],
) -> Result<[u8; MAC_SIZE], AppError> {
    let encoded = encode_entry(
        id,
        sequence,
        timestamp,
        action,
        entity_type,
        entity_id,
        details,
        previous_mac,
    )?;

    let key = hmac::Key::new(hmac::HMAC_SHA256, mac_key);
    let tag = hmac::sign(&key, &encoded);
    let mut mac = [0u8; MAC_SIZE];
    mac.copy_from_slice(tag.as_ref());
    Ok(mac)
}

#[allow(clippy::too_many_arguments)]
fn verify_entry_mac(
    mac_key: &[u8; MAC_SIZE],
    id: i64,
    sequence: i64,
    timestamp: &str,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    details: Option<&str>,
    previous_mac: &[u8],
    entry_mac: &[u8],
) -> Result<(), AppError> {
    let encoded = encode_entry(
        id,
        sequence,
        timestamp,
        action,
        entity_type,
        entity_id,
        details,
        previous_mac,
    )?;
    hmac::verify(
        &hmac::Key::new(hmac::HMAC_SHA256, mac_key),
        &encoded,
        entry_mac,
    )
    .map_err(|_| AppError::AuditIntegrity(format!("entry MAC mismatch at sequence {sequence}")))
}

pub fn register_mac_function(conn: &Connection, mac_key: [u8; MAC_SIZE]) -> Result<(), AppError> {
    let mac_key = Zeroizing::new(mac_key);
    conn.create_scalar_function(
        MAC_FUNCTION,
        8,
        FunctionFlags::SQLITE_UTF8
            | FunctionFlags::SQLITE_DETERMINISTIC
            | FunctionFlags::SQLITE_DIRECTONLY,
        move |ctx: &Context<'_>| {
            let id: i64 = ctx.get(0)?;
            let sequence: i64 = ctx.get(1)?;
            let timestamp: String = ctx.get(2)?;
            let action: String = ctx.get(3)?;
            let entity_type: String = ctx.get(4)?;
            let entity_id: Option<String> = ctx.get(5)?;
            let details: Option<String> = ctx.get(6)?;
            let previous_mac: Vec<u8> = ctx.get(7)?;
            compute_entry_mac(
                &mac_key,
                id,
                sequence,
                &timestamp,
                &action,
                &entity_type,
                entity_id.as_deref(),
                details.as_deref(),
                &previous_mac,
            )
            .map(|mac| mac.to_vec())
            .map_err(|err| rusqlite::Error::UserFunctionError(Box::new(err)))
        },
    )?;
    Ok(())
}

/// Audit action types for nDSG compliance logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    View,
    Create,
    Update,
    Delete,
    Export,
    Import,
    LlmQuery,
    Login,
    Logout,
    RecoveryUsed,
}

impl AuditAction {
    pub fn as_str(&self) -> &str {
        match self {
            AuditAction::View => "view",
            AuditAction::Create => "create",
            AuditAction::Update => "update",
            AuditAction::Delete => "delete",
            AuditAction::Export => "export",
            AuditAction::Import => "import",
            AuditAction::LlmQuery => "llm_query",
            AuditAction::Login => "login",
            AuditAction::Logout => "logout",
            AuditAction::RecoveryUsed => "recovery_used",
        }
    }
}

/// Audit log entry returned from queries
#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub timestamp: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub details: Option<String>,
}

/// Log an auditable action. Call this from every command that touches patient data.
///
/// # Arguments
/// * `conn` - Database connection
/// * `action` - The action being performed
/// * `entity_type` - Type of entity (e.g., "patient", "file", "session")
/// * `entity_id` - Optional UUID of the entity
/// * `details` - Optional details (field names changed, not values - no PHI)
///
/// # Examples
/// ```ignore
/// use crate::audit::{self, AuditAction};
/// # let conn = rusqlite::Connection::open_in_memory().unwrap();
/// # let id = "patient-123";
/// audit::log(&conn, AuditAction::View, "patient", Some(&id), None)?;
/// audit::log(&conn, AuditAction::Update, "patient", Some(&id), Some("fields: first_name,last_name"))?;
/// # Ok::<(), crate::error::AppError>(())
/// ```
pub fn log(
    conn: &Connection,
    action: AuditAction,
    entity_type: &str,
    entity_id: Option<&str>,
    details: Option<&str>,
) -> Result<(), AppError> {
    let timestamp = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO audit_log (
             id, timestamp, action, entity_type, entity_id, details,
             sequence, previous_mac, entry_mac
         )
         SELECT
             COALESCE(MAX(id), 0) + 1,
             ?1, ?2, ?3, ?4, ?5,
             COALESCE(MAX(sequence), 0) + 1,
             COALESCE(
                 (SELECT entry_mac FROM audit_log ORDER BY sequence DESC LIMIT 1),
                 zeroblob(32)
             ),
             audit_chain_mac_v1(
                 COALESCE(MAX(id), 0) + 1,
                 COALESCE(MAX(sequence), 0) + 1,
                 ?1, ?2, ?3, ?4, ?5,
                 COALESCE(
                     (SELECT entry_mac FROM audit_log ORDER BY sequence DESC LIMIT 1),
                     zeroblob(32)
                 )
             )
         FROM audit_log",
        rusqlite::params![timestamp, action.as_str(), entity_type, entity_id, details,],
    )?;

    Ok(())
}

fn chain_rows(conn: &Connection) -> Result<Vec<ChainRow>, AppError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, sequence, timestamp, action, entity_type, entity_id, details,
                    previous_mac, entry_mac
             FROM audit_log
             ORDER BY sequence ASC",
        )
        .map_err(|err| AppError::AuditIntegrity(format!("failed to read audit chain rows: {err}")))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ChainRow {
                id: row.get(0)?,
                sequence: row.get(1)?,
                timestamp: row.get(2)?,
                action: row.get(3)?,
                entity_type: row.get(4)?,
                entity_id: row.get(5)?,
                details: row.get(6)?,
                previous_mac: row.get(7)?,
                entry_mac: row.get(8)?,
            })
        })
        .map_err(|err| AppError::AuditIntegrity(format!("failed to read audit chain rows: {err}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| AppError::AuditIntegrity(format!("failed to read audit chain rows: {err}")))?;
    Ok(rows)
}

/// Verify every row in sequence order and return the authenticated chain head.
/// Any modification, insertion, deletion in the middle, or reordering fails.
pub fn verify_chain(
    conn: &Connection,
    mac_key: &[u8; MAC_SIZE],
) -> Result<ChainCheckpoint, AppError> {
    let rows = chain_rows(conn)?;
    let mut expected_sequence = 1i64;
    let mut previous_mac = GENESIS_MAC;

    for row in rows {
        if row.sequence != expected_sequence {
            return Err(AppError::AuditIntegrity(format!(
                "expected audit sequence {}, found {}",
                expected_sequence, row.sequence
            )));
        }
        if row.previous_mac.as_slice() != previous_mac {
            return Err(AppError::AuditIntegrity(format!(
                "previous MAC mismatch at audit sequence {}",
                row.sequence
            )));
        }

        let expected_mac = compute_entry_mac(
            mac_key,
            row.id,
            row.sequence,
            &row.timestamp,
            &row.action,
            &row.entity_type,
            row.entity_id.as_deref(),
            row.details.as_deref(),
            &row.previous_mac,
        )?;
        ring::constant_time::verify_slices_are_equal(&expected_mac[..], row.entry_mac.as_slice())
            .map_err(|_| {
                AppError::AuditIntegrity(format!(
                    "entry MAC mismatch at sequence {}",
                    row.sequence
                ))
            })?;

        previous_mac = expected_mac;
        expected_sequence += 1;
    }

    Ok(ChainCheckpoint {
        sequence: expected_sequence - 1,
        mac: previous_mac,
    })
}

fn verify_chain_with_registered_key(conn: &Connection) -> Result<ChainCheckpoint, AppError> {
    let rows = chain_rows(conn)?;
    let mut expected_sequence = 1i64;
    let mut previous_mac = GENESIS_MAC;

    for row in rows {
        if row.sequence != expected_sequence || row.previous_mac.as_slice() != previous_mac {
            return Err(AppError::AuditIntegrity(format!(
                "audit chain linkage failed at sequence {}",
                row.sequence
            )));
        }
        let expected_mac: Vec<u8> = conn.query_row(
            "SELECT audit_chain_mac_v1(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                row.id,
                row.sequence,
                row.timestamp,
                row.action,
                row.entity_type,
                row.entity_id,
                row.details,
                row.previous_mac,
            ],
            |result| result.get(0),
        )?;
        if expected_mac.len() != MAC_SIZE {
            return Err(AppError::AuditIntegrity(format!(
                "entry MAC length mismatch at sequence {}",
                row.sequence
            )));
        }
        // The registered SQLite function owns the same protected key and computes
        // the canonical HMAC. This comparison handles only non-secret tag bytes.
        if expected_mac.as_slice() != row.entry_mac {
            return Err(AppError::AuditIntegrity(format!(
                "entry MAC mismatch at audit sequence {}",
                row.sequence
            )));
        }
        previous_mac.copy_from_slice(&expected_mac);
        expected_sequence += 1;
    }

    Ok(ChainCheckpoint {
        sequence: expected_sequence - 1,
        mac: previous_mac,
    })
}

/// Verify that a trusted checkpoint is present in the valid chain. A chain may
/// be ahead after a crash between the SQLite commit and Keychain checkpoint.
pub fn verify_checkpoint(conn: &Connection, checkpoint: &ChainCheckpoint) -> Result<(), AppError> {
    if checkpoint.sequence == 0 {
        if checkpoint.mac == GENESIS_MAC {
            return Ok(());
        }
        return Err(AppError::AuditIntegrity(
            "invalid genesis audit checkpoint".to_string(),
        ));
    }

    let stored: Option<Vec<u8>> = conn
        .query_row(
            "SELECT entry_mac FROM audit_log WHERE sequence = ?1",
            [checkpoint.sequence],
            |row| row.get(0),
        )
        .optional()?;
    match stored {
        Some(mac) if mac.as_slice() == checkpoint.mac => Ok(()),
        Some(_) => Err(AppError::AuditIntegrity(format!(
            "Keychain checkpoint MAC does not match audit sequence {}",
            checkpoint.sequence
        ))),
        None => Err(AppError::AuditIntegrity(format!(
            "audit log was truncated before Keychain checkpoint sequence {}",
            checkpoint.sequence
        ))),
    }
}

pub fn verify_chain_from_checkpoint(
    conn: &Connection,
    mac_key: &[u8; MAC_SIZE],
    checkpoint: &ChainCheckpoint,
) -> Result<ChainCheckpoint, AppError> {
    verify_checkpoint(conn, checkpoint)?;
    let mut stmt = conn.prepare(
        "SELECT id, sequence, timestamp, action, entity_type, entity_id, details,
                previous_mac, entry_mac
         FROM audit_log WHERE sequence > ?1 ORDER BY sequence ASC",
    )?;
    let rows = stmt
        .query_map([checkpoint.sequence], |row| {
            Ok(ChainRow {
                id: row.get(0)?,
                sequence: row.get(1)?,
                timestamp: row.get(2)?,
                action: row.get(3)?,
                entity_type: row.get(4)?,
                entity_id: row.get(5)?,
                details: row.get(6)?,
                previous_mac: row.get(7)?,
                entry_mac: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut expected_sequence = checkpoint.sequence + 1;
    let mut previous_mac = checkpoint.mac;
    for row in rows {
        if row.sequence != expected_sequence || row.previous_mac.as_slice() != previous_mac {
            return Err(AppError::AuditIntegrity(format!(
                "audit chain linkage failed after checkpoint at sequence {}",
                row.sequence
            )));
        }
        verify_entry_mac(
            mac_key,
            row.id,
            row.sequence,
            &row.timestamp,
            &row.action,
            &row.entity_type,
            row.entity_id.as_deref(),
            row.details.as_deref(),
            &row.previous_mac,
            &row.entry_mac,
        )?;
        let expected_mac = compute_entry_mac(
            mac_key,
            row.id,
            row.sequence,
            &row.timestamp,
            &row.action,
            &row.entity_type,
            row.entity_id.as_deref(),
            row.details.as_deref(),
            &row.previous_mac,
        )?;
        previous_mac = expected_mac;
        expected_sequence += 1;
    }

    Ok(ChainCheckpoint {
        sequence: expected_sequence - 1,
        mac: previous_mac,
    })
}

pub fn migrate_to_hmac_chain(conn: &Connection, mac_key: &[u8; MAC_SIZE]) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(include_str!("migrations/014_audit_hmac_chain.sql"))?;

    let legacy_rows = {
        let mut stmt = tx.prepare(
            "SELECT id, timestamp, action, entity_type, entity_id, details
             FROM audit_log ORDER BY id ASC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };

    let mut previous_mac = GENESIS_MAC;
    for (index, (id, timestamp, action, entity_type, entity_id, details)) in
        legacy_rows.into_iter().enumerate()
    {
        let sequence = index as i64 + 1;
        let entry_mac = compute_entry_mac(
            mac_key,
            id,
            sequence,
            &timestamp,
            &action,
            &entity_type,
            entity_id.as_deref(),
            details.as_deref(),
            &previous_mac,
        )?;
        tx.execute(
            "UPDATE audit_log
             SET sequence = ?1, previous_mac = ?2, entry_mac = ?3
             WHERE id = ?4",
            rusqlite::params![sequence, previous_mac.as_slice(), entry_mac.as_slice(), id],
        )?;
        previous_mac = entry_mac;
    }

    install_integrity_triggers(&tx)?;
    tx.execute("PRAGMA user_version = 14", [])?;
    tx.commit()?;
    Ok(())
}

pub fn install_integrity_triggers(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(include_str!("migrations/014_audit_integrity_triggers.sql"))?;
    Ok(())
}

/// Query audit log with optional filters
///
/// # Arguments
/// * `conn` - Database connection
/// * `entity_type` - Optional filter by entity type
/// * `entity_id` - Optional filter by entity ID
/// * `from` - Optional start date (ISO 8601 format)
/// * `to` - Optional end date (ISO 8601 format)
/// * `limit` - Maximum number of entries to return
/// * `offset` - Offset for pagination
///
/// # Returns
/// Vector of audit entries, ordered by timestamp descending (newest first)
pub fn query_log(
    conn: &Connection,
    entity_type: Option<&str>,
    entity_id: Option<&str>,
    from: Option<&str>,
    to: Option<&str>,
    limit: u32,
    offset: u32,
) -> Result<Vec<AuditEntry>, AppError> {
    // Never present audit data that has not passed cryptographic verification.
    verify_chain_with_registered_key(conn)?;

    let mut query = String::from(
        "SELECT id, timestamp, action, entity_type, entity_id, details
         FROM audit_log
         WHERE 1=1",
    );

    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(et) = entity_type {
        query.push_str(" AND entity_type = ?");
        params.push(Box::new(et.to_string()));
    }

    if let Some(eid) = entity_id {
        query.push_str(" AND entity_id = ?");
        params.push(Box::new(eid.to_string()));
    }

    if let Some(from_date) = from {
        query.push_str(" AND timestamp >= ?");
        params.push(Box::new(from_date.to_string()));
    }

    if let Some(to_date) = to {
        query.push_str(" AND timestamp <= ?");
        params.push(Box::new(to_date.to_string()));
    }

    query.push_str(" ORDER BY timestamp DESC, id DESC LIMIT ? OFFSET ?");
    params.push(Box::new(limit as i64));
    params.push(Box::new(offset as i64));

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();

    let mut stmt = conn.prepare(&query)?;
    let entries = stmt
        .query_map(&param_refs[..], |row| {
            Ok(AuditEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                action: row.get(2)?,
                entity_type: row.get(3)?,
                entity_id: row.get(4)?,
                details: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(entries)
}

/// Create the audit_log table in the database
/// This should be called during database initialization
///
/// Note: In production, the audit_log table is created via migrations (001_initial.sql).
/// This function is provided for testing purposes and should match the migration schema.
pub fn create_table(conn: &Connection) -> Result<(), AppError> {
    let test_key = derive_mac_key(&[0x3C; 32], &[0xA5; 32]);
    register_mac_function(conn, test_key)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            action TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT,
            details TEXT,
            sequence INTEGER NOT NULL UNIQUE,
            previous_mac BLOB NOT NULL CHECK(length(previous_mac) = 32),
            entry_mac BLOB NOT NULL CHECK(length(entry_mac) = 32)
        )",
        [],
    )?;

    // Create indexes matching the migration schema
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp DESC)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit_log(entity_type, entity_id)",
        [],
    )?;

    install_integrity_triggers(conn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_mac_key() -> [u8; MAC_SIZE] {
        derive_mac_key(&[0x3C; 32], &[0xA5; 32])
    }

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_table(&conn).unwrap();
        conn
    }

    #[test]
    fn test_log_and_query() {
        let conn = setup_test_db();

        // Log some entries
        log(
            &conn,
            AuditAction::Create,
            "patient",
            Some("patient-123"),
            None,
        )
        .unwrap();
        log(
            &conn,
            AuditAction::View,
            "patient",
            Some("patient-123"),
            None,
        )
        .unwrap();
        log(
            &conn,
            AuditAction::Update,
            "patient",
            Some("patient-123"),
            Some("fields: first_name"),
        )
        .unwrap();

        // Query all entries
        let entries = query_log(&conn, None, None, None, None, 100, 0).unwrap();
        assert_eq!(entries.len(), 3);

        // Should be ordered newest first
        assert_eq!(entries[0].action, "update");
        assert_eq!(entries[1].action, "view");
        assert_eq!(entries[2].action, "create");
    }

    #[test]
    fn test_filter_by_entity_type() {
        let conn = setup_test_db();

        log(
            &conn,
            AuditAction::Create,
            "patient",
            Some("patient-123"),
            None,
        )
        .unwrap();
        log(&conn, AuditAction::Create, "file", Some("file-456"), None).unwrap();
        log(
            &conn,
            AuditAction::Create,
            "patient",
            Some("patient-789"),
            None,
        )
        .unwrap();

        let entries = query_log(&conn, Some("patient"), None, None, None, 100, 0).unwrap();
        assert_eq!(entries.len(), 2);

        for entry in entries {
            assert_eq!(entry.entity_type, "patient");
        }
    }

    #[test]
    fn test_filter_by_entity_id() {
        let conn = setup_test_db();

        log(
            &conn,
            AuditAction::Create,
            "patient",
            Some("patient-123"),
            None,
        )
        .unwrap();
        log(
            &conn,
            AuditAction::View,
            "patient",
            Some("patient-123"),
            None,
        )
        .unwrap();
        log(
            &conn,
            AuditAction::View,
            "patient",
            Some("patient-456"),
            None,
        )
        .unwrap();

        let entries = query_log(&conn, None, Some("patient-123"), None, None, 100, 0).unwrap();
        assert_eq!(entries.len(), 2);

        for entry in entries {
            assert_eq!(entry.entity_id, Some("patient-123".to_string()));
        }
    }

    #[test]
    fn test_pagination() {
        let conn = setup_test_db();

        // Create 10 entries
        for i in 0..10 {
            log(
                &conn,
                AuditAction::View,
                "patient",
                Some(&format!("patient-{}", i)),
                None,
            )
            .unwrap();
        }

        // Get first 5
        let page1 = query_log(&conn, None, None, None, None, 5, 0).unwrap();
        assert_eq!(page1.len(), 5);

        // Get next 5
        let page2 = query_log(&conn, None, None, None, None, 5, 5).unwrap();
        assert_eq!(page2.len(), 5);

        // Ensure no overlap
        assert_ne!(page1[0].id, page2[0].id);
    }

    #[test]
    fn test_no_phi_in_details() {
        let conn = setup_test_db();

        // Correct usage: only field names, no values
        log(
            &conn,
            AuditAction::Update,
            "patient",
            Some("patient-123"),
            Some("fields: first_name,last_name,date_of_birth"),
        )
        .unwrap();

        let entries = query_log(&conn, None, None, None, None, 100, 0).unwrap();
        assert_eq!(entries.len(), 1);

        // Details should only contain field names, not actual patient data
        assert!(entries[0].details.as_ref().unwrap().contains("fields:"));
        assert!(!entries[0].details.as_ref().unwrap().contains("John")); // no names
        assert!(!entries[0].details.as_ref().unwrap().contains("@")); // no emails
    }

    #[test]
    fn test_audit_actions() {
        assert_eq!(AuditAction::View.as_str(), "view");
        assert_eq!(AuditAction::Create.as_str(), "create");
        assert_eq!(AuditAction::Update.as_str(), "update");
        assert_eq!(AuditAction::Delete.as_str(), "delete");
        assert_eq!(AuditAction::Export.as_str(), "export");
        assert_eq!(AuditAction::Import.as_str(), "import");
        assert_eq!(AuditAction::LlmQuery.as_str(), "llm_query");
        assert_eq!(AuditAction::Login.as_str(), "login");
        assert_eq!(AuditAction::Logout.as_str(), "logout");
        assert_eq!(AuditAction::RecoveryUsed.as_str(), "recovery_used");
    }

    #[test]
    fn hmac_chain_detects_modified_row() {
        let conn = setup_test_db();
        log(&conn, AuditAction::Create, "patient", Some("p-1"), None).unwrap();
        log(&conn, AuditAction::View, "patient", Some("p-1"), None).unwrap();
        let head = verify_chain(&conn, &test_mac_key()).unwrap();
        assert_eq!(head.sequence, 2);

        conn.execute_batch("DROP TRIGGER audit_log_no_update;")
            .unwrap();
        conn.execute(
            "UPDATE audit_log SET action = 'delete' WHERE sequence = 1",
            [],
        )
        .unwrap();

        assert!(matches!(
            verify_chain(&conn, &test_mac_key()),
            Err(AppError::AuditIntegrity(_))
        ));
        assert!(matches!(
            query_log(&conn, None, None, None, None, 100, 0),
            Err(AppError::AuditIntegrity(_))
        ));
    }

    #[test]
    fn external_checkpoint_detects_tail_truncation() {
        let conn = setup_test_db();
        log(&conn, AuditAction::Create, "patient", Some("p-1"), None).unwrap();
        log(&conn, AuditAction::View, "patient", Some("p-1"), None).unwrap();
        let anchored_head = verify_chain(&conn, &test_mac_key()).unwrap();

        conn.execute_batch("DROP TRIGGER audit_log_no_delete;")
            .unwrap();
        conn.execute("DELETE FROM audit_log WHERE sequence = 2", [])
            .unwrap();

        // The remaining prefix is internally valid, but it no longer reaches the
        // independently stored chain head.
        assert_eq!(verify_chain(&conn, &test_mac_key()).unwrap().sequence, 1);
        assert!(matches!(
            verify_checkpoint(&conn, &anchored_head),
            Err(AppError::AuditIntegrity(_))
        ));
    }

    #[test]
    fn chain_detects_middle_row_deletion() {
        let conn = setup_test_db();
        for id in 1..=3 {
            log(
                &conn,
                AuditAction::View,
                "patient",
                Some(&format!("p-{id}")),
                None,
            )
            .unwrap();
        }
        conn.execute_batch("DROP TRIGGER audit_log_no_delete;")
            .unwrap();
        conn.execute("DELETE FROM audit_log WHERE sequence = 2", [])
            .unwrap();
        assert!(matches!(
            verify_chain(&conn, &test_mac_key()),
            Err(AppError::AuditIntegrity(_))
        ));
    }

    #[test]
    fn forged_entry_fails_cryptographic_verification() {
        let conn = setup_test_db();
        conn.execute(
            "INSERT INTO audit_log (
                id, timestamp, action, entity_type, sequence, previous_mac, entry_mac
             ) VALUES (1, '2026-01-01T00:00:00Z', 'view', 'patient', 1,
                       zeroblob(32), zeroblob(32))",
            [],
        )
        .unwrap();
        assert!(matches!(
            verify_chain(&conn, &test_mac_key()),
            Err(AppError::AuditIntegrity(_))
        ));
    }

    #[test]
    fn schema_trigger_cannot_use_protected_hmac_function() {
        let conn = setup_test_db();
        conn.execute_batch(
            "CREATE TABLE attacker_probe(value TEXT);
             CREATE TRIGGER attacker_uses_hmac AFTER INSERT ON attacker_probe
             BEGIN
                 SELECT audit_chain_mac_v1(
                     1, 1, '2026-01-01T00:00:00Z', 'view', 'patient',
                     NULL, NULL, zeroblob(32)
                 );
             END;",
        )
        .unwrap();

        assert!(conn
            .execute("INSERT INTO attacker_probe(value) VALUES ('trigger')", [])
            .is_err());
    }

    #[test]
    fn rolled_back_audit_entry_does_not_advance_chain() {
        let conn = setup_test_db();
        {
            let tx = conn.unchecked_transaction().unwrap();
            log(&tx, AuditAction::Create, "patient", Some("p-1"), None).unwrap();
            // Dropping without commit rolls back both the clinical operation and audit row.
        }
        assert_eq!(
            verify_chain(&conn, &test_mac_key()).unwrap(),
            ChainCheckpoint::genesis()
        );
    }

    #[test]
    fn wrong_hmac_key_cannot_verify_chain() {
        let conn = setup_test_db();
        log(&conn, AuditAction::View, "patient", Some("p-1"), None).unwrap();
        let wrong_key = derive_mac_key(&[0xC3; 32], &[0x5A; 32]);
        assert!(matches!(
            verify_chain(&conn, &wrong_key),
            Err(AppError::AuditIntegrity(_))
        ));
    }

    #[test]
    fn migration_backfills_legacy_rows_atomically() {
        let conn = Connection::open_in_memory().unwrap();
        let key = test_mac_key();
        register_mac_function(&conn, key).unwrap();
        conn.execute_batch(
            "CREATE TABLE audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                action TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id TEXT,
                details TEXT
             );
             CREATE TRIGGER audit_log_no_update BEFORE UPDATE ON audit_log
             BEGIN SELECT RAISE(ABORT, 'append-only'); END;
             CREATE TRIGGER audit_log_no_delete BEFORE DELETE ON audit_log
             BEGIN SELECT RAISE(ABORT, 'append-only'); END;
             INSERT INTO audit_log(timestamp, action, entity_type, entity_id)
             VALUES ('2026-01-01T00:00:00Z', 'create', 'patient', 'p-1');
             INSERT INTO audit_log(timestamp, action, entity_type, entity_id)
             VALUES ('2026-01-02T00:00:00Z', 'view', 'patient', 'p-1');",
        )
        .unwrap();

        migrate_to_hmac_chain(&conn, &key).unwrap();
        assert_eq!(verify_chain(&conn, &key).unwrap().sequence, 2);
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            14
        );
        assert!(conn
            .execute("UPDATE audit_log SET action = 'delete' WHERE id = 1", [])
            .is_err());
    }
}
