use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::models::diagnosis::{self, CreateDiagnosis, Diagnosis, UpdateDiagnosis};
use crate::search;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn create_diagnosis(
    state: State<'_, AppState>,
    input: CreateDiagnosis,
) -> Result<Diagnosis, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let diagnosis = diagnosis::create_diagnosis(&tx, input)?;

    audit::log(
        &tx,
        AuditAction::Create,
        "diagnosis",
        Some(&diagnosis.id),
        None,
    )?;

    tx.commit()?;

    search::index_diagnosis_from_model(&conn, &diagnosis)?;

    Ok(diagnosis)
}

#[tauri::command]
pub async fn get_diagnosis(state: State<'_, AppState>, id: String) -> Result<Diagnosis, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let diagnosis = diagnosis::get_diagnosis(&conn, &id)?;

    audit::log(&conn, AuditAction::View, "diagnosis", Some(&id), None)?;

    Ok(diagnosis)
}

#[tauri::command]
pub async fn list_diagnoses_for_patient(
    state: State<'_, AppState>,
    patient_id: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Diagnosis>, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let diagnoses = diagnosis::list_diagnoses_for_patient(&conn, &patient_id, limit, offset)?;

    audit::log(
        &conn,
        AuditAction::View,
        "diagnosis",
        None,
        Some(&format!(
            "list: {} diagnoses for patient {}",
            diagnoses.len(),
            patient_id
        )),
    )?;

    Ok(diagnoses)
}

#[tauri::command]
pub async fn update_diagnosis(
    state: State<'_, AppState>,
    id: String,
    input: UpdateDiagnosis,
) -> Result<Diagnosis, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    let diagnosis = diagnosis::update_diagnosis(&tx, &id, input)?;

    audit::log(&tx, AuditAction::Update, "diagnosis", Some(&id), None)?;

    tx.commit()?;

    search::index_diagnosis_from_model(&conn, &diagnosis)?;

    Ok(diagnosis)
}

#[tauri::command]
pub async fn delete_diagnosis(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let tx = conn.unchecked_transaction()?;

    search::remove_from_index(&tx, "diagnosis", &id)?;

    diagnosis::delete_diagnosis(&tx, &id)?;

    audit::log(&tx, AuditAction::Delete, "diagnosis", Some(&id), None)?;

    tx.commit()?;

    Ok(())
}
