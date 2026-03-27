use crate::error::AppError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

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
        "INSERT INTO audit_log (timestamp, action, entity_type, entity_id, details)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![timestamp, action.as_str(), entity_type, entity_id, details,],
    )?;

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
    conn.execute(
        "CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            action TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT,
            details TEXT
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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

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
}
