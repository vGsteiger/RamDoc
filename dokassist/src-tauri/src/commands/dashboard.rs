use crate::error::AppError;
use crate::models::patient::Patient;
use crate::models::session::SessionWithPatient;
use crate::state::AppState;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub todays_sessions: Vec<SessionWithPatient>,
    pub recent_patients: Vec<Patient>,
    pub sessions_with_incomplete_notes: Vec<SessionWithPatient>,
}

fn get_todays_sessions(conn: &Connection) -> Result<Vec<SessionWithPatient>, AppError> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let mut stmt = conn.prepare(
        "SELECT s.id, s.patient_id, s.session_date, s.session_type, s.duration_minutes,
                s.notes, s.amdp_data, s.created_at, s.updated_at,
                p.first_name || ' ' || p.last_name AS patient_name
         FROM sessions s
         JOIN patients p ON s.patient_id = p.id
         WHERE s.session_date >= ? AND s.session_date < date(?, '+1 day')
         ORDER BY s.session_date ASC",
    )?;

    let items = stmt
        .query_map(params![today, today], |row| {
            Ok(SessionWithPatient {
                session: crate::models::session::Session {
                    id: row.get(0)?,
                    patient_id: row.get(1)?,
                    session_date: row.get(2)?,
                    session_type: row.get(3)?,
                    duration_minutes: row.get(4)?,
                    notes: row.get(5)?,
                    amdp_data: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                },
                patient_name: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

fn get_recent_patients(conn: &Connection, limit: u32) -> Result<Vec<Patient>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.ahv_number, p.first_name, p.last_name, p.date_of_birth,
                p.gender, p.address, p.phone, p.email, p.insurance,
                p.gp_name, p.gp_address, p.notes, p.created_at, p.updated_at
         FROM patients p
         JOIN (
             SELECT patient_id, MAX(created_at) AS last_session_created_at
             FROM sessions
             GROUP BY patient_id
         ) s ON p.id = s.patient_id
         ORDER BY s.last_session_created_at DESC
         LIMIT ?",
    )?;

    let patients = stmt
        .query_map(params![limit], |row| {
            Ok(Patient {
                id: row.get(0)?,
                ahv_number: row.get(1)?,
                first_name: row.get(2)?,
                last_name: row.get(3)?,
                date_of_birth: row.get(4)?,
                gender: row.get(5)?,
                address: row.get(6)?,
                phone: row.get(7)?,
                email: row.get(8)?,
                insurance: row.get(9)?,
                gp_name: row.get(10)?,
                gp_address: row.get(11)?,
                notes: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(patients)
}

fn get_sessions_with_incomplete_notes(
    conn: &Connection,
    limit: u32,
) -> Result<Vec<SessionWithPatient>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.patient_id, s.session_date, s.session_type, s.duration_minutes,
                s.notes, s.amdp_data, s.created_at, s.updated_at,
                p.first_name || ' ' || p.last_name AS patient_name
         FROM sessions s
         JOIN patients p ON s.patient_id = p.id
         WHERE (s.notes IS NULL OR s.notes = '')
         ORDER BY s.session_date DESC
         LIMIT ?",
    )?;

    let items = stmt
        .query_map(params![limit], |row| {
            Ok(SessionWithPatient {
                session: crate::models::session::Session {
                    id: row.get(0)?,
                    patient_id: row.get(1)?,
                    session_date: row.get(2)?,
                    session_type: row.get(3)?,
                    duration_minutes: row.get(4)?,
                    notes: row.get(5)?,
                    amdp_data: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                },
                patient_name: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(items)
}

#[tauri::command]
pub async fn get_dashboard_data(state: State<'_, AppState>) -> Result<DashboardData, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let todays_sessions = get_todays_sessions(&conn)?;
    let recent_patients = get_recent_patients(&conn, 5)?;
    let sessions_with_incomplete_notes = get_sessions_with_incomplete_notes(&conn, 10)?;

    Ok(DashboardData {
        todays_sessions,
        recent_patients,
        sessions_with_incomplete_notes,
    })
}
