//! Tool dispatch for the agent loop.
//! Each tool validates its arguments, calls existing model functions, and returns
//! a JSON value.  Security rules (scope enforcement, input sanitization) are
//! applied here before reaching model code.

use super::agent::{AgentScope, ToolCallRequest};
use super::sanitize::sanitize_for_prompt;
use crate::error::AppError;
use crate::llm::embed::EmbedEngine;
use crate::llm::LlmEngine;
use crate::models::diagnosis;
use crate::models::medication;
use crate::models::patient;
use crate::models::session;
use crate::models::treatment_plan;
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
    thinking_effort: crate::llm::ThinkingEffort,
) -> Result<Value, AppError> {
    match call.name.as_str() {
        "get_patient" => tool_get_patient(conn, scope, &call.args),
        "list_patients" => tool_list_patients(conn, scope),
        "get_calendar_events" => tool_get_calendar_events(conn, scope, &call.args),
        "create_calendar_event" => tool_create_calendar_event(conn, scope, &call.args),
        "search" => tool_search(conn, &call.args),
        "search_literature" => tool_search_literature(conn, app, &call.args),
        "lookup_medication_reference" => tool_lookup_medication_reference(app, &call.args),
        "write_report" => tool_write_report(conn, app, engine, scope, &call.args, thinking_effort),
        "list_diagnoses" => tool_list_diagnoses(conn, scope, &call.args),
        "create_diagnosis" => tool_create_diagnosis(conn, scope, &call.args),
        "list_medications" => tool_list_medications(conn, scope, &call.args),
        "create_medication" => tool_create_medication(conn, scope, &call.args),
        "compare_medications" => tool_compare_medications(app, &call.args),
        "draft_email" => tool_draft_email(conn, scope, &call.args),
        "list_treatment_plans" => tool_list_treatment_plans(conn, scope, &call.args),
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

/// Return a validated write proposal without persisting it.
///
/// Agent input can include untrusted patient documents, so an agent must never
/// be able to mutate a clinical record directly. The UI (or an existing
/// clinician-operated form) must explicitly submit the proposal through the
/// command named by `action` before any database write occurs.
fn proposed_write(action: &str, proposal: Value) -> Value {
    json!({
        "status": "pending_clinician_confirmation",
        "action": action,
        "proposal": proposal,
        "message": "Dieser Vorschlag wurde nicht gespeichert. Bitte prüfen und über die Benutzeroberfläche ausdrücklich bestätigen.",
    })
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
    _conn: &Connection,
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

    Ok(proposed_write(
        "create_session",
        json!({
            "patient_id": patient_id,
            "session_date": date,
            "session_type": session_type,
            "duration_minutes": duration_minutes,
            "scheduled_time": null,
            "notes": notes,
            "amdp_data": null,
        }),
    ))
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

fn tool_lookup_medication_reference(
    app: &tauri::AppHandle,
    args: &Value,
) -> Result<Value, AppError> {
    let query = sanitize_for_prompt(str_arg(args, "query")?);
    let state = app.state::<crate::state::AppState>();
    let guard = state
        .get_medication_ref()
        .ok_or_else(|| AppError::Validation("Medication ref mutex poisoned".to_string()))?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| AppError::NotFound("medication reference DB not installed".to_string()))?;

    let details = crate::medication_reference::search_substance_details(conn, &query, 5)?;

    Ok(serde_json::to_value(details).unwrap_or(json!({"error": "serialize"})))
}

fn tool_write_report(
    conn: &Connection,
    app: &tauri::AppHandle,
    engine: &Arc<LlmEngine>,
    scope: &AgentScope,
    args: &Value,
    thinking_effort: crate::llm::ThinkingEffort,
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

    // Fetch patient + clinical data for the report
    let p = patient::get_patient(conn, &patient_id)?;
    let diagnoses =
        diagnosis::list_diagnoses_for_patient(conn, &patient_id, 20, 0).unwrap_or_default();
    let medications =
        medication::list_medications_for_patient(conn, &patient_id, 20, 0).unwrap_or_default();
    let sessions = session::list_sessions_for_patient(conn, &patient_id, 5, 0).unwrap_or_default();

    let mut ctx = format!(
        "Name: {} {}\nGeburtsdatum: {}\nAHV-Nummer: {}",
        p.first_name, p.last_name, p.date_of_birth, p.ahv_number
    );
    if let Some(ins) = &p.insurance {
        ctx.push_str(&format!("\nVersicherung: {}", ins));
    }
    if let Some(gp) = &p.gp_name {
        ctx.push_str(&format!("\nHausarzt: {}", gp));
    }

    if !diagnoses.is_empty() {
        ctx.push_str("\n\nDiagnosen:");
        for d in &diagnoses {
            ctx.push_str(&format!(
                "\n- {} {} ({}, seit {})",
                sanitize_for_prompt(&d.icd10_code),
                sanitize_for_prompt(&d.description),
                sanitize_for_prompt(&d.status),
                sanitize_for_prompt(&d.diagnosed_date),
            ));
        }
    }

    let current_meds: Vec<_> = medications
        .iter()
        .filter(|m| m.end_date.is_none())
        .collect();
    if !current_meds.is_empty() {
        ctx.push_str("\n\nAktuelle Medikamente:");
        for m in &current_meds {
            ctx.push_str(&format!(
                "\n- {} {}, {}",
                sanitize_for_prompt(&m.substance),
                sanitize_for_prompt(&m.dosage),
                sanitize_for_prompt(&m.frequency),
            ));
        }
    }

    if !sessions.is_empty() {
        ctx.push_str("\n\nLetzte Sitzungen:");
        for s in &sessions {
            let mut line = format!(
                "\n- {}: {}",
                sanitize_for_prompt(&s.session_date),
                sanitize_for_prompt(&s.session_type),
            );
            let summary = s.clinical_summary.as_deref().or(s.notes.as_deref());
            if let Some(text) = summary {
                let safe = sanitize_for_prompt(text);
                let truncated: String = safe.chars().take(400).collect();
                line.push_str(&format!(" — {}", truncated));
            }
            ctx.push_str(&line);
        }
    }

    let patient_context = ctx;

    let content = crate::llm::generate_report_streaming_with_prompt(
        app,
        engine,
        rt,
        &patient_context,
        &session_notes,
        None,
        None,
        crate::llm::SYSTEM_PROMPT_DE,
        thinking_effort,
    )?;

    crate::llm::sanitize::validate_report_output(&content)?;

    Ok(proposed_write(
        "create_report",
        json!({
            "patient_id": patient_id,
            "report_type": report_type_raw,
            "content": content,
            "model_name": null,
            "prompt_hash": null,
            "session_ids": null,
        }),
    ))
}

/// Validate YYYY-MM-DD format (same check as in tool_create_calendar_event).
fn validate_date(date: &str) -> Result<(), AppError> {
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
    Ok(())
}

fn tool_list_diagnoses(
    conn: &Connection,
    scope: &AgentScope,
    args: &Value,
) -> Result<Value, AppError> {
    let patient_id = sanitize_for_prompt(str_arg(args, "patient_id")?);
    enforce_patient_scope(scope, &patient_id)?;
    let diagnoses = diagnosis::list_diagnoses_for_patient(conn, &patient_id, 50, 0)?;
    Ok(serde_json::to_value(diagnoses).unwrap_or(json!({"error": "serialize"})))
}

fn tool_create_diagnosis(
    _conn: &Connection,
    scope: &AgentScope,
    args: &Value,
) -> Result<Value, AppError> {
    let patient_id = sanitize_for_prompt(str_arg(args, "patient_id")?);
    enforce_patient_scope(scope, &patient_id)?;

    let icd10_code = str_arg(args, "icd10_code")?.trim().to_uppercase();
    // Basic ICD-10 validation: length 3–10, starts with a letter then 2 alphanumeric chars,
    // followed by optional dot and alphanumeric suffix.
    if icd10_code.len() < 3
        || icd10_code.len() > 10
        || !icd10_code
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic())
            .unwrap_or(false)
        || !icd10_code
            .chars()
            .nth(1)
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        || !icd10_code
            .chars()
            .nth(2)
            .map(|c| c.is_ascii_alphanumeric())
            .unwrap_or(false)
        || !icd10_code
            .chars()
            .skip(3)
            .all(|c| c.is_ascii_alphanumeric() || c == '.')
    {
        return Err(AppError::Validation(
            "icd10_code must be a valid ICD-10 code (e.g. F32.1)".to_string(),
        ));
    }

    let description = sanitize_for_prompt(str_arg(args, "description")?);
    let diagnosed_date = sanitize_for_prompt(str_arg(args, "diagnosed_date")?);
    validate_date(&diagnosed_date)?;

    let status_raw = opt_str_arg(args, "status").unwrap_or("active");
    let allowed_statuses = ["active", "resolved", "chronic"];
    if !allowed_statuses.contains(&status_raw) {
        return Err(AppError::Validation(format!(
            "Invalid status '{}'. Allowed: active | resolved | chronic",
            status_raw
        )));
    }
    let notes = opt_str_arg(args, "notes").map(sanitize_for_prompt);

    Ok(proposed_write(
        "create_diagnosis",
        json!({
            "patient_id": patient_id,
            "icd10_code": icd10_code,
            "description": description,
            "status": status_raw,
            "diagnosed_date": diagnosed_date,
            "resolved_date": null,
            "notes": notes,
        }),
    ))
}

fn tool_list_medications(
    conn: &Connection,
    scope: &AgentScope,
    args: &Value,
) -> Result<Value, AppError> {
    let patient_id = sanitize_for_prompt(str_arg(args, "patient_id")?);
    enforce_patient_scope(scope, &patient_id)?;
    let medications = medication::list_medications_for_patient(conn, &patient_id, 50, 0)?;
    Ok(serde_json::to_value(medications).unwrap_or(json!({"error": "serialize"})))
}

fn tool_create_medication(
    _conn: &Connection,
    scope: &AgentScope,
    args: &Value,
) -> Result<Value, AppError> {
    let patient_id = sanitize_for_prompt(str_arg(args, "patient_id")?);
    enforce_patient_scope(scope, &patient_id)?;

    let substance = sanitize_for_prompt(str_arg(args, "substance")?);
    let dosage = sanitize_for_prompt(str_arg(args, "dosage")?);
    let frequency = sanitize_for_prompt(str_arg(args, "frequency")?);
    let start_date = sanitize_for_prompt(str_arg(args, "start_date")?);
    validate_date(&start_date)?;
    let notes = opt_str_arg(args, "notes").map(sanitize_for_prompt);

    Ok(proposed_write(
        "create_medication",
        json!({
            "patient_id": patient_id,
            "substance": substance,
            "dosage": dosage,
            "frequency": frequency,
            "start_date": start_date,
            "end_date": null,
            "notes": notes,
        }),
    ))
}

fn tool_compare_medications(app: &tauri::AppHandle, args: &Value) -> Result<Value, AppError> {
    let current_id = sanitize_for_prompt(str_arg(args, "current_substance_id")?);
    let replacement_id = sanitize_for_prompt(str_arg(args, "replacement_substance_id")?);

    let state = app.state::<crate::state::AppState>();

    let guard = state
        .get_medication_ref()
        .ok_or_else(|| AppError::Validation("Medication ref mutex poisoned".to_string()))?;

    let conn = guard
        .as_ref()
        .ok_or_else(|| AppError::NotFound("medication reference DB not installed".to_string()))?;

    let current = crate::medication_reference::get_substance_detail(conn, &current_id)?;
    let replacement = crate::medication_reference::get_substance_detail(conn, &replacement_id)?;

    // Build a structured comparison highlighting key differences
    let mut comparison = json!({
        "current_medication": {
            "name": current.name_de,
            "atc_code": current.atc_code,
            "trade_names": current.trade_names,
            "indication": current.indication,
            "side_effects": current.side_effects,
            "contraindications": current.contraindications,
        },
        "replacement_medication": {
            "name": replacement.name_de,
            "atc_code": replacement.atc_code,
            "trade_names": replacement.trade_names,
            "indication": replacement.indication,
            "side_effects": replacement.side_effects,
            "contraindications": replacement.contraindications,
        }
    });

    // Add a summary field to help the LLM
    let mut summary_notes = Vec::new();

    if current.atc_code == replacement.atc_code && current.atc_code.is_some() {
        summary_notes.push(
            "Beide Medikamente haben denselben ATC-Code (gleiche pharmakologische Klasse)"
                .to_string(),
        );
    }

    // Check for potential overlapping side effects
    if let (Some(current_se), Some(replacement_se)) =
        (&current.side_effects, &replacement.side_effects)
    {
        if !current_se.is_empty() && !replacement_se.is_empty() {
            summary_notes.push("Bitte Nebenwirkungen beider Medikamente vergleichen".to_string());
        }
    }

    if let Value::Object(ref mut map) = comparison {
        map.insert(
            "summary_notes".to_string(),
            Value::Array(summary_notes.into_iter().map(Value::String).collect()),
        );
    }

    Ok(comparison)
}

fn tool_draft_email(
    _conn: &Connection,
    scope: &AgentScope,
    args: &Value,
) -> Result<Value, AppError> {
    let patient_id = sanitize_for_prompt(str_arg(args, "patient_id")?);
    enforce_patient_scope(scope, &patient_id)?;

    let recipient_email = str_arg(args, "recipient_email")?;
    // Basic email validation: must have '@' and a '.' after the '@'
    let at_pos = recipient_email.find('@').ok_or_else(|| {
        AppError::Validation("recipient_email must be a valid email address".to_string())
    })?;
    if !recipient_email[at_pos + 1..].contains('.') {
        return Err(AppError::Validation(
            "recipient_email must be a valid email address".to_string(),
        ));
    }
    let recipient_email = sanitize_for_prompt(recipient_email);

    let subject = sanitize_for_prompt(str_arg(args, "subject")?);
    let body = sanitize_for_prompt(str_arg(args, "body")?);

    Ok(proposed_write(
        "create_email",
        json!({
            "patient_id": patient_id,
            "recipient_email": recipient_email,
            "subject": subject,
            "body": body,
        }),
    ))
}

fn tool_list_treatment_plans(
    conn: &Connection,
    scope: &AgentScope,
    args: &Value,
) -> Result<Value, AppError> {
    let patient_id = sanitize_for_prompt(str_arg(args, "patient_id")?);
    enforce_patient_scope(scope, &patient_id)?;
    let plans = treatment_plan::list_treatment_plans_for_patient(conn, &patient_id, 50, 0)?;
    Ok(serde_json::to_value(plans).unwrap_or(json!({"error": "serialize"})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_proposal_requires_clinician_confirmation() {
        let result = proposed_write("create_diagnosis", json!({"patient_id": "p1"}));
        assert_eq!(result["status"], "pending_clinician_confirmation");
        assert_eq!(result["action"], "create_diagnosis");
        assert_eq!(result["proposal"]["patient_id"], "p1");
    }

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
