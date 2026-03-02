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
