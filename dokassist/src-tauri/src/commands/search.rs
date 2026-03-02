use crate::error::AppError;
use crate::search::{self, SearchResult};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn global_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SearchResult>, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

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
