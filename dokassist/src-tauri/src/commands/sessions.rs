use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::models::session::{self, CreateSession, Session, UpdateSession};
use crate::search;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn create_session(
    state: State<'_, AppState>,
    input: CreateSession,
) -> Result<Session, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let session = session::create_session(&tx, input)?;

    audit::log(&tx, AuditAction::Create, "session", Some(&session.id), None)?;

    tx.commit()?;

    search::index_session_from_model(&conn, &session)?;

    Ok(session)
}

#[tauri::command]
pub async fn get_session(state: State<'_, AppState>, id: String) -> Result<Session, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let session = session::get_session(&conn, &id)?;

    audit::log(&conn, AuditAction::View, "session", Some(&id), None)?;

    Ok(session)
}

#[tauri::command]
pub async fn list_sessions_for_patient(
    state: State<'_, AppState>,
    patient_id: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Session>, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let sessions = session::list_sessions_for_patient(&conn, &patient_id, limit, offset)?;

    audit::log(
        &conn,
        AuditAction::View,
        "session",
        None,
        Some(&format!(
            "list: {} sessions for patient {}",
            sessions.len(),
            patient_id
        )),
    )?;

    Ok(sessions)
}

#[tauri::command]
pub async fn update_session(
    state: State<'_, AppState>,
    id: String,
    input: UpdateSession,
) -> Result<Session, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let session = session::update_session(&tx, &id, input)?;

    audit::log(&tx, AuditAction::Update, "session", Some(&id), None)?;

    tx.commit()?;

    search::index_session_from_model(&conn, &session)?;

    Ok(session)
}

#[tauri::command]
pub async fn delete_session(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    search::remove_from_index(&tx, "session", &id)?;

    session::delete_session(&tx, &id)?;

    audit::log(&tx, AuditAction::Delete, "session", Some(&id), None)?;

    tx.commit()?;

    Ok(())
}
