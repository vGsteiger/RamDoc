use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::models::medication::{self, CreateMedication, Medication, UpdateMedication};
use crate::search;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn create_medication(
    state: State<'_, AppState>,
    input: CreateMedication,
) -> Result<Medication, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let medication = medication::create_medication(&tx, input)?;

    audit::log(
        &tx,
        AuditAction::Create,
        "medication",
        Some(&medication.id),
        None,
    )?;

    tx.commit()?;

    search::index_medication_from_model(&conn, &medication)?;

    Ok(medication)
}

#[tauri::command]
pub async fn get_medication(
    state: State<'_, AppState>,
    id: String,
) -> Result<Medication, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let medication = medication::get_medication(&conn, &id)?;

    audit::log(&conn, AuditAction::View, "medication", Some(&id), None)?;

    Ok(medication)
}

#[tauri::command]
pub async fn list_medications_for_patient(
    state: State<'_, AppState>,
    patient_id: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Medication>, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let medications = medication::list_medications_for_patient(&conn, &patient_id, limit, offset)?;

    audit::log(
        &conn,
        AuditAction::View,
        "medication",
        None,
        Some(&format!(
            "list: {} medications for patient {}",
            medications.len(),
            patient_id
        )),
    )?;

    Ok(medications)
}

#[tauri::command]
pub async fn update_medication(
    state: State<'_, AppState>,
    id: String,
    input: UpdateMedication,
) -> Result<Medication, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let medication = medication::update_medication(&tx, &id, input)?;

    audit::log(&tx, AuditAction::Update, "medication", Some(&id), None)?;

    tx.commit()?;

    search::index_medication_from_model(&conn, &medication)?;

    Ok(medication)
}

#[tauri::command]
pub async fn delete_medication(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    search::remove_from_index(&tx, "medication", &id)?;

    medication::delete_medication(&tx, &id)?;

    audit::log(&tx, AuditAction::Delete, "medication", Some(&id), None)?;

    tx.commit()?;

    Ok(())
}
