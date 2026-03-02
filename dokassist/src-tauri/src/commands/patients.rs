use tauri::State;
use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::state::AppState;
use crate::models::patient::{Patient, CreatePatient, UpdatePatient};

#[tauri::command]
pub async fn create_patient(
    state: State<'_, AppState>,
    input: CreatePatient,
) -> Result<Patient, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    // Begin transaction to ensure atomicity
    let tx = conn.unchecked_transaction()?;

    let patient = crate::models::patient::create_patient(&tx, input)?;

    // PKG-6: Audit logging (within same transaction)
    audit::log(&tx, AuditAction::Create, "patient", Some(&patient.id), None)?;

    tx.commit()?;

    Ok(patient)
}

#[tauri::command]
pub async fn get_patient(
    state: State<'_, AppState>,
    id: String,
) -> Result<Patient, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let patient = crate::models::patient::get_patient(&conn, &id)?;

    // PKG-6: Audit logging
    audit::log(&conn, AuditAction::View, "patient", Some(&id), None)?;

    Ok(patient)
}

#[tauri::command]
pub async fn list_patients(
    state: State<'_, AppState>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Patient>, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    let patients = crate::models::patient::list_patients(&conn, limit, offset)?;

    // PKG-6: Audit logging for list operations
    audit::log(&conn, AuditAction::View, "patient", None, Some(&format!("list: {} patients", patients.len())))?;

    Ok(patients)
}

#[tauri::command]
pub async fn update_patient(
    state: State<'_, AppState>,
    id: String,
    input: UpdatePatient,
) -> Result<Patient, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    // Begin transaction to ensure atomicity
    let tx = conn.unchecked_transaction()?;

    // Build details string with changed field names only (no PHI values)
    let mut changed_fields = Vec::new();
    if input.first_name.is_some() { changed_fields.push("first_name"); }
    if input.last_name.is_some() { changed_fields.push("last_name"); }
    if input.date_of_birth.is_some() { changed_fields.push("date_of_birth"); }
    if input.gender.is_some() { changed_fields.push("gender"); }
    if input.ahv_number.is_some() { changed_fields.push("ahv_number"); }
    if input.address.is_some() { changed_fields.push("address"); }
    if input.phone.is_some() { changed_fields.push("phone"); }
    if input.email.is_some() { changed_fields.push("email"); }
    if input.insurance.is_some() { changed_fields.push("insurance"); }
    if input.gp_name.is_some() { changed_fields.push("gp_name"); }
    if input.gp_address.is_some() { changed_fields.push("gp_address"); }
    if input.notes.is_some() { changed_fields.push("notes"); }

    let patient = crate::models::patient::update_patient(&tx, &id, input)?;

    // PKG-6: Audit logging with field tracking (within same transaction)
    let details = if !changed_fields.is_empty() {
        Some(format!("fields: {}", changed_fields.join(",")))
    } else {
        None
    };
    audit::log(&tx, AuditAction::Update, "patient", Some(&id), details.as_deref())?;

    tx.commit()?;

    Ok(patient)
}

#[tauri::command]
pub async fn delete_patient(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    // Begin transaction to ensure atomicity
    let tx = conn.unchecked_transaction()?;

    crate::models::patient::delete_patient(&tx, &id)?;

    // PKG-6: Audit logging (within same transaction)
    audit::log(&tx, AuditAction::Delete, "patient", Some(&id), None)?;

    tx.commit()?;

    Ok(())
}
