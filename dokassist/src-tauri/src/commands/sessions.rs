use crate::error::AppError;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn create_session(
    _state: State<'_, AppState>,
    _patient_id: String,
) -> Result<String, AppError> {
    // PKG-4: implement
    Err(AppError::Llm("Not implemented".to_string()))
}
