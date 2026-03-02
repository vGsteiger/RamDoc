use tauri::State;
use crate::error::AppError;
use crate::state::{AppState, AuthState};
use crate::models::file_record::{self, FileRecord};
use crate::filesystem;

#[tauri::command]
pub async fn upload_file(
    state: State<'_, AppState>,
    patient_id: String,
    filename: String,
    data: Vec<u8>,
    mime_type: String,
) -> Result<FileRecord, AppError> {
    // Ensure vault is initialized
    filesystem::init_vault(&state.data_dir)?;

    // Get fs_key from auth state
    let auth = state.auth.lock().map_err(|_| {
        AppError::Database(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("Auth state mutex poisoned".to_string())
        ))
    })?;

    let fs_key = match &*auth {
        AuthState::Unlocked { fs_key, .. } => fs_key.clone(),
        _ => return Err(AppError::AuthRequired),
    };
    drop(auth);

    // Store encrypted file in vault
    let vault_path = filesystem::store_file(
        &state.data_dir,
        &fs_key,
        &patient_id,
        &filename,
        &data,
    )?;

    // Create database record
    let db = state.get_db()?;
    let conn = db.conn()?;
    let size_bytes = data.len() as u64;

    let file_record = file_record::create_file_record(
        &conn,
        &patient_id,
        &filename,
        &vault_path,
        &mime_type,
        size_bytes,
    )?;

    Ok(file_record)
}

#[tauri::command]
pub async fn download_file(
    state: State<'_, AppState>,
    vault_path: String,
) -> Result<Vec<u8>, AppError> {
    // Get fs_key from auth state
    let auth = state.auth.lock().map_err(|_| {
        AppError::Database(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("Auth state mutex poisoned".to_string())
        ))
    })?;

    let fs_key = match &*auth {
        AuthState::Unlocked { fs_key, .. } => fs_key.clone(),
        _ => return Err(AppError::AuthRequired),
    };
    drop(auth);

    // Read and decrypt file from vault
    let plaintext = filesystem::read_file(&state.data_dir, &fs_key, &vault_path)?;

    Ok(plaintext)
}

#[tauri::command]
pub async fn list_files(
    state: State<'_, AppState>,
    patient_id: String,
) -> Result<Vec<FileRecord>, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;

    let files = file_record::list_files_for_patient(&conn, &patient_id)?;

    Ok(files)
}

#[tauri::command]
pub async fn delete_file(
    state: State<'_, AppState>,
    file_id: String,
) -> Result<(), AppError> {
    // Get file record to find vault path
    let db = state.get_db()?;
    let conn = db.conn()?;

    let file = file_record::get_file_record(&conn, &file_id)?;

    // Delete from vault
    filesystem::delete_file(&state.data_dir, &file.vault_path)?;

    // Delete database record
    file_record::delete_file_record(&conn, &file_id)?;

    Ok(())
}

