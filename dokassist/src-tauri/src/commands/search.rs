use crate::error::AppError;
use crate::search::{self, SearchResult};
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn global_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SearchResult>, AppError> {
    let limit = limit.unwrap_or(50);

    // ── Embed the query BEFORE acquiring the DB connection so that the
    // MutexGuard<Connection> is never held across an `.await` point (which
    // would make the future !Send and fail the Tauri handler bound).
    let embed_arc: Option<Arc<_>> = state.try_get_embed();
    let query_vec: Option<Vec<f32>> = if let Some(arc) = embed_arc {
        let q = query.clone();
        tokio::task::spawn_blocking(move || {
            arc.lock()
                .map_err(|_| AppError::Llm("Embed mutex poisoned".to_string()))?
                .embed_one(&q)
        })
        .await
        .ok()
        .and_then(|r| r.ok())
    } else {
        None
    };

    // ── Now it is safe to hold the DB lock for the synchronous search ──────
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    search::hybrid_search(&conn, &query, query_vec.as_deref(), limit)
}

// Keep the old search_patients command for backwards compatibility
#[tauri::command]
pub async fn search_patients(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<SearchResult>, AppError> {
    global_search(state, query, Some(50)).await
}
