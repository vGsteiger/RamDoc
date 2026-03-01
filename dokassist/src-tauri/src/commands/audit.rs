use crate::audit::{query_log, AuditEntry};
use crate::error::AppError;
use crate::state::AppState;
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
pub struct QueryAuditLogRequest {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Query the audit log
/// This command is for administrative/settings purposes to view audit history
#[tauri::command]
pub async fn query_audit_log(
    _state: State<'_, AppState>,
    request: QueryAuditLogRequest,
) -> Result<Vec<AuditEntry>, AppError> {
    // PKG-2: When database is implemented, get connection from state
    // For now, return empty vec as database infrastructure isn't ready yet
    // let conn = state.db.conn()?;

    // query_log(
    //     &conn,
    //     request.entity_type.as_deref(),
    //     request.entity_id.as_deref(),
    //     request.from.as_deref(),
    //     request.to.as_deref(),
    //     request.limit.unwrap_or(100),
    //     request.offset.unwrap_or(0),
    // )

    // Temporary: return empty until PKG-2 is implemented
    Ok(vec![])
}
