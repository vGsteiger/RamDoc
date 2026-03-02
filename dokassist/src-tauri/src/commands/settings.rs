use crate::error::AppError;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_settings(_state: State<'_, AppState>) -> Result<String, AppError> {
    // PKG-1: implement
    Ok("{}".to_string())
}

#[tauri::command]
pub async fn update_settings(
    _state: State<'_, AppState>,
    _settings: String,
) -> Result<(), AppError> {
    // PKG-1: implement
    Ok(())
}
