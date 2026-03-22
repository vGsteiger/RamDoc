use crate::error::AppError;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub patient_id: String,
    pub report_type: String,
    pub content: String,
    pub generated_at: String,
    pub model_name: Option<String>,
    pub prompt_hash: Option<String>,
    pub session_ids: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReport {
    pub patient_id: String,
    pub report_type: String,
    pub content: String,
    pub model_name: Option<String>,
    pub prompt_hash: Option<String>,
    pub session_ids: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReport {
    pub report_type: Option<String>,
    pub content: Option<String>,
    pub model_name: Option<String>,
    pub prompt_hash: Option<String>,
    pub session_ids: Option<String>,
}

fn row_to_report(row: &Row) -> Result<Report, rusqlite::Error> {
    Ok(Report {
        id: row.get(0)?,
        patient_id: row.get(1)?,
        report_type: row.get(2)?,
        content: row.get(3)?,
        generated_at: row.get(4)?,
        model_name: row.get(5)?,
        prompt_hash: row.get(6)?,
        session_ids: row.get(7)?,
        created_at: row.get(8)?,
    })
}

pub fn create_report(conn: &Connection, input: CreateReport) -> Result<Report, AppError> {
    let id = Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO reports (id, patient_id, report_type, content, model_name, prompt_hash, session_ids)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            id,
            input.patient_id,
            input.report_type,
            input.content,
            input.model_name,
            input.prompt_hash,
            input.session_ids,
        ],
    )?;

    get_report(conn, &id)
}

pub fn get_report(conn: &Connection, id: &str) -> Result<Report, AppError> {
    let report = conn
        .query_row(
            "SELECT id, patient_id, report_type, content, generated_at, model_name, prompt_hash, session_ids, created_at
             FROM reports WHERE id = ?",
            params![id],
            row_to_report,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Report not found: {}", id))
            }
            other => AppError::from(other),
        })?;

    Ok(report)
}

pub fn update_report(conn: &Connection, id: &str, input: UpdateReport) -> Result<Report, AppError> {
    get_report(conn, id)?;

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(report_type) = input.report_type {
        updates.push("report_type = ?");
        values.push(Box::new(report_type));
    }
    if let Some(content) = input.content {
        updates.push("content = ?");
        values.push(Box::new(content));
    }
    if let Some(model_name) = input.model_name {
        updates.push("model_name = ?");
        values.push(Box::new(model_name));
    }
    if let Some(prompt_hash) = input.prompt_hash {
        updates.push("prompt_hash = ?");
        values.push(Box::new(prompt_hash));
    }
    if let Some(session_ids) = input.session_ids {
        updates.push("session_ids = ?");
        values.push(Box::new(session_ids));
    }

    if updates.is_empty() {
        return get_report(conn, id);
    }

    let query = format!("UPDATE reports SET {} WHERE id = ?", updates.join(", "));
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice())?;

    get_report(conn, id)
}

pub fn delete_report(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM reports WHERE id = ?", params![id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("Report not found: {}", id)));
    }

    Ok(())
}

pub fn list_reports_for_patient(
    conn: &Connection,
    patient_id: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<Report>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, patient_id, report_type, content, generated_at, model_name, prompt_hash, session_ids, created_at
         FROM reports
         WHERE patient_id = ?
         ORDER BY generated_at DESC
         LIMIT ? OFFSET ?",
    )?;

    let reports = stmt
        .query_map(params![patient_id, limit, offset], row_to_report)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(reports)
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

    fn make_report(conn: &Connection, report_type: &str, content: &str) -> Report {
        create_report(
            conn,
            CreateReport {
                patient_id: "p1".into(),
                report_type: report_type.into(),
                content: content.into(),
                model_name: None,
                prompt_hash: None,
                session_ids: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn test_create_and_get_report() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let r = make_report(&conn, "psychiatric", "Patient is stable.");
        assert_eq!(r.report_type, "psychiatric");
        assert_eq!(r.content, "Patient is stable.");
        assert!(!r.generated_at.is_empty());
        let r2 = get_report(&conn, &r.id).unwrap();
        assert_eq!(r.id, r2.id);
    }

    #[test]
    fn test_update_report_content() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let r = make_report(&conn, "referral", "Original content.");
        let updated = update_report(
            &conn,
            &r.id,
            UpdateReport {
                report_type: None,
                content: Some("Updated content.".into()),
                model_name: None,
                prompt_hash: None,
                session_ids: None,
            },
        )
        .unwrap();
        assert_eq!(updated.content, "Updated content.");
        assert_eq!(updated.report_type, "referral");
    }

    #[test]
    fn test_delete_report() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let r = make_report(&conn, "discharge", "Discharged.");
        delete_report(&conn, &r.id).unwrap();
        assert!(matches!(
            get_report(&conn, &r.id),
            Err(AppError::NotFound(_))
        ));
    }

    #[test]
    fn test_list_reports_for_patient() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        make_report(&conn, "type_a", "Content A");
        make_report(&conn, "type_b", "Content B");
        make_report(&conn, "type_c", "Content C");
        let list = list_reports_for_patient(&conn, "p1", 10, 0).unwrap();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_update_report_no_fields() {
        let (_dir, pool) = open_test_db();
        let conn = pool.conn().unwrap();
        insert_patient(&conn);
        let r = make_report(&conn, "progress", "No change.");
        let unchanged = update_report(
            &conn,
            &r.id,
            UpdateReport {
                report_type: None,
                content: None,
                model_name: None,
                prompt_hash: None,
                session_ids: None,
            },
        )
        .unwrap();
        assert_eq!(unchanged.content, "No change.");
        assert_eq!(unchanged.report_type, "progress");
    }
}
