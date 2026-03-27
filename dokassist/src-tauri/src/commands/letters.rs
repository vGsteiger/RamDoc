use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::models::letter::{self, CreateLetter, Letter, UpdateLetter};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn create_letter(
    state: State<'_, AppState>,
    input: CreateLetter,
) -> Result<Letter, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let letter = letter::create_letter(&tx, input)?;

    // Audit logging
    audit::log(&tx, AuditAction::Create, "letter", Some(&letter.id), None)?;

    tx.commit()?;

    Ok(letter)
}

#[tauri::command]
pub async fn get_letter(state: State<'_, AppState>, id: String) -> Result<Letter, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let letter = letter::get_letter(&conn, &id)?;

    // Audit logging
    audit::log(&conn, AuditAction::View, "letter", Some(&id), None)?;

    Ok(letter)
}

#[tauri::command]
pub async fn list_letters(
    state: State<'_, AppState>,
    patient_id: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Letter>, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let letters = letter::list_letters_for_patient(&conn, &patient_id, limit, offset)?;

    // Audit logging for list access
    audit::log(
        &conn,
        AuditAction::View,
        "letter",
        None,
        Some(&format!("list letters for patient {}", patient_id)),
    )?;

    Ok(letters)
}

#[tauri::command]
pub async fn update_letter(
    state: State<'_, AppState>,
    id: String,
    input: UpdateLetter,
) -> Result<Letter, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let letter = letter::update_letter(&tx, &id, input)?;

    // Audit logging
    audit::log(&tx, AuditAction::Update, "letter", Some(&id), None)?;

    tx.commit()?;

    Ok(letter)
}

#[tauri::command]
pub async fn delete_letter(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    letter::delete_letter(&tx, &id)?;

    // Audit logging
    audit::log(&tx, AuditAction::Delete, "letter", Some(&id), None)?;

    tx.commit()?;

    Ok(())
}

#[tauri::command]
pub async fn mark_letter_as_finalized(
    state: State<'_, AppState>,
    id: String,
) -> Result<Letter, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let letter = letter::mark_letter_as_finalized(&tx, &id)?;

    // Audit logging
    audit::log(
        &tx,
        AuditAction::Update,
        "letter",
        Some(&id),
        Some("marked as finalized"),
    )?;

    tx.commit()?;

    Ok(letter)
}

#[tauri::command]
pub async fn mark_letter_as_sent(
    state: State<'_, AppState>,
    id: String,
) -> Result<Letter, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let letter = letter::mark_letter_as_sent(&tx, &id)?;

    // Audit logging
    audit::log(
        &tx,
        AuditAction::Update,
        "letter",
        Some(&id),
        Some("marked as sent"),
    )?;

    tx.commit()?;

    Ok(letter)
}
