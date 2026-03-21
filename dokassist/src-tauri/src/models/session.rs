use crate::error::AppError;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub patient_id: String,
    pub session_date: String,
    pub session_type: String,
    pub duration_minutes: Option<i32>,
    pub scheduled_time: Option<String>,
    pub notes: Option<String>,
    pub amdp_data: Option<String>,
    pub clinical_summary: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSession {
    pub patient_id: String,
    pub session_date: String,
    pub session_type: String,
    pub duration_minutes: Option<i32>,
    pub scheduled_time: Option<String>,
    pub notes: Option<String>,
    pub amdp_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSession {
    pub session_date: Option<String>,
    pub session_type: Option<String>,
    pub duration_minutes: Option<i32>,
    pub scheduled_time: Option<String>,
    pub notes: Option<String>,
    pub amdp_data: Option<String>,
    pub clinical_summary: Option<String>,
}

fn row_to_session(row: &Row) -> Result<Session, rusqlite::Error> {
    Ok(Session {
        id: row.get(0)?,
        patient_id: row.get(1)?,
        session_date: row.get(2)?,
        session_type: row.get(3)?,
        duration_minutes: row.get(4)?,
        scheduled_time: row.get(5)?,
        notes: row.get(6)?,
        amdp_data: row.get(7)?,
        clinical_summary: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub fn create_session(conn: &Connection, input: CreateSession) -> Result<Session, AppError> {
    let id = Uuid::now_v7().to_string();

    conn.execute(
        "INSERT INTO sessions (id, patient_id, session_date, session_type, duration_minutes, scheduled_time, notes, amdp_data)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            id,
            input.patient_id,
            input.session_date,
            input.session_type,
            input.duration_minutes,
            input.scheduled_time,
            input.notes,
            input.amdp_data,
        ],
    )?;

    get_session(conn, &id)
}

pub fn get_session(conn: &Connection, id: &str) -> Result<Session, AppError> {
    let session = conn
        .query_row(
            "SELECT id, patient_id, session_date, session_type, duration_minutes, scheduled_time, notes, amdp_data,
                    clinical_summary, created_at, updated_at
             FROM sessions WHERE id = ?",
            params![id],
            row_to_session,
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound(format!("Session not found: {}", id))
            }
            other => AppError::from(other),
        })?;

    Ok(session)
}

pub fn update_session(
    conn: &Connection,
    id: &str,
    input: UpdateSession,
) -> Result<Session, AppError> {
    get_session(conn, id)?;

    let mut updates = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(session_date) = input.session_date {
        updates.push("session_date = ?");
        values.push(Box::new(session_date));
    }
    if let Some(session_type) = input.session_type {
        updates.push("session_type = ?");
        values.push(Box::new(session_type));
    }
    if let Some(duration_minutes) = input.duration_minutes {
        updates.push("duration_minutes = ?");
        values.push(Box::new(duration_minutes));
    }
    if let Some(scheduled_time) = input.scheduled_time {
        updates.push("scheduled_time = ?");
        values.push(Box::new(scheduled_time));
    }
    if let Some(notes) = input.notes {
        updates.push("notes = ?");
        values.push(Box::new(notes));
    }
    if let Some(amdp_data) = input.amdp_data {
        updates.push("amdp_data = ?");
        values.push(Box::new(amdp_data));
    }
    if let Some(clinical_summary) = input.clinical_summary {
        updates.push("clinical_summary = ?");
        values.push(Box::new(clinical_summary));
    }

    if updates.is_empty() {
        return get_session(conn, id);
    }

    let query = format!("UPDATE sessions SET {} WHERE id = ?", updates.join(", "));
    values.push(Box::new(id.to_string()));

    let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&query, params.as_slice())?;

    get_session(conn, id)
}

pub fn delete_session(conn: &Connection, id: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM sessions WHERE id = ?", params![id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("Session not found: {}", id)));
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWithPatient {
    pub session: Session,
    pub patient_name: String,
}

pub fn list_all_sessions(
    conn: &Connection,
    limit: u32,
    offset: u32,
) -> Result<Vec<SessionWithPatient>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.patient_id, s.session_date, s.session_type, s.duration_minutes,
                s.scheduled_time, s.notes, s.amdp_data, s.clinical_summary, s.created_at, s.updated_at,
                p.first_name || ' ' || p.last_name AS patient_name
         FROM sessions s
         JOIN patients p ON s.patient_id = p.id
         ORDER BY s.session_date DESC
         LIMIT ? OFFSET ?",
    )?;

    let items = stmt
        .query_map(params![limit, offset], |row| {
            Ok(SessionWithPatient {
                session: row_to_session(row)?,
                patient_name: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

pub fn list_sessions_for_patient(
    conn: &Connection,
    patient_id: &str,
    limit: u32,
    offset: u32,
) -> Result<Vec<Session>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, patient_id, session_date, session_type, duration_minutes, scheduled_time, notes, amdp_data,
                clinical_summary, created_at, updated_at
         FROM sessions
         WHERE patient_id = ?
         ORDER BY session_date DESC
         LIMIT ? OFFSET ?",
    )?;

    let sessions = stmt
        .query_map(params![patient_id, limit, offset], row_to_session)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(sessions)
}
