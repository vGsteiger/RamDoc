use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::models::email::{self, CreateEmail, Email, UpdateEmail};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn create_email(
    state: State<'_, AppState>,
    input: CreateEmail,
) -> Result<Email, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let email = email::create_email(&tx, input)?;

    // Audit logging
    audit::log(&tx, AuditAction::Create, "email", Some(&email.id), None)?;

    tx.commit()?;

    Ok(email)
}

#[tauri::command]
pub async fn get_email(state: State<'_, AppState>, id: String) -> Result<Email, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let email = email::get_email(&conn, &id)?;

    // Audit logging
    audit::log(&conn, AuditAction::View, "email", Some(&id), None)?;

    Ok(email)
}

#[tauri::command]
pub async fn list_emails(
    state: State<'_, AppState>,
    patient_id: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Email>, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let emails = email::list_emails_for_patient(&conn, &patient_id, limit, offset)?;

    // Audit logging for list access
    audit::log(
        &conn,
        AuditAction::View,
        "email",
        None,
        Some(&format!("list emails for patient {}", patient_id)),
    )?;

    Ok(emails)
}

#[tauri::command]
pub async fn update_email(
    state: State<'_, AppState>,
    id: String,
    input: UpdateEmail,
) -> Result<Email, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let email = email::update_email(&tx, &id, input)?;

    // Audit logging
    audit::log(&tx, AuditAction::Update, "email", Some(&id), None)?;

    tx.commit()?;

    Ok(email)
}

#[tauri::command]
pub async fn delete_email(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    email::delete_email(&tx, &id)?;

    // Audit logging
    audit::log(&tx, AuditAction::Delete, "email", Some(&id), None)?;

    tx.commit()?;

    Ok(())
}

#[tauri::command]
pub async fn mark_email_as_sent(state: State<'_, AppState>, id: String) -> Result<Email, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let email = email::mark_email_as_sent(&tx, &id)?;

    // Audit logging
    audit::log(
        &tx,
        AuditAction::Update,
        "email",
        Some(&id),
        Some("marked as sent"),
    )?;

    tx.commit()?;

    Ok(email)
}
