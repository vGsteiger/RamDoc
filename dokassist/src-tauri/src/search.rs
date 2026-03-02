use crate::error::AppError;
use crate::models::diagnosis::Diagnosis;
use crate::models::medication::Medication;
use crate::models::patient::Patient;
use crate::models::session::Session;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub result_type: String,
    pub entity_id: String,
    pub patient_id: String,
    pub patient_name: String,
    pub title: String,
    pub snippet: String,
    pub date: Option<String>,
    pub rank: f64,
}

/// Escape a query string for use as a literal FTS5 phrase match.
///
/// MED-1: FTS5 treats operators like `*`, `OR`, `AND`, `NOT`, `"`, and `^`
/// as metacharacters inside a MATCH clause even when the value is bound
/// via a `?` parameter.  Wrapping the input in double-quotes and escaping
/// any internal double-quotes makes the whole string a phrase query,
/// preventing wildcard/operator injection.
fn sanitize_fts5_query(input: &str) -> String {
    // Replace every " with "" (FTS5 phrase-literal escaping)
    let escaped = input.replace('"', "\"\"");
    // Wrap in double-quotes to force a phrase match
    format!("\"{}\"", escaped)
}

/// Full-text search across all indexed content
pub fn search(conn: &Connection, query: &str, limit: u32) -> Result<Vec<SearchResult>, AppError> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    // Normalize AHV numbers in query (remove dots for searching)
    let normalized_query = normalize_ahv_for_search(query);
    // MED-1: Escape FTS5 metacharacters to prevent operator injection / DoS
    let normalized_query = sanitize_fts5_query(&normalized_query);

    // FTS5 search with ranking and snippet generation
    let mut stmt = conn.prepare(
        r#"
        SELECT
            entity_type,
            entity_id,
            patient_id,
            patient_name,
            title,
            snippet(search_index, 5, '<mark>', '</mark>', '...', 64) as snippet,
            date,
            bm25(search_index) as rank
        FROM search_index
        WHERE search_index MATCH ?1
        ORDER BY rank
        LIMIT ?2
        "#,
    )?;

    let results = stmt
        .query_map([&normalized_query, &limit.to_string()], |row| {
            Ok(SearchResult {
                result_type: row.get(0)?,
                entity_id: row.get(1)?,
                patient_id: row.get(2)?,
                patient_name: row.get(3)?,
                title: row.get(4)?,
                snippet: row.get(5)?,
                date: row.get(6)?,
                rank: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

/// Index or re-index a patient's searchable fields
pub fn index_patient(conn: &Connection, patient: &Patient) -> Result<(), AppError> {
    // Remove existing patient index entry
    remove_from_index(conn, "patient", &patient.id)?;

    let patient_name = format!("{} {}", patient.first_name, patient.last_name);

    // Prepare content for indexing - include all searchable fields
    let mut content_parts = vec![
        patient.first_name.clone(),
        patient.last_name.clone(),
        patient.ahv_number.clone(),
        patient.ahv_number.replace(".", ""), // Also index plain format
    ];

    // Index both dotted and plain AHV formats
    content_parts.push(patient.ahv_number.clone());
    content_parts.push(patient.ahv_number.replace(".", ""));

    if let Some(ref email) = patient.email {
        content_parts.push(email.clone());
    }

    if let Some(ref phone) = patient.phone {
        content_parts.push(phone.clone());
    }

    let content = content_parts.join(" ");

    conn.execute(
        r#"
        INSERT INTO search_index (entity_type, entity_id, patient_id, patient_name, title, content, date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        (
            "patient",
            &patient.id,
            &patient.id,
            &patient_name,
            &patient_name,
            &content,
            &patient.date_of_birth,
        ),
    )?;

    Ok(())
}

/// Index file content (called after LLM metadata extraction)
#[allow(clippy::too_many_arguments)]
pub fn index_file(
    conn: &Connection,
    file_id: &str,
    patient_id: &str,
    patient_name: &str,
    filename: &str,
    extracted_text: &str,
    document_type: Option<&str>,
    date: Option<&str>,
) -> Result<(), AppError> {
    // Remove existing file index entry
    remove_from_index(conn, "file", file_id)?;

    let title = document_type
        .map(|dt| format!("{} - {}", dt, filename))
        .unwrap_or_else(|| filename.to_string());

    conn.execute(
        r#"
        INSERT INTO search_index (entity_type, entity_id, patient_id, patient_name, title, content, date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        (
            "file",
            file_id,
            patient_id,
            patient_name,
            &title,
            extracted_text,
            date,
        ),
    )?;

    Ok(())
}

/// Index session notes
pub fn index_session(
    conn: &Connection,
    session_id: &str,
    patient_id: &str,
    patient_name: &str,
    session_type: &str,
    notes: &str,
    session_date: &str,
) -> Result<(), AppError> {
    // Remove existing session index entry
    remove_from_index(conn, "session", session_id)?;

    let title = format!("{} - {}", session_type, session_date);

    conn.execute(
        r#"
        INSERT INTO search_index (entity_type, entity_id, patient_id, patient_name, title, content, date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        (
            "session",
            session_id,
            patient_id,
            patient_name,
            &title,
            notes,
            session_date,
        ),
    )?;

    Ok(())
}

/// Index finalized report content
#[allow(clippy::too_many_arguments)]
pub fn index_report(
    conn: &Connection,
    report_id: &str,
    patient_id: &str,
    patient_name: &str,
    report_type: &str,
    title: &str,
    content: &str,
    generated_at: &str,
) -> Result<(), AppError> {
    // Remove existing report index entry
    remove_from_index(conn, "report", report_id)?;

    let full_title = format!("{} - {}", report_type, title);

    conn.execute(
        r#"
        INSERT INTO search_index (entity_type, entity_id, patient_id, patient_name, title, content, date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        (
            "report",
            report_id,
            patient_id,
            patient_name,
            &full_title,
            content,
            generated_at,
        ),
    )?;

    Ok(())
}

/// Remove all index entries for an entity
pub fn remove_from_index(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM search_index WHERE entity_type = ?1 AND entity_id = ?2",
        (entity_type, entity_id),
    )?;

    Ok(())
}

// Wrapper functions that take model structs directly

pub fn index_session_from_model(conn: &Connection, session: &Session) -> Result<(), AppError> {
    // Get patient name from database
    let patient_name: String = conn
        .query_row(
            "SELECT first_name || ' ' || last_name FROM patients WHERE id = ?",
            [&session.patient_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "Unknown Patient".to_string());

    index_session(
        conn,
        &session.id,
        &session.patient_id,
        &patient_name,
        &session.session_type,
        session.notes.as_deref().unwrap_or(""),
        &session.session_date,
    )
}

pub fn index_diagnosis_from_model(
    conn: &Connection,
    diagnosis: &Diagnosis,
) -> Result<(), AppError> {
    // Get patient name from database
    let patient_name: String = conn
        .query_row(
            "SELECT first_name || ' ' || last_name FROM patients WHERE id = ?",
            [&diagnosis.patient_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "Unknown Patient".to_string());

    // Remove existing diagnosis index entry
    remove_from_index(conn, "diagnosis", &diagnosis.id)?;

    let title = format!("{} - {}", diagnosis.icd10_code, diagnosis.description);
    let content = format!(
        "{} {} {}",
        diagnosis.description,
        diagnosis.icd10_code,
        diagnosis.notes.as_deref().unwrap_or("")
    );

    conn.execute(
        r#"
        INSERT INTO search_index (entity_type, entity_id, patient_id, patient_name, title, content, date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        (
            "diagnosis",
            &diagnosis.id,
            &diagnosis.patient_id,
            &patient_name,
            &title,
            &content,
            &diagnosis.diagnosed_date,
        ),
    )?;

    Ok(())
}

pub fn index_medication_from_model(
    conn: &Connection,
    medication: &Medication,
) -> Result<(), AppError> {
    // Get patient name from database
    let patient_name: String = conn
        .query_row(
            "SELECT first_name || ' ' || last_name FROM patients WHERE id = ?",
            [&medication.patient_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "Unknown Patient".to_string());

    // Remove existing medication index entry
    remove_from_index(conn, "medication", &medication.id)?;

    let title = format!("{} - {}", medication.substance, medication.dosage);
    let content = format!(
        "{} {} {} {}",
        medication.substance,
        medication.dosage,
        medication.frequency,
        medication.notes.as_deref().unwrap_or("")
    );

    conn.execute(
        r#"
        INSERT INTO search_index (entity_type, entity_id, patient_id, patient_name, title, content, date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        (
            "medication",
            &medication.id,
            &medication.patient_id,
            &patient_name,
            &title,
            &content,
            &medication.start_date,
        ),
    )?;

    Ok(())
}

/// Normalize AHV queries: "7561234567897" and "756.1234.5678.97" both match
fn normalize_ahv_for_search(query: &str) -> String {
    // Check if query looks like an AHV number
    let digits_only: String = query.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits_only.len() == 13 && digits_only.starts_with("756") {
        // Search using plain digits only — FTS5 treats dots as separators,
        // making "756.1234.5678.97" a syntax error as an unquoted query term.
        // The content is indexed with both dotted and plain formats, so
        // matching the plain digits is sufficient.
        digits_only
    } else {
        query.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{self, DbPool};
    use tempfile::TempDir;

    fn setup_test_db() -> (TempDir, DbPool) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let key = [42u8; 32];
        let pool = database::init_db(&db_path, &key).unwrap();
        (temp_dir, pool)
    }

    #[test]
    fn test_sanitize_fts5_query() {
        // Normal text becomes a phrase-quoted string
        assert_eq!(sanitize_fts5_query("John Doe"), "\"John Doe\"");

        // FTS5 wildcard is neutralised
        let escaped = sanitize_fts5_query("*");
        assert_eq!(escaped, "\"*\"");

        // Boolean operators are neutralised
        let escaped = sanitize_fts5_query("a OR * OR b");
        assert_eq!(escaped, "\"a OR * OR b\"");

        // Internal double-quotes are escaped
        let escaped = sanitize_fts5_query("say \"hello\"");
        assert_eq!(escaped, "\"say \"\"hello\"\"\"");
    }

    #[test]
    fn test_normalize_ahv() {
        let query1 = "756.1234.5678.97";
        let result1 = normalize_ahv_for_search(query1);
        assert!(result1.contains("7561234567897"));

        let query2 = "7561234567897";
        let result2 = normalize_ahv_for_search(query2);
        assert!(result2.contains("7561234567897"));

        let query3 = "John Doe";
        let result3 = normalize_ahv_for_search(query3);
        assert_eq!(result3, "John Doe");
    }

    #[test]
    fn test_index_and_search_patient() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        let patient = Patient {
            id: "patient-123".to_string(),
            first_name: "Max".to_string(),
            last_name: "Müller".to_string(),
            date_of_birth: "1980-01-01".to_string(),
            gender: Some("male".to_string()),
            ahv_number: "756.1234.5678.97".to_string(),
            email: None,
            phone: None,
            address: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        index_patient(&conn, &patient).unwrap();

        // Search by name
        let results = search(&conn, "Müller", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].result_type, "patient");

        // Search by AHV (dotted format)
        let results = search(&conn, "756.1234.5678.97", 10).unwrap();
        assert_eq!(results.len(), 1);

        // Search by AHV (plain format)
        let results = search(&conn, "7561234567897", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_empty_query() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();
        let results = search(&conn, "", 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_remove_from_index() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        let patient = Patient {
            id: "patient-456".to_string(),
            first_name: "Anna".to_string(),
            last_name: "Schmidt".to_string(),
            date_of_birth: "1990-05-15".to_string(),
            gender: Some("female".to_string()),
            ahv_number: "756.0000.0004.56".to_string(),
            email: None,
            phone: None,
            address: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        index_patient(&conn, &patient).unwrap();

        let results = search(&conn, "Schmidt", 10).unwrap();
        assert_eq!(results.len(), 1);

        remove_from_index(&conn, "patient", &patient.id).unwrap();

        let results = search(&conn, "Schmidt", 10).unwrap();
        assert_eq!(results.len(), 0);
    }
}
