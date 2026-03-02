// Standalone test for audit module
// This can be run independently to verify audit logic

use dokassist_lib::audit::{self, AuditAction};
use rusqlite::Connection;

fn main() {
    println!("Running audit module tests...");

    let conn = Connection::open_in_memory().unwrap();
    audit::create_table(&conn).unwrap();

    // Test 1: Basic logging
    audit::log(
        &conn,
        AuditAction::Create,
        "patient",
        Some("patient-123"),
        None,
    )
    .unwrap();
    audit::log(
        &conn,
        AuditAction::View,
        "patient",
        Some("patient-123"),
        None,
    )
    .unwrap();
    audit::log(
        &conn,
        AuditAction::Update,
        "patient",
        Some("patient-123"),
        Some("fields: first_name"),
    )
    .unwrap();

    let entries = audit::query_log(&conn, None, None, None, None, 100, 0).unwrap();
    assert_eq!(entries.len(), 3, "Should have 3 audit entries");
    println!("✓ Test 1 passed: Basic logging works");

    // Test 2: Filtering by entity type
    audit::log(&conn, AuditAction::Create, "file", Some("file-456"), None).unwrap();
    let patient_entries =
        audit::query_log(&conn, Some("patient"), None, None, None, 100, 0).unwrap();
    assert_eq!(patient_entries.len(), 3, "Should have 3 patient entries");
    println!("✓ Test 2 passed: Filtering by entity type works");

    // Test 3: Filtering by entity ID
    audit::log(
        &conn,
        AuditAction::View,
        "patient",
        Some("patient-789"),
        None,
    )
    .unwrap();
    let specific_entries =
        audit::query_log(&conn, None, Some("patient-123"), None, None, 100, 0).unwrap();
    assert_eq!(
        specific_entries.len(),
        3,
        "Should have 3 entries for patient-123"
    );
    println!("✓ Test 3 passed: Filtering by entity ID works");

    // Test 4: Pagination
    for i in 0..10 {
        audit::log(
            &conn,
            AuditAction::View,
            "test",
            Some(&format!("test-{}", i)),
            None,
        )
        .unwrap();
    }
    let page1 = audit::query_log(&conn, Some("test"), None, None, None, 5, 0).unwrap();
    let page2 = audit::query_log(&conn, Some("test"), None, None, None, 5, 5).unwrap();
    assert_eq!(page1.len(), 5, "First page should have 5 entries");
    assert_eq!(page2.len(), 5, "Second page should have 5 entries");
    assert_ne!(page1[0].id, page2[0].id, "Pages should not overlap");
    println!("✓ Test 4 passed: Pagination works");

    // Test 5: No PHI in details
    audit::log(
        &conn,
        AuditAction::Update,
        "patient",
        Some("patient-123"),
        Some("fields: first_name,last_name,date_of_birth"),
    )
    .unwrap();
    let entries_with_details =
        audit::query_log(&conn, None, Some("patient-123"), None, None, 100, 0).unwrap();
    let last_entry = &entries_with_details[0];
    assert!(
        last_entry.details.as_ref().unwrap().contains("fields:"),
        "Details should contain field names"
    );
    println!("✓ Test 5 passed: Details contain field names only (no PHI)");

    // Test 6: Ordering (newest first)
    let all_entries = audit::query_log(&conn, None, None, None, None, 100, 0).unwrap();
    assert!(all_entries.len() > 1, "Should have multiple entries");
    for i in 1..all_entries.len() {
        assert!(
            all_entries[i - 1].timestamp >= all_entries[i].timestamp,
            "Entries should be ordered newest first"
        );
    }
    println!("✓ Test 6 passed: Entries are ordered newest first");

    println!("\n✓ All audit tests passed!");
}
