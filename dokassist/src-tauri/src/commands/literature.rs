//! Tauri commands for literature document management

use crate::error::AppError;
use crate::filesystem;
use crate::llm::chunk::{create_literature_chunks, get_literature_chunks, ChunkConfig};
use crate::llm::embed::EmbedEngine;
use crate::models::literature::{
    create_literature, delete_literature, get_literature, list_literature, update_literature,
    CreateLiterature, Literature, UpdateLiterature,
};
use crate::search::{save_chunk_embedding, search_literature_chunks, LiteratureChunkResult};
use crate::state::{AppState, AuthState};
use rusqlite;
use tauri::Emitter;

fn get_fs_key(state: &AppState) -> Result<[u8; 32], AppError> {
    let auth = state.auth.lock().map_err(|_| {
        AppError::Database(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("Auth state mutex poisoned".to_string()),
        ))
    })?;
    match &*auth {
        AuthState::Unlocked { fs_key, .. } => Ok(**fs_key),
        _ => Err(AppError::AuthRequired),
    }
}

/// Upload a literature document
#[tauri::command]
pub async fn upload_literature(
    state: tauri::State<'_, AppState>,
    filename: String,
    data: Vec<u8>,
    mime_type: String,
    description: Option<String>,
) -> Result<Literature, AppError> {
    if data.len() > 500 * 1024 * 1024 {
        return Err(AppError::Validation(
            "File size exceeds 500 MiB limit".to_string(),
        ));
    }

    filesystem::init_vault(&state.data_dir)?;
    let fs_key = get_fs_key(&state)?;
    let vault_path = filesystem::store_literature_file(&state.data_dir, &fs_key, &data)?;

    let db = state.get_db()?;
    let conn = db.conn()?;
    let literature = create_literature(
        &conn,
        CreateLiterature {
            filename,
            vault_path,
            mime_type,
            size_bytes: data.len() as i64,
            description,
        },
    )?;

    Ok(literature)
}

/// Get a literature document by ID
#[tauri::command]
pub async fn get_literature_by_id(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Literature, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;
    get_literature(&conn, &id)
}

/// List all literature documents
#[tauri::command]
pub async fn list_all_literature(
    state: tauri::State<'_, AppState>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Literature>, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;
    list_literature(&conn, limit.unwrap_or(100), offset.unwrap_or(0))
}

/// Update literature metadata
#[tauri::command]
pub async fn update_literature_metadata(
    state: tauri::State<'_, AppState>,
    id: String,
    description: Option<String>,
) -> Result<Literature, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;
    update_literature(&conn, &id, UpdateLiterature { description })
}

/// Delete a literature document
#[tauri::command]
pub async fn delete_literature_document(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;

    let literature = get_literature(&conn, &id)?;
    delete_literature(&conn, &id)?;

    filesystem::delete_literature_file(&state.data_dir, &literature.vault_path)?;

    Ok(())
}

/// Download a literature document
#[tauri::command]
pub async fn download_literature(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Vec<u8>, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;
    let literature = get_literature(&conn, &id)?;
    drop(conn);
    drop(db);

    let fs_key = get_fs_key(&state)?;
    filesystem::read_literature_file(&state.data_dir, &fs_key, &literature.vault_path)
}

/// Process a literature document (extract text, chunk, and embed)
#[tauri::command]
pub async fn process_literature(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    id: String,
) -> Result<(), AppError> {
    let fs_key = get_fs_key(&state)?;

    let (literature, vault_path) = {
        let db = state.get_db()?;
        let conn = db.conn()?;
        let lit = get_literature(&conn, &id)?;
        let vp = lit.vault_path.clone();
        (lit, vp)
    };

    let data_dir = state.data_dir.clone();
    let mime_type = literature.mime_type.clone();

    // Decrypt and extract text (blocking I/O / CPU)
    let extracted_text = tokio::task::spawn_blocking(move || -> Result<String, AppError> {
        let decrypted = filesystem::read_literature_file(&data_dir, &fs_key, &vault_path)?;
        match mime_type.as_str() {
            "application/pdf" => pdf_extract::extract_text_from_mem(&decrypted)
                .map_err(|e| AppError::Validation(format!("PDF extraction failed: {}", e))),
            "text/plain" => String::from_utf8(decrypted)
                .map_err(|e| AppError::Validation(format!("UTF-8 decode failed: {}", e))),
            _ => Err(AppError::Validation(format!(
                "Unsupported file type for processing: {}",
                mime_type
            ))),
        }
    })
    .await
    .map_err(|e| AppError::Llm(format!("Task join error: {}", e)))??;

    if extracted_text.trim().is_empty() {
        return Err(AppError::Validation(
            "No text content could be extracted from file".to_string(),
        ));
    }

    // Chunk the text
    let config = ChunkConfig::default();
    let chunks = {
        let db = state.get_db()?;
        let conn = db.conn()?;
        create_literature_chunks(&conn, &id, &extracted_text, &config)?
    };

    // Get or create embed engine
    let embed_engine = if let Some(engine) = state.try_get_embed() {
        engine
    } else {
        let embed_cache_dir = state.data_dir.join("models").join("embed");
        let (engine,) = tokio::task::spawn_blocking(move || -> Result<(EmbedEngine,), AppError> {
            std::fs::create_dir_all(&embed_cache_dir)?;
            Ok((EmbedEngine::new(&embed_cache_dir)?,))
        })
        .await
        .map_err(|e| AppError::Llm(format!("Task join error: {}", e)))??;
        state.set_embed(engine)?;
        state
            .try_get_embed()
            .ok_or_else(|| AppError::Llm("Embed engine unavailable".to_string()))?
    };

    // Generate and save embeddings for each chunk
    for chunk in chunks {
        let content = chunk.content.clone();
        let chunk_id = chunk.id.clone();
        let engine = embed_engine.clone();

        let embedding = tokio::task::spawn_blocking(move || {
            engine
                .lock()
                .map_err(|_| AppError::Llm("Embed mutex poisoned".to_string()))?
                .embed_one(&content)
        })
        .await
        .map_err(|e| AppError::Llm(format!("Task join error: {}", e)))??;

        let db = state.get_db()?;
        let conn = db.conn()?;
        save_chunk_embedding(&conn, &chunk_id, &embedding)?;
    }

    app.emit("literature-processed", &id)
        .map_err(|e| AppError::Llm(format!("Failed to emit event: {}", e)))?;

    Ok(())
}

/// Search literature chunks using semantic similarity
#[tauri::command]
pub async fn search_literature(
    state: tauri::State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<LiteratureChunkResult>, AppError> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    // Lazy-init: load embed engine if not already in state (downloads ~130 MB on first use)
    let embed_engine = if let Some(engine) = state.try_get_embed() {
        engine
    } else {
        let embed_cache_dir = state.data_dir.join("models").join("embed");
        let (engine,) = tokio::task::spawn_blocking(move || -> Result<(EmbedEngine,), AppError> {
            std::fs::create_dir_all(&embed_cache_dir)?;
            Ok((EmbedEngine::new(&embed_cache_dir)?,))
        })
        .await
        .map_err(|e| AppError::Llm(format!("Task join error: {}", e)))??;
        state.set_embed(engine)?;
        state
            .try_get_embed()
            .ok_or_else(|| AppError::Llm("Embed engine unavailable".to_string()))?
    };

    let query_clone = query.clone();
    let query_vec = tokio::task::spawn_blocking(move || {
        embed_engine
            .lock()
            .map_err(|_| AppError::Llm("Embed mutex poisoned".to_string()))?
            .embed_one(&query_clone)
    })
    .await
    .map_err(|e| AppError::Llm(format!("Task join error: {}", e)))??;

    let db = state.get_db()?;
    let conn = db.conn()?;
    search_literature_chunks(&conn, &query_vec, limit.unwrap_or(5))
}

/// Get all chunks for a literature document (for debugging/inspection)
#[tauri::command]
pub async fn get_literature_document_chunks(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Vec<crate::llm::chunk::DocumentChunk>, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;
    get_literature_chunks(&conn, &id)
}
