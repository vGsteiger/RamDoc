use crate::error::AppError;
use crate::llm::{self, ChunkConfig};
use crate::models::compendium::{
    self, CompendiumEntry, DocumentChunk, create_compendium_entry, create_document_chunk,
    delete_chunks_for_file, delete_compendium_entry, list_chunks_for_file,
    list_compendium_entries, search_compendium as db_search_compendium,
};
use crate::models::file_record;
use crate::state::{AppState, AuthState};
use serde::{Deserialize, Serialize};
use tauri::State;

/// Check that the user is authenticated before processing sensitive data
fn check_auth(state: &AppState) -> Result<(), AppError> {
    let auth = state
        .auth
        .lock()
        .map_err(|_| AppError::Llm("Auth state mutex poisoned".to_string()))?;

    if !matches!(*auth, AuthState::Unlocked { .. }) {
        return Err(AppError::AuthRequired);
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk: DocumentChunk,
    pub entry: CompendiumEntry,
}

/// List all compendium entries
#[tauri::command]
pub async fn list_compendium(state: State<'_, AppState>) -> Result<Vec<CompendiumEntry>, AppError> {
    check_auth(&state)?;

    let db = state.get_db()?;
    let conn = db.conn()?;

    list_compendium_entries(&conn)
}

/// Add a file to the compendium
#[tauri::command]
pub async fn add_to_compendium(
    state: State<'_, AppState>,
    file_id: String,
    title: String,
    description: Option<String>,
    category: Option<String>,
    priority: i32,
) -> Result<CompendiumEntry, AppError> {
    check_auth(&state)?;

    let db = state.get_db()?;
    let conn = db.conn()?;

    // Verify the file exists
    let file = file_record::get_file_record(&conn, &file_id)?;

    // Mark file as compendium
    conn.execute(
        "UPDATE files SET is_compendium = 1 WHERE id = ?1",
        [&file_id],
    )?;

    // Create compendium entry
    create_compendium_entry(
        &conn,
        &file_id,
        &title,
        description.as_deref(),
        category.as_deref(),
        priority,
    )
}

/// Remove a file from the compendium
#[tauri::command]
pub async fn remove_from_compendium(
    state: State<'_, AppState>,
    entry_id: String,
) -> Result<(), AppError> {
    check_auth(&state)?;

    let db = state.get_db()?;
    let conn = db.conn()?;

    // Get the entry to find the file_id
    let entry = compendium::get_compendium_entry(&conn, &entry_id)?;

    // Delete the compendium entry
    delete_compendium_entry(&conn, &entry_id)?;

    // Delete associated chunks
    delete_chunks_for_file(&conn, &entry.file_id)?;

    // Unmark file as compendium
    conn.execute(
        "UPDATE files SET is_compendium = 0 WHERE id = ?1",
        [&entry.file_id],
    )?;

    Ok(())
}

/// Process a compendium document: chunk it and store chunks
#[tauri::command]
pub async fn process_compendium_document(
    state: State<'_, AppState>,
    file_id: String,
) -> Result<Vec<DocumentChunk>, AppError> {
    check_auth(&state)?;

    let db = state.get_db()?;
    let conn = db.conn()?;

    // Get the file record
    let file = file_record::get_file_record(&conn, &file_id)?;

    // Check if file has extracted text
    let text = file.extracted_text.ok_or_else(|| {
        AppError::Validation(
            "File has no extracted text. Please extract metadata first.".to_string(),
        )
    })?;

    // Delete existing chunks
    delete_chunks_for_file(&conn, &file_id)?;

    // Chunk the document
    let config = ChunkConfig::default();
    let chunks = llm::chunk_document(&text, &config)?;

    // Store chunks in database
    let mut stored_chunks = Vec::new();
    for chunk in chunks {
        let stored = create_document_chunk(
            &conn,
            &file_id,
            chunk.chunk_index as i32,
            &chunk.content,
            Some(chunk.token_count as i32),
        )?;
        stored_chunks.push(stored);
    }

    Ok(stored_chunks)
}

/// Search the compendium for relevant context
#[tauri::command]
pub async fn search_compendium(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, AppError> {
    check_auth(&state)?;

    let db = state.get_db()?;
    let conn = db.conn()?;

    let limit = limit.unwrap_or(5);
    let results = db_search_compendium(&conn, &query, limit)?;

    Ok(results
        .into_iter()
        .map(|(chunk, entry)| SearchResult { chunk, entry })
        .collect())
}

/// Get all chunks for a compendium file
#[tauri::command]
pub async fn get_compendium_chunks(
    state: State<'_, AppState>,
    file_id: String,
) -> Result<Vec<DocumentChunk>, AppError> {
    check_auth(&state)?;

    let db = state.get_db()?;
    let conn = db.conn()?;

    list_chunks_for_file(&conn, &file_id)
}
