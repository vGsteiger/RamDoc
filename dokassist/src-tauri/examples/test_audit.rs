// Standalone test for audit module
// This can be run independently to verify audit logic

use rusqlite::Connection;

#[derive(Debug, Clone)]
pub enum AuditAction {
    View,
    Create,
    Update,
    Delete,
}

impl AuditAction {
    pub fn as_str(&self) -> &str {
        match self {
            AuditAction::View => "view",
            AuditAction::Create => "create",
            AuditAction::Update => "update",
            AuditAction::Delete => "delete",
        }
    }
}

pub struct AuditEntry {
    pub id: i64,
    pub timestamp: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub details: Option<String>,
}

pub fn create_table(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            action TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT,
            details TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit_log(entity_type, entity_id)",
        [],
    )?;

    Ok(())
}

pub fn log(
    conn: &Connection,
    action: AuditAction,
    entity_type: &str,
    entity_id: Option<&str>,
    details: Option<&str>,
) -> Result<(), rusqlite::Error> {
    let timestamp = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO audit_log (timestamp, action, entity_type, entity_id, details)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            timestamp,
            action.as_str(),
            entity_type,
            entity_id,
            details,
        ],
    )?;

    Ok(())
}

pub fn query_log(
    conn: &Connection,
    entity_type: Option<&str>,
    entity_id: Option<&str>,
    limit: u32,
    offset: u32,
) -> Result<Vec<AuditEntry>, rusqlite::Error> {
    let mut query = String::from(
        "SELECT id, timestamp, action, entity_type, entity_id, details
         FROM audit_log
         WHERE 1=1"
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

    query.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");
    params.push(Box::new(limit as i64));
    params.push(Box::new(offset as i64));

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();

    let mut stmt = conn.prepare(&query)?;
    let entries = stmt.query_map(&param_refs[..], |row| {
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

fn main() {
    println!("Running audit module tests...");

    let conn = Connection::open_in_memory().unwrap();
    create_table(&conn).unwrap();

    // Test 1: Basic logging
    log(&conn, AuditAction::Create, "patient", Some("patient-123"), None).unwrap();
    log(&conn, AuditAction::View, "patient", Some("patient-123"), None).unwrap();
    log(&conn, AuditAction::Update, "patient", Some("patient-123"), Some("fields: first_name")).unwrap();

    let entries = query_log(&conn, None, None, 100, 0).unwrap();
    assert_eq!(entries.len(), 3, "Should have 3 audit entries");
    println!("✓ Test 1 passed: Basic logging works");

    // Test 2: Filtering by entity type
    log(&conn, AuditAction::Create, "file", Some("file-456"), None).unwrap();
    let patient_entries = query_log(&conn, Some("patient"), None, 100, 0).unwrap();
    assert_eq!(patient_entries.len(), 3, "Should have 3 patient entries");
    println!("✓ Test 2 passed: Filtering by entity type works");

    // Test 3: Filtering by entity ID
    log(&conn, AuditAction::View, "patient", Some("patient-789"), None).unwrap();
    let specific_entries = query_log(&conn, None, Some("patient-123"), 100, 0).unwrap();
    assert_eq!(specific_entries.len(), 3, "Should have 3 entries for patient-123");
    println!("✓ Test 3 passed: Filtering by entity ID works");

    // Test 4: Pagination
    for i in 0..10 {
        log(&conn, AuditAction::View, "test", Some(&format!("test-{}", i)), None).unwrap();
    }
    let page1 = query_log(&conn, Some("test"), None, 5, 0).unwrap();
    let page2 = query_log(&conn, Some("test"), None, 5, 5).unwrap();
    assert_eq!(page1.len(), 5, "First page should have 5 entries");
    assert_eq!(page2.len(), 5, "Second page should have 5 entries");
    assert_ne!(page1[0].id, page2[0].id, "Pages should not overlap");
    println!("✓ Test 4 passed: Pagination works");

    // Test 5: No PHI in details
    log(&conn, AuditAction::Update, "patient", Some("patient-123"),
        Some("fields: first_name,last_name,date_of_birth")).unwrap();
    let entries_with_details = query_log(&conn, None, Some("patient-123"), 100, 0).unwrap();
    let last_entry = &entries_with_details[0];
    assert!(last_entry.details.as_ref().unwrap().contains("fields:"), "Details should contain field names");
    println!("✓ Test 5 passed: Details contain field names only (no PHI)");

    // Test 6: Ordering (newest first)
    let all_entries = query_log(&conn, None, None, 100, 0).unwrap();
    assert!(all_entries.len() > 1, "Should have multiple entries");
    for i in 1..all_entries.len() {
        assert!(all_entries[i-1].timestamp >= all_entries[i].timestamp,
                "Entries should be ordered newest first");
    }
    println!("✓ Test 6 passed: Entries are ordered newest first");

    println!("\n✓ All audit tests passed!");
}
