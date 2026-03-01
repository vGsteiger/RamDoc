use tauri::State;
use crate::error::AppError;
use crate::state::AppState;
use crate::search::{self, SearchResult};

#[tauri::command]
pub async fn global_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SearchResult>, AppError> {
    let db_guard = state.db.lock().map_err(|_| AppError::Database(rusqlite::Error::InvalidQuery))?;
    let db = db_guard.as_ref().ok_or(AppError::AuthRequired)?;
    let conn = db.conn()?;

    search::search(&conn, &query, limit.unwrap_or(50))
}

// Keep the old search_patients command for backwards compatibility
#[tauri::command]
pub async fn search_patients(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<SearchResult>, AppError> {
    global_search(state, query, Some(50)).await
}
