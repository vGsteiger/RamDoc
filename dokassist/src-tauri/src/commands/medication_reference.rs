use crate::error::AppError;
use crate::medication_reference::{
    self, download, get_db_version, SubstanceDetail, SubstanceSummary,
};
use crate::state::AppState;
use tauri::{AppHandle, State};

/// Search the local medication reference DB.
///
/// Returns up to 10 matching substances for the autocomplete dropdown.
/// Returns an empty list (not an error) when the reference DB is not installed.
#[tauri::command]
pub async fn search_medication_reference(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<SubstanceSummary>, AppError> {
    let guard = state
        .get_medication_ref()
        .ok_or_else(|| AppError::Validation("Medication ref mutex poisoned".to_string()))?;

    match guard.as_ref() {
        None => Ok(vec![]),
        Some(conn) => medication_reference::search_substances(conn, &query, 10),
    }
}

/// Return full detail for a single substance (indication, side effects, contraindications).
///
/// Returns `NOT_FOUND` if the reference DB is not installed or the ID is unknown.
#[tauri::command]
pub async fn get_medication_reference_detail(
    state: State<'_, AppState>,
    id: String,
) -> Result<SubstanceDetail, AppError> {
    let guard = state
        .get_medication_ref()
        .ok_or_else(|| AppError::Validation("Medication ref mutex poisoned".to_string()))?;

    match guard.as_ref() {
        None => Err(AppError::NotFound(
            "medication reference DB not installed".to_string(),
        )),
        Some(conn) => medication_reference::get_substance_detail(conn, &id),
    }
}

/// Return the installed reference DB version string, or `null` if not installed.
#[tauri::command]
pub async fn get_medication_reference_version(
    state: State<'_, AppState>,
) -> Result<Option<String>, AppError> {
    let guard = state
        .get_medication_ref()
        .ok_or_else(|| AppError::Validation("Medication ref mutex poisoned".to_string()))?;

    Ok(guard.as_ref().and_then(get_db_version))
}

/// Download, verify, and install the medication reference DB.
///
/// Emits `"medication-ref-download-progress"` (f64 0.0–1.0) and
/// `"medication-ref-download-done"` events on the app handle.
#[tauri::command]
pub async fn download_medication_reference(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let dest_path = state.data_dir.join("medication_ref.sqlite");

    // Close any existing DB connection before download to avoid file lock issues on Windows
    {
        let mut guard = state
            .medication_ref
            .lock()
            .map_err(|_| AppError::Validation("Medication ref mutex poisoned".to_string()))?;
        *guard = None;
    }

    download::download_reference_db(&app, &dest_path).await?;

    // Reload the connection in AppState so searches work immediately.
    let conn = medication_reference::open_reference_db(&dest_path)?;
    state.set_medication_ref(conn)?;

    Ok(())
}
