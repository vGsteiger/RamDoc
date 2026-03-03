use crate::error::AppError;
use crate::filesystem::{self, MAX_FILE_SIZE};
use crate::llm::embed::EmbedEngine;
use crate::models::file_record::{self, FileRecord};
use crate::state::{AppState, AuthState};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn upload_file(
    state: State<'_, AppState>,
    patient_id: String,
    filename: String,
    data: Vec<u8>,
    mime_type: String,
) -> Result<FileRecord, AppError> {
    // HIGH-1: Enforce maximum file size before any I/O
    if data.len() > MAX_FILE_SIZE {
        return Err(AppError::Validation(format!(
            "Uploaded file size {} bytes exceeds maximum allowed size of {} bytes",
            data.len(),
            MAX_FILE_SIZE
        )));
    }

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

/// Extract text from an uploaded file, update the FTS5 index, embed the text,
/// and persist the embedding vector.  Call this after `upload_file` returns.
///
/// Flow:
///   1. Auth guard (Unlocked)
///   2. Decrypt file from vault
///   3. Extract text (PDF → pdf-extract, text/* → UTF-8, others → skip)
///   4. Sanitize extracted text
///   5. Persist extracted_text + document_type to `files` table
///   6. Update FTS5 search_index
///   7. Embed text via fastembed NomicEmbedTextV15 (downloads model on first call)
///   8. Persist embedding BLOB to `document_embeddings`
///   9. Cache embed engine in AppState for `global_search`
///  10. Emit `"file-processed"` event with the file_id
#[tauri::command]
pub async fn process_file(
    app: AppHandle,
    state: State<'_, AppState>,
    file_id: String,
) -> Result<(), AppError> {
    // ── 1. Auth guard ──────────────────────────────────────────────────────
    let fs_key: [u8; 32] = {
        let auth = state.auth.lock().map_err(|_| {
            AppError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some("Auth state mutex poisoned".to_string()),
            ))
        })?;
        match &*auth {
            AuthState::Unlocked { fs_key, .. } => **fs_key,
            _ => return Err(AppError::AuthRequired),
        }
    };

    // ── 2. Load file record ────────────────────────────────────────────────
    let file = {
        let db = state.get_db()?;
        let conn = db.conn()?;
        file_record::get_file_record(&conn, &file_id)?
    };

    let mime_type = file.mime_type.clone();
    let vault_path = file.vault_path.clone();
    let data_dir = state.data_dir.clone();

    // ── 3 & 4. Decrypt + extract text (blocking I/O / CPU) ─────────────────
    let text: String = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        let bytes = filesystem::read_file(&data_dir, &fs_key, &vault_path)?;

        let raw = match mime_type.as_str() {
            "application/pdf" => pdf_extract::extract_text_from_mem(&bytes)
                .map_err(|e| AppError::Llm(format!("PDF text extraction failed: {e}")))?,
            t if t.starts_with("text/") => String::from_utf8_lossy(&bytes).into_owned(),
            _ => return Ok(String::new()), // unsupported type — skip
        };

        Ok(crate::llm::sanitize::sanitize_for_prompt(&raw))
    })
    .await
    .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;

    // Nothing to embed for unsupported file types
    if text.is_empty() {
        return Ok(());
    }

    // ── 5 & 6. Persist text + update FTS5 ─────────────────────────────────
    {
        let db = state.get_db()?;
        let conn = db.conn()?;

        let patient_name: String = conn
            .query_row(
                "SELECT first_name || ' ' || last_name FROM patients WHERE id = ?1",
                [&file.patient_id],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "Unknown Patient".to_string());

        file_record::update_file_text(&conn, &file_id, &text, None)?;

        crate::search::index_file(
            &conn,
            &file_id,
            &file.patient_id,
            &patient_name,
            &file.filename,
            &text,
            None,
            None,
        )?;
    }

    // ── 7. Embed text (blocking — may download ~130 MB model on first run) ─
    let embed_cache_dir = state.data_dir.join("models").join("embed");
    let text_for_embed = text.clone();

    let (vector, engine) =
        tokio::task::spawn_blocking(move || -> Result<(Vec<f32>, EmbedEngine), AppError> {
            std::fs::create_dir_all(&embed_cache_dir)?;
            let engine = EmbedEngine::new(&embed_cache_dir)?;
            let vector = engine.embed_one(&text_for_embed)?;
            Ok((vector, engine))
        })
        .await
        .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;

    // ── 8. Persist embedding ───────────────────────────────────────────────
    {
        let db = state.get_db()?;
        let conn = db.conn()?;
        crate::search::save_embedding(&conn, &file_id, &vector)?;
    }

    // ── 9. Cache engine in state so global_search can use it ──────────────
    state.set_embed(engine)?;

    // ── 10. Notify frontend ────────────────────────────────────────────────
    let _ = app.emit("file-processed", &file_id);

    Ok(())
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
