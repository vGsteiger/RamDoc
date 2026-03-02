use crate::error::AppError;
use crate::filesystem;
use crate::models::file_record::{self, FileRecord};
use crate::state::{AppState, AuthState};
use tauri::State;

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
            Some("Auth state mutex poisoned".to_string()),
        ))
    })?;

    let fs_key = match &*auth {
        AuthState::Unlocked { fs_key, .. } => fs_key.clone(),
        _ => return Err(AppError::AuthRequired),
    };
    drop(auth);

    // Store encrypted file in vault
    let vault_path = filesystem::store_file(&state.data_dir, &fs_key, &patient_id, &data)?;

    // Create database record
    let db = state.get_db()?;
    let conn = db.conn()?;
    let size_bytes = data.len() as u64;

    // If DB insert fails, clean up the vault file
    let file_record = match file_record::create_file_record(
        &conn,
        &patient_id,
        &filename,
        &vault_path,
        &mime_type,
        size_bytes,
    ) {
        Ok(record) => record,
        Err(e) => {
            // Best-effort cleanup: remove the vault file if the DB insert fails
            let _ = filesystem::delete_file(&state.data_dir, &vault_path);
            return Err(e);
        }
    };

    Ok(file_record)
}

#[tauri::command]
pub async fn download_file(
    state: State<'_, AppState>,
    file_id: String,
) -> Result<Vec<u8>, AppError> {
    // Get file record from database to validate access and get vault_path
    let db = state.get_db()?;
    let conn = db.conn()?;
    let file = file_record::get_file_record(&conn, &file_id)?;

    // Release DB connection before filesystem I/O
    drop(conn);
    drop(db);

    // Get fs_key from auth state
    let auth = state.auth.lock().map_err(|_| {
        AppError::Database(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("Auth state mutex poisoned".to_string()),
        ))
    })?;

    let fs_key = match &*auth {
        AuthState::Unlocked { fs_key, .. } => fs_key.clone(),
        _ => return Err(AppError::AuthRequired),
    };
    drop(auth);

    // Read and decrypt file from vault
    let plaintext = filesystem::read_file(&state.data_dir, &fs_key, &file.vault_path)?;

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
pub async fn delete_file(state: State<'_, AppState>, file_id: String) -> Result<(), AppError> {
    // Get file record to find vault path
    let db = state.get_db()?;
    let conn = db.conn()?;

    let file = file_record::get_file_record(&conn, &file_id)?;
    let vault_path = file.vault_path.clone();

    // Release DB connection before performing filesystem I/O
    drop(conn);
    drop(db);

    // Delete from vault
    filesystem::delete_file(&state.data_dir, &vault_path)?;

    // Reacquire DB connection and delete database record
    let db = state.get_db()?;
    let conn = db.conn()?;
    file_record::delete_file_record(&conn, &file_id)?;

    Ok(())
}
