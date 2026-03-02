use crate::audit::{query_log, AuditEntry};
use crate::error::AppError;
use crate::state::AppState;
use chrono::DateTime;
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

/// Validate and normalize an ISO 8601 timestamp string
fn validate_timestamp(timestamp: Option<&str>) -> Result<Option<String>, AppError> {
    match timestamp {
        None => Ok(None),
        Some(ts) => {
            // Parse as RFC3339/ISO8601 to validate format
            DateTime::parse_from_rfc3339(ts)
                .map_err(|_| AppError::Validation(format!("Invalid ISO 8601 timestamp: {}", ts)))?;
            Ok(Some(ts.to_string()))
        }
    }
}

/// Query the audit log
/// This command is for administrative/settings purposes to view audit history
#[tauri::command]
pub async fn query_audit_log(
    state: State<'_, AppState>,
    request: QueryAuditLogRequest,
) -> Result<Vec<AuditEntry>, AppError> {
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    // Validate timestamp inputs
    let from = validate_timestamp(request.from.as_deref())?;
    let to = validate_timestamp(request.to.as_deref())?;

    query_log(
        &conn,
        request.entity_type.as_deref(),
        request.entity_id.as_deref(),
        from.as_deref(),
        to.as_deref(),
        request.limit.unwrap_or(100),
        request.offset.unwrap_or(0),
    )
}
