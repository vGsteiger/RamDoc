//! Tool dispatch for the agent loop.
//! Each tool validates its arguments, calls existing model functions, and returns
//! a JSON value.  Security rules (scope enforcement, input sanitization) are
//! applied here before reaching model code.

use super::agent::{AgentScope, ToolCallRequest};
use super::sanitize::sanitize_for_prompt;
use crate::error::AppError;
use crate::llm::embed::EmbedEngine;
use crate::llm::LlmEngine;
use crate::models::patient;
use crate::models::session::{self, CreateSession};
use crate::search;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::Manager;

/// Dispatch a tool call, returning a JSON Value.
pub fn dispatch_tool(
    conn: &Connection,
    app: &tauri::AppHandle,
    engine: &Arc<LlmEngine>,
    scope: &AgentScope,
    call: &ToolCallRequest,
) -> Result<Value, AppError> {
    match call.name.as_str() {
        "get_patient" => tool_get_patient(conn, scope, &call.args),
        "list_patients" => tool_list_patients(conn, scope),
        "get_calendar_events" => tool_get_calendar_events(conn, scope, &call.args),
        "create_calendar_event" => tool_create_calendar_event(conn, scope, &call.args),
        "search" => tool_search(conn, &call.args),
        "search_literature" => tool_search_literature(conn, app, &call.args),
        "write_report" => tool_write_report(conn, app, engine, scope, &call.args),
        unknown => Err(AppError::Validation(format!("Unknown tool: {}", unknown))),
    }
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, AppError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Validation(format!("Missing or invalid arg: {}", key)))
}

fn opt_str_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

fn enforce_patient_scope(scope: &AgentScope, patient_id: &str) -> Result<(), AppError> {
    if let AgentScope::Patient {
        patient_id: scope_id,
    } = scope
    {
        if scope_id != patient_id {
            return Err(AppError::Validation(
                "Patient scope: only own patient_id is accessible".to_string(),
            ));
        }
    }
    Ok(())
}

fn tool_get_patient(
    conn: &Connection,
    scope: &AgentScope,
    args: &Value,
) -> Result<Value, AppError> {
    let patient_id = sanitize_for_prompt(str_arg(args, "patient_id")?);
    enforce_patient_scope(scope, &patient_id)?;
    let p = patient::get_patient(conn, &patient_id)?;
    Ok(serde_json::to_value(p).unwrap_or(json!({"error": "serialize"})))
}

fn tool_list_patients(conn: &Connection, scope: &AgentScope) -> Result<Value, AppError> {
    if matches!(scope, AgentScope::Patient { .. }) {
        return Err(AppError::Validation(
            "list_patients is only available in global scope".to_string(),
        ));
    }
    let patients = patient::list_patients(conn, 100, 0)?;
    Ok(serde_json::to_value(patients).unwrap_or(json!({"error": "serialize"})))
}

fn tool_get_calendar_events(
    conn: &Connection,
    scope: &AgentScope,
    args: &Value,
) -> Result<Value, AppError> {
    let patient_id_arg = opt_str_arg(args, "patient_id").map(sanitize_for_prompt);

    // In patient scope, force to own patient
    let effective_patient_id = match scope {
        AgentScope::Patient { patient_id } => Some(patient_id.clone()),
        AgentScope::Global => patient_id_arg,
    };

    if let Some(pid) = &effective_patient_id {
        let sessions = session::list_sessions_for_patient(conn, pid, 50, 0)?;
        Ok(serde_json::to_value(sessions).unwrap_or(json!({"error": "serialize"})))
    } else {
        let sessions = session::list_all_sessions(conn, 50, 0)?;
        Ok(serde_json::to_value(sessions).unwrap_or(json!({"error": "serialize"})))
    }
}

fn tool_create_calendar_event(
    conn: &Connection,
    scope: &AgentScope,
    args: &Value,
) -> Result<Value, AppError> {
    let patient_id = sanitize_for_prompt(str_arg(args, "patient_id")?);
    enforce_patient_scope(scope, &patient_id)?;

    let date = sanitize_for_prompt(str_arg(args, "date")?);
    // Basic date validation (YYYY-MM-DD)
    if date.len() != 10
        || !date.chars().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        })
    {
        return Err(AppError::Validation(
            "date must be in YYYY-MM-DD format".to_string(),
        ));
    }

    let session_type_raw = str_arg(args, "session_type")?;
    // Whitelist of allowed session types
    let allowed_types = [
        "Erstgespräch",
        "Einzeltherapie",
        "Gruppentherapie",
        "Diagnostik",
        "Verlaufskontrolle",
        "Abschlussgespräch",
        "Krisenintervention",
        "Konsultation",
    ];
    if !allowed_types.contains(&session_type_raw) {
        return Err(AppError::Validation(format!(
            "Invalid session_type '{}'. Allowed: {}",
            session_type_raw,
            allowed_types.join(", ")
        )));
    }
    let session_type = session_type_raw.to_string();

    let duration_minutes = args
        .get("duration_minutes")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    let notes = opt_str_arg(args, "notes").map(sanitize_for_prompt);

    let created = session::create_session(
        conn,
        CreateSession {
            patient_id,
            session_date: date,
            session_type,
            duration_minutes,
            notes,
            amdp_data: None,
        },
    )?;
    Ok(serde_json::to_value(created).unwrap_or(json!({"error": "serialize"})))
}

fn tool_search(conn: &Connection, args: &Value) -> Result<Value, AppError> {
    let raw_query = str_arg(args, "query")?;
    let safe_query = sanitize_for_prompt(raw_query);
    let results = search::search(conn, &safe_query, 20)?;
    Ok(serde_json::to_value(results).unwrap_or(json!({"error": "serialize"})))
}

fn tool_search_literature(
    conn: &Connection,
    app: &tauri::AppHandle,
    args: &Value,
) -> Result<Value, AppError> {
    let raw_query = str_arg(args, "query")?;
    let safe_query = sanitize_for_prompt(raw_query);

    // Lazy-init: this fn runs inside spawn_blocking so direct blocking I/O is safe
    let state = app.state::<crate::state::AppState>();
    let embed_engine = if let Some(engine) = state.try_get_embed() {
        engine
    } else {
        let embed_cache_dir = state.data_dir.join("models").join("embed");
        std::fs::create_dir_all(&embed_cache_dir)
            .map_err(|e| AppError::Llm(format!("Failed to create embed cache dir: {e}")))?;
        let engine = EmbedEngine::new(&embed_cache_dir)?;
        state.set_embed(engine)?;
        state
            .try_get_embed()
            .ok_or_else(|| AppError::Llm("Embed engine unavailable".to_string()))?
    };

    // Embed the query (CPU-bound operation)
    let query_vec = embed_engine
        .lock()
        .map_err(|_| AppError::Llm("Embed mutex poisoned".to_string()))?
        .embed_one(&safe_query)
        .map_err(|e| AppError::Llm(format!("Failed to embed query: {}", e)))?;

    // Search literature chunks
    let results = search::search_literature_chunks(conn, &query_vec, 5)?;

    Ok(serde_json::to_value(results).unwrap_or(json!({"error": "serialize"})))
}

fn tool_write_report(
    conn: &Connection,
    app: &tauri::AppHandle,
    engine: &Arc<LlmEngine>,
    scope: &AgentScope,
    args: &Value,
) -> Result<Value, AppError> {
    let patient_id = sanitize_for_prompt(str_arg(args, "patient_id")?);
    enforce_patient_scope(scope, &patient_id)?;

    let report_type_raw = str_arg(args, "report_type")?;
    let rt = match report_type_raw {
        "Befundbericht" => crate::llm::ReportType::Befundbericht,
        "Verlaufsbericht" => crate::llm::ReportType::Verlaufsbericht,
        "Ueberweisungsschreiben" => crate::llm::ReportType::Ueberweisungsschreiben,
        other => {
            return Err(AppError::Validation(format!(
                "Invalid report_type '{}'. Use: Befundbericht | Verlaufsbericht | Ueberweisungsschreiben",
                other
            )))
        }
    };

    let session_notes = sanitize_for_prompt(str_arg(args, "session_notes")?);

    // Fetch patient context for the report
    let p = patient::get_patient(conn, &patient_id)?;
    let patient_context = format!("{} {}, geb. {}", p.first_name, p.last_name, p.date_of_birth);

    let content = crate::llm::generate_report_streaming_with_prompt(
        app,
        engine,
        rt,
        &patient_context,
        &session_notes,
        crate::llm::SYSTEM_PROMPT_DE,
    )?;

    // Persist the report
    let report = crate::models::report::create_report(
        conn,
        crate::models::report::CreateReport {
            patient_id: patient_id.clone(),
            report_type: report_type_raw.to_string(),
            content: content.clone(),
            model_name: None,
            prompt_hash: None,
            session_ids: None,
        },
    )?;

    Ok(json!({
        "report_id": report.id,
        "content_preview": &content[..content.len().min(500)],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enforce_patient_scope_same_id() {
        let scope = AgentScope::Patient {
            patient_id: "p1".to_string(),
        };
        assert!(enforce_patient_scope(&scope, "p1").is_ok());
    }

    #[test]
    fn test_enforce_patient_scope_different_id() {
        let scope = AgentScope::Patient {
            patient_id: "p1".to_string(),
        };
        assert!(enforce_patient_scope(&scope, "p2").is_err());
    }

    #[test]
    fn test_enforce_global_scope_passes_any_id() {
        let scope = AgentScope::Global;
        assert!(enforce_patient_scope(&scope, "any_id").is_ok());
    }

    #[test]
    fn test_list_patients_blocked_in_patient_scope() {
        // We don't have a real DB here, but the scope check should fail early
        let dir = tempfile::tempdir().unwrap();
        let pool =
            crate::database::init_db(&dir.path().join("t.db"), &crate::crypto::generate_key())
                .unwrap();
        let conn = pool.conn().unwrap();
        let scope = AgentScope::Patient {
            patient_id: "p1".to_string(),
        };
        let result = tool_list_patients(&conn, &scope);
        assert!(result.is_err());
    }
}
