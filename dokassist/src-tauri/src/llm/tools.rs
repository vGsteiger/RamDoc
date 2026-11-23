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
    let state = app.state::<crate::state::AppState>();
    let guard = state
        .get_medication_ref()
        .ok_or_else(|| AppError::Validation("Medication ref mutex poisoned".to_string()))?;
    let conn = guard
        .as_ref()
        .ok_or_else(|| AppError::NotFound("medication reference DB not installed".to_string()))?;

    lookup_medication_reference(conn, args)
}

fn truncate_reference_text(value: Option<String>, limit: usize) -> Option<String> {
    value.map(|text| {
        let mut chars = text.chars();
        let shortened: String = chars.by_ref().take(limit).collect();
        if chars.next().is_some() {
            format!("{shortened}…")
        } else {
            shortened
        }
    })
}

fn lookup_medication_reference(conn: &Connection, args: &Value) -> Result<Value, AppError> {
    let query = sanitize_for_prompt(str_arg(args, "query")?);
    let mut matches = crate::medication_reference::search_substance_details(conn, &query, 3)?;
    let best_match = matches.first_mut().map(|detail| {
        detail.trade_names.truncate(5);
        detail.indication = truncate_reference_text(detail.indication.take(), 650);
        detail.side_effects = truncate_reference_text(detail.side_effects.take(), 900);
        detail.contraindications = truncate_reference_text(detail.contraindications.take(), 650);
        detail.clone()
    });
    let alternatives: Vec<_> = matches
        .into_iter()
        .skip(1)
        .map(|detail| {
            json!({
                "id": detail.id,
                "name_de": detail.name_de,
                "atc_code": detail.atc_code,
                "trade_names": detail.trade_names.into_iter().take(3).collect::<Vec<_>>(),
            })
        })
        .collect();

    Ok(json!({
        "best_match": best_match,
        "alternatives": alternatives,
        "note": "Text fields are bounded for model context. Search a more specific name for another full match.",
    }))
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

    // The tool-calling model only supplies the clinician's request and focus.
    // The record itself is loaded here, so the report never depends on how
    // much of it the router managed to paraphrase into a JSON argument.
    let focus = sanitize_for_prompt(opt_str_arg(args, "session_notes").unwrap_or_default());
    let patient_context =
        assemble_report_context(conn, engine, &patient_id, &rt, &focus, thinking_effort)?;
    // Same revision the agent conversation is keyed by, so a regenerated
    // report for this record reuses the resident prompt prefix.
    let record = crate::llm::context_cache::PatientRevision {
        revision: crate::llm::evidence::provenance::patient_revision(conn, &patient_id)?,
        patient_id: patient_id.clone(),
    };

    let content = crate::llm::generate_report_streaming_with_prompt(
        app,
        engine,
        rt,
        &patient_context,
        &focus,
        None,
        None,
        crate::llm::SYSTEM_PROMPT_DE,
        thinking_effort,
        Some(&record),
    )?;
    let content = crate::llm::sanitize::polish_document_text(&content);

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

/// Retrieval terms per report type. Retrieval is lexical when no query
/// embedding is available, so these name the sections each report draws on.
fn report_retrieval_query(report_type: &crate::llm::ReportType) -> &'static str {
    use crate::llm::ReportType;
    match report_type {
        ReportType::Befundbericht => {
            "Anamnese Vorgeschichte Anlass Vorstellung psychischer Befund Diagnose Beurteilung \
             Empfehlung Therapieplan Medikation"
        }
        ReportType::Verlaufsbericht => {
            "Verlauf Sitzung Therapie Veränderung Befund Medikation Verträglichkeit Nebenwirkung \
             Ziel Fortschritt Plan"
        }
        ReportType::Ueberweisungsschreiben => {
            "Überweisung Fragestellung Anamnese Befund Diagnose Verlauf Behandlung Medikation \
             Beurteilung"
        }
    }
}

/// Tokens reserved beyond `evidence::PROMPT_SCAFFOLD_TOKENS` for the report
/// instructions and quality rules, which are longer than a history question.
const REPORT_INSTRUCTION_TOKENS: usize = 1_536;

/// Build the report's patient context from the provenance-bearing evidence
/// layer: structured facts (diagnoses, current medication, scores, plans)
/// first, then verbatim excerpts retrieved for the report type and focus, all
/// sized to this model's context window. Replaces a hand-built summary that
/// kept only five sessions at 400 characters each.
fn assemble_report_context(
    conn: &Connection,
    engine: &Arc<LlmEngine>,
    patient_id: &str,
    report_type: &crate::llm::ReportType,
    focus: &str,
    thinking_effort: crate::llm::ThinkingEffort,
) -> Result<String, AppError> {
    report_evidence(
        conn,
        engine.as_ref(),
        engine.context_size(),
        patient_id,
        report_type,
        focus,
        thinking_effort,
    )
}

/// Model-independent core of [`assemble_report_context`], testable with a
/// heuristic token counter.
fn report_evidence(
    conn: &Connection,
    counter: &dyn crate::llm::evidence::TokenCounter,
    context_size: usize,
    patient_id: &str,
    report_type: &crate::llm::ReportType,
    focus: &str,
    thinking_effort: crate::llm::ThinkingEffort,
) -> Result<String, AppError> {
    use crate::llm::evidence;

    // Retrieval keeps only the first dozen query terms, so the clinician's
    // focus goes first and the generic report terms fill what is left.
    let question = format!("{focus} {}", report_retrieval_query(report_type));
    let budget = evidence::budget_for_context(context_size, thinking_effort.max_tokens())
        .saturating_sub(REPORT_INSTRUCTION_TOKENS + counter.count(focus));

    // Recent sessions are included verbatim regardless of retrieval, so a new
    // development (e.g. acute suicidality) cannot be missed because it shares
    // no terms with the query. They take at most a quarter of the budget.
    let recent = recent_session_baseline(conn, counter, patient_id, budget / 4)?;
    let budget = budget.saturating_sub(counter.count(&recent));

    let request =
        evidence::EvidenceRequest::new(patient_id, question.trim()).with_token_budget(budget);
    let assembled = evidence::assemble_patient_evidence(conn, &request, counter)?;
    log::info!(
        "write_report evidence: {} units, {} omitted, budget {budget} tokens",
        assembled.manifest.entries.len(),
        assembled.manifest.omitted.len()
    );
    Ok(if recent.is_empty() {
        assembled.evidence
    } else {
        format!("{recent}\n{}", assembled.evidence)
    })
}

/// Sessions always shown in full, newest first.
const RECENT_SESSION_BASELINE: u32 = 3;

/// The latest sessions verbatim, newest first, within `budget` tokens. A
/// session that does not fit whole is left out rather than cut mid-sentence.
fn recent_session_baseline(
    conn: &Connection,
    counter: &dyn crate::llm::evidence::TokenCounter,
    patient_id: &str,
    budget: usize,
) -> Result<String, AppError> {
    let sessions =
        session::list_sessions_for_patient(conn, patient_id, RECENT_SESSION_BASELINE, 0)?;
    let mut block = String::from("--- LETZTE SITZUNGEN (vollständig, neueste zuerst) ---\n");
    let header_tokens = counter.count(&block);
    let mut spent = header_tokens;
    let mut included = 0;
    for s in &sessions {
        let mut entry = format!(
            "{} ({}):",
            sanitize_for_prompt(&s.session_date),
            sanitize_for_prompt(&s.session_type)
        );
        if let Some(notes) = s.notes.as_deref().filter(|n| !n.trim().is_empty()) {
            entry.push_str(&format!(
                "\n{}",
                crate::llm::sanitize::sanitize_for_prompt_with_limit(notes, 60_000)
            ));
        }
        if let Some(summary) = s
            .clinical_summary
            .as_deref()
            .filter(|n| !n.trim().is_empty())
        {
            entry.push_str(&format!(
                "\nZusammenfassung: {}",
                sanitize_for_prompt(summary)
            ));
        }
        entry.push('\n');
        let cost = counter.count(&entry);
        if spent + cost > budget {
            break;
        }
        spent += cost;
        block.push_str(&entry);
        included += 1;
    }
    Ok(if included == 0 { String::new() } else { block })
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

    #[test]
    fn medication_lookup_is_bounded_and_serializes_grounding_fields() {
        let conn = Connection::open_in_memory().unwrap();
        crate::medication_reference::open_reference_db_for_tests(&conn).unwrap();
        conn.execute(
            "INSERT INTO substances (
                id, name_de, atc_code, trade_names, indication,
                side_effects, contraindications, source_version
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                "substance-1",
                "Beispielstoff",
                "N06AB00",
                r#"["Beispielmed"]"#,
                "I".repeat(2_000),
                "N".repeat(2_000),
                "K".repeat(2_000),
                "test-version"
            ],
        )
        .unwrap();

        let value = lookup_medication_reference(&conn, &json!({"query": "Beispielmed"})).unwrap();
        let serialized = value.to_string();

        assert!(serialized.len() < 4_000);
        assert_eq!(value["best_match"]["name_de"], "Beispielstoff");
        assert_eq!(value["best_match"]["source_version"], "test-version");
    }

    #[test]
    fn medication_lookup_requires_a_query() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(lookup_medication_reference(&conn, &json!({})).is_err());
    }

    #[test]
    fn report_context_carries_structured_facts_and_full_session_text() {
        use crate::llm::evidence::test_support::{seed_patient, TestVault};
        use crate::llm::evidence::HeuristicCounter;

        let vault = TestVault::new();
        let conn = vault.conn();
        let seeded = seed_patient(&conn, "756.1234.5678.97", "Erika", "Beispiel");
        let context = report_evidence(
            &conn,
            &HeuristicCounter,
            16_384,
            &seeded.patient_id,
            &crate::llm::ReportType::Verlaufsbericht,
            "Schlaf",
            crate::llm::ThinkingEffort::Medium,
        )
        .unwrap();

        for expected in ["F32.1", "Sertralin", "100 mg", "PHQ-9", "Schlafhygiene"] {
            assert!(context.contains(expected), "missing {expected}");
        }
        // Full session text, not a 400-character excerpt of the five latest.
        assert!(context.contains("Patientin möchte die Therapiefrequenz reduzieren"));
    }

    #[test]
    fn report_context_respects_a_small_budget() {
        use crate::llm::evidence::test_support::{seed_patient, TestVault};
        use crate::llm::evidence::{HeuristicCounter, TokenCounter};

        let vault = TestVault::new();
        let conn = vault.conn();
        let seeded = seed_patient(&conn, "756.1234.5678.97", "Erika", "Beispiel");
        let context = report_evidence(
            &conn,
            &HeuristicCounter,
            6_656,
            &seeded.patient_id,
            &crate::llm::ReportType::Befundbericht,
            "",
            crate::llm::ThinkingEffort::Medium,
        )
        .unwrap();
        // 6656 - 4096 completion - 768 scaffold - 1536 instructions = 256 tokens.
        assert!(HeuristicCounter.count(&context) <= 256);
    }

    /// Seed a patient whose record is far larger than any context window:
    /// 300 sessions of ~1,600 characters each (~480K characters).
    fn seed_oversized_record(conn: &Connection) -> String {
        use crate::llm::evidence::test_support::seed_patient;
        let seeded = seed_patient(conn, "756.1234.5678.97", "Erika", "Beispiel");
        let paragraph = "Patientin berichtet über wechselnde Stimmung, Schlafstörungen und \
            Grübeln am Abend. Sertralin 100 mg 1-0-0 wird gut vertragen. Keine Suizidalität. \
            Psychoedukation zu Schlafhygiene, Verhaltensaktivierung besprochen. ";
        for i in 0..300 {
            let date = (chrono::Utc::now() - chrono::Duration::days(400 - i))
                .format("%Y-%m-%d")
                .to_string();
            let notes = format!("Sitzung {i}. {}", paragraph.repeat(7));
            conn.execute(
                "INSERT INTO sessions (id, patient_id, session_date, session_type, \
                     duration_minutes, notes) VALUES (?1, ?2, ?3, 'Verlaufsgespräch', 50, ?4)",
                rusqlite::params![format!("bulk-{i}"), seeded.patient_id, date, notes],
            )
            .unwrap();
        }
        seeded.patient_id
    }

    /// Token count of the complete report request: system prompt with the
    /// grounding suffix plus the user turn the report generator builds.
    fn full_report_request_tokens(
        counter: &dyn crate::llm::evidence::TokenCounter,
        evidence: &str,
        focus: &str,
    ) -> usize {
        let system = crate::llm::harness::GenerationTask::Report
            .apply_to_system_prompt(crate::llm::SYSTEM_PROMPT_DE);
        let user = super::super::prompts::report_generation_prompt(
            crate::llm::ReportType::Ueberweisungsschreiben,
            evidence,
            focus,
            None,
            None,
        );
        // Chat-template turn markers add a few dozen tokens at most.
        counter.count(&system) + counter.count(&user) + 64
    }

    #[test]
    fn oversized_record_still_fits_the_context_with_completion_reserved() {
        use crate::llm::evidence::test_support::TestVault;
        use crate::llm::evidence::HeuristicCounter;

        let vault = TestVault::new();
        let conn = vault.conn();
        let patient_id = seed_oversized_record(&conn);
        for (context_size, effort) in [
            (16_384, crate::llm::ThinkingEffort::Medium),
            (16_384, crate::llm::ThinkingEffort::ExtraHigh),
            (32_768, crate::llm::ThinkingEffort::High),
        ] {
            let focus = "Bitte Verlauf der Schlafstörungen hervorheben.";
            let evidence = report_evidence(
                &conn,
                &HeuristicCounter,
                context_size,
                &patient_id,
                &crate::llm::ReportType::Ueberweisungsschreiben,
                focus,
                effort,
            )
            .unwrap();
            let request = full_report_request_tokens(&HeuristicCounter, &evidence, focus);
            assert!(
                request + effort.max_tokens() <= context_size,
                "{request} prompt + {} completion exceeds {context_size}",
                effort.max_tokens()
            );
            // The block is filled, not starved: the structured facts are present.
            assert!(evidence.contains("F32.1") && evidence.contains("Sertralin"));
        }
    }

    /// Same check with a real tokenizer. German clinical text tokenizes less
    /// efficiently than the heuristic assumes for some vocabularies, so the
    /// fixed instruction reserve must hold for the shipped models too.
    #[test]
    #[ignore = "requires a local GGUF model"]
    fn oversized_record_fits_with_a_real_tokenizer() {
        use crate::llm::evidence::test_support::TestVault;

        let path = std::env::var("RAMDOC_BENCH_MODEL").expect("RAMDOC_BENCH_MODEL is required");
        let engine = LlmEngine::load(std::path::PathBuf::from(path), "ctx-check".into()).unwrap();
        let vault = TestVault::new();
        let conn = vault.conn();
        let patient_id = seed_oversized_record(&conn);
        let context_size = engine.context_size();
        let effort = crate::llm::ThinkingEffort::Medium;
        let focus = "Bitte Verlauf der Schlafstörungen hervorheben.";
        let evidence = report_evidence(
            &conn,
            &engine,
            context_size,
            &patient_id,
            &crate::llm::ReportType::Ueberweisungsschreiben,
            focus,
            effort,
        )
        .unwrap();
        let instructions_only = full_report_request_tokens(&engine, "", focus);
        let request = full_report_request_tokens(&engine, &evidence, focus);
        println!(
            "context={context_size} instructions={instructions_only} \
             reserved={} evidence={} request={request} completion={}",
            crate::llm::evidence::PROMPT_SCAFFOLD_TOKENS + REPORT_INSTRUCTION_TOKENS,
            engine.count_tokens(&evidence),
            effort.max_tokens()
        );
        assert!(
            instructions_only
                <= crate::llm::evidence::PROMPT_SCAFFOLD_TOKENS + REPORT_INSTRUCTION_TOKENS
        );
        assert!(request + effort.max_tokens() <= context_size);
    }

    #[test]
    fn recent_session_is_included_even_without_matching_terms() {
        use crate::llm::evidence::test_support::{seed_patient, TestVault};
        use crate::llm::evidence::HeuristicCounter;

        let vault = TestVault::new();
        let conn = vault.conn();
        let seeded = seed_patient(&conn, "756.1234.5678.97", "Erika", "Beispiel");
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        conn.execute(
            "INSERT INTO sessions (id, patient_id, session_date, session_type, duration_minutes, \
                 notes) VALUES ('acute', ?1, ?2, 'Krisenintervention', 30, ?3)",
            rusqlite::params![
                seeded.patient_id,
                today,
                "Zunehmende Hoffnungslosigkeit, äussert konkrete Pläne. Notfallplan erstellt."
            ],
        )
        .unwrap();
        let context = report_evidence(
            &conn,
            &HeuristicCounter,
            16_384,
            &seeded.patient_id,
            &crate::llm::ReportType::Befundbericht,
            "",
            crate::llm::ThinkingEffort::Medium,
        )
        .unwrap();
        assert!(context.starts_with("--- LETZTE SITZUNGEN"));
        assert!(context.contains("äussert konkrete Pläne"));
    }

    #[test]
    fn clinician_focus_leads_the_retrieval_query() {
        // question_terms keeps 12 terms; the generic terms alone are 11.
        let focus = "Lithiumspiegel Nierenfunktion";
        let question = format!(
            "{focus} {}",
            report_retrieval_query(&crate::llm::ReportType::Befundbericht)
        );
        let terms = crate::llm::evidence::retrieve::question_terms(&question);
        assert!(terms.contains(&"lithiumspiegel".to_string()));
        assert!(terms.contains(&"nierenfunktion".to_string()));
    }
}
