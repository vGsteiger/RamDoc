//! Agentic loop: the LLM can call tools (patient data, calendar, search, reports)
//! and converses with the user over multiple turns.

use super::engine::{format_chatml_history, AgentMessage, LlmEngine};
use crate::database::DbPool;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;

/// Maximum tool-call iterations per agent turn before forcing a final answer.
const MAX_ITERATIONS: usize = 8;
/// Tokens budget for the "is there a tool call?" probe.
const PROBE_MAX_TOKENS: usize = 512;
/// Tokens budget for the final streaming answer.
const ANSWER_MAX_TOKENS: usize = 2048;
/// Temperature for the probe (deterministic tool selection).
const PROBE_TEMP: f32 = 0.1;
/// Temperature for the final answer (more creative).
const ANSWER_TEMP: f32 = 0.7;
/// Maximum chars of a tool result that are fed back into the LLM.
const TOOL_RESULT_TRIM: usize = 4_000;
/// Token context window (must match engine.rs N_CTX).
const N_CTX: usize = 4096;
/// Trigger summarization when estimated prompt tokens exceed this threshold.
/// Leaves headroom for PROBE_MAX_TOKENS + ANSWER_MAX_TOKENS.
const SUMMARIZE_THRESHOLD: usize = 1024;
/// Tokens budget for the summarization call itself.
const SUMMARY_MAX_TOKENS: usize = 512;
/// Rough estimate: 1 token ≈ 4 UTF-8 bytes.
fn estimate_tokens(s: &str) -> usize {
    (s.len() / 4).max(1)
}

/// Whether the agent is scoped to a single patient or has global access.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentScope {
    Global,
    Patient { patient_id: String },
}

/// A single tool call parsed from the LLM output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

/// Record of a tool call that was actually executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedToolCall {
    pub name: String,
    pub args_json: String,
    pub result_json: String,
}

/// Returned from `run_agent_loop` after a complete agent turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLoopResult {
    pub final_answer: String,
    pub tool_calls_made: Vec<ExecutedToolCall>,
}

/// Build a scope-specific system prompt.
///
/// * In **patient scope** `list_patients` is omitted (it is blocked at tool
///   dispatch too, but hiding it from the model avoids wasted iterations).
/// * `patient_context` is pre-fetched patient JSON injected directly so the
///   model never needs to call `get_patient` just to learn who it is talking
///   about.
fn build_system_prompt(scope: &AgentScope, patient_context: Option<&str>) -> String {
    let today = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Compute YYYY-MM-DD from Unix timestamp (no external crate needed)
        let days_since_epoch = now / 86400;
        let (y, m, d) = days_from_epoch(days_since_epoch);
        format!("{y:04}-{m:02}-{d:02}")
    };

    let tools = match scope {
        AgentScope::Patient { .. } => concat!(
            "get_patient(patient_id: string) - Holt Patientendaten für eine gegebene ID.\n",
            "get_calendar_events(start?: string, end?: string, patient_id?: string) - Listet Therapiesitzungen. Datum im Format YYYY-MM-DD.\n",
            "create_calendar_event(patient_id: string, date: string, session_type: string, duration_minutes: number, notes?: string) - Erstellt eine neue Therapiesitzung.\n",
            "list_diagnoses(patient_id: string) - Listet ICD-10-Diagnosen des Patienten.\n",
            "create_diagnosis(patient_id: string, icd10_code: string, description: string, diagnosed_date: string, status?: string, notes?: string) - Erstellt eine neue Diagnose. status: active | resolved | chronic.\n",
            "list_medications(patient_id: string) - Listet aktuelle Medikationen des Patienten.\n",
            "create_medication(patient_id: string, substance: string, dosage: string, frequency: string, start_date: string, notes?: string) - Dokumentiert ein Medikament (start_date: YYYY-MM-DD).\n",
            "draft_email(patient_id: string, recipient_email: string, subject: string, body: string) - Erstellt einen E-Mail-Entwurf (Status: draft, wird nicht gesendet). Falls die E-Mail-Adresse des Patienten fehlt (null), rufe dieses Tool NICHT auf – weise den Benutzer stattdessen darauf hin, die E-Mail-Adresse zuerst in den Patientendaten zu ergänzen.\n",
            "list_treatment_plans(patient_id: string) - Listet Behandlungspläne des Patienten.\n",
            "search(query: string) - Volltextsuche über Patienten, Diagnosen, Sitzungen und Berichte.\n",
            "search_literature(query: string) - Semantische Suche in Fachliteratur (z.B. Medikamentenrichtlinien, Behandlungsleitlinien). Gibt relevante Textausschnitte zurück.\n",
            "write_report(patient_id: string, report_type: string, session_notes: string) - Generiert einen klinischen Bericht. report_type: Befundbericht | Verlaufsbericht | Ueberweisungsschreiben.",
        ),
        AgentScope::Global => concat!(
            "get_patient(patient_id: string) - Holt Patientendaten für eine gegebene ID.\n",
            "list_patients() - Listet alle Patienten auf.\n",
            "get_calendar_events(start?: string, end?: string, patient_id?: string) - Listet Therapiesitzungen. Datum im Format YYYY-MM-DD.\n",
            "create_calendar_event(patient_id: string, date: string, session_type: string, duration_minutes: number, notes?: string) - Erstellt eine neue Therapiesitzung.\n",
            "list_diagnoses(patient_id: string) - Listet ICD-10-Diagnosen des Patienten.\n",
            "create_diagnosis(patient_id: string, icd10_code: string, description: string, diagnosed_date: string, status?: string, notes?: string) - Erstellt eine neue Diagnose. status: active | resolved | chronic.\n",
            "list_medications(patient_id: string) - Listet aktuelle Medikationen des Patienten.\n",
            "create_medication(patient_id: string, substance: string, dosage: string, frequency: string, start_date: string, notes?: string) - Dokumentiert ein Medikament (start_date: YYYY-MM-DD).\n",
            "draft_email(patient_id: string, recipient_email: string, subject: string, body: string) - Erstellt einen E-Mail-Entwurf (Status: draft, wird nicht gesendet). Falls die E-Mail-Adresse des Patienten fehlt (null), rufe dieses Tool NICHT auf – weise den Benutzer stattdessen darauf hin, die E-Mail-Adresse zuerst in den Patientendaten zu ergänzen.\n",
            "list_treatment_plans(patient_id: string) - Listet Behandlungspläne des Patienten.\n",
            "search(query: string) - Volltextsuche über Patienten, Diagnosen, Sitzungen und Berichte.\n",
            "search_literature(query: string) - Semantische Suche in Fachliteratur (z.B. Medikamentenrichtlinien, Behandlungsleitlinien). Gibt relevante Textausschnitte zurück.\n",
            "write_report(patient_id: string, report_type: string, session_notes: string) - Generiert einen klinischen Bericht. report_type: Befundbericht | Verlaufsbericht | Ueberweisungsschreiben.",
        ),
    };

    let patient_section = match (scope, patient_context) {
        (AgentScope::Patient { patient_id }, Some(ctx)) => format!(
            "\n\nDu arbeitest im Kontext von Patient-ID: {patient_id}.\n\
             Aktuelle Patientendaten (bereits geladen – kein get_patient-Aufruf nötig):\n{ctx}",
        ),
        (AgentScope::Patient { patient_id }, None) => {
            format!("\n\nDu arbeitest im Kontext von Patient-ID: {patient_id}.")
        }
        _ => String::new(),
    };

    format!(
        "Du bist DokAssist, ein hilfreicher medizinischer Assistent für psychiatrische Praxen.\n\
         Heutiges Datum: {today}.\n\
         Du antwortest auf Deutsch und hast Zugriff auf folgende Tools:\n\n\
         <tools>\n{tools}\n</tools>\n\n\
         Um ein Tool aufzurufen, antworte AUSSCHLIESSLICH mit einem JSON-Block in folgendem Format (keine anderen Texte):\n\
         <tool_call>{{\"name\": \"tool_name\", \"args\": {{...}}}}</tool_call>\n\n\
         Wenn du die endgültige Antwort geben kannst (kein Tool mehr nötig), antworte direkt auf Deutsch ohne <tool_call>-Block.\n\
         WICHTIG: Gib bei tool_call-Antworten KEINEN anderen Text aus – nur den Block.{patient_section}"
    )
}

/// Convert days since Unix epoch (1970-01-01) to (year, month, day).
fn days_from_epoch(days: u64) -> (u64, u8, u8) {
    // Algorithm: civil date from days (Howard Hinnant, public domain)
    let z = days as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u64, m as u8, d as u8)
}

/// Try to extract a `<tool_call>{...}</tool_call>` block from the LLM output.
fn parse_tool_call(output: &str) -> Option<ToolCallRequest> {
    let start = output.find("<tool_call>")?;
    let end = output.find("</tool_call>")?;
    if end <= start {
        return None;
    }
    let json = &output[start + "<tool_call>".len()..end].trim();
    serde_json::from_str(json).ok()
}

/// Summarize the middle of the history to keep prompt size manageable.
///
/// Keeps the first message (original user request) and the last two messages
/// (most recent context), replacing everything in between with an LLM-generated
/// summary.  Returns `history` unchanged on any error.
fn summarize_history(
    engine: &Arc<LlmEngine>,
    system_prompt: &str,
    mut history: Vec<AgentMessage>,
) -> Vec<AgentMessage> {
    if history.len() <= 3 {
        return history;
    }

    // Build a plain-text digest of the middle messages to summarize
    let middle: Vec<_> = history[1..history.len() - 2].iter().collect();
    let digest = middle
        .iter()
        .map(|m| format!("[{}]: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let summary_prompt = format!(
        "{system_prompt}\n\n\
         Fasse die folgende Gesprächshistorie in maximal 3 Sätzen auf Deutsch zusammen. \
         Antworte NUR mit der Zusammenfassung, ohne Einleitung:\n\n{digest}"
    );

    let mut summary = String::new();
    if engine
        .generate_streaming_raw(
            &format!("<|im_start|>system\n{summary_prompt}<|im_end|>\n<|im_start|>assistant\n"),
            SUMMARY_MAX_TOKENS,
            0.3,
            |token| {
                summary.push_str(token);
                true
            },
        )
        .is_err()
    {
        return history;
    }

    let summary = summary.trim().to_string();
    if summary.is_empty() {
        return history;
    }

    log::info!("Agent: summarized {} history messages", middle.len());

    // Rebuild: first message + summary placeholder + last 2 messages
    let last_two = history.split_off(history.len() - 2);
    history.truncate(1);
    history.push(AgentMessage {
        role: "assistant".to_string(),
        content: format!("[Zusammenfassung vorheriger Schritte]: {summary}"),
    });
    history.extend(last_two);
    history
}

/// Run a full agent turn: append user message, loop until final answer.
/// Streams the final answer token-by-token via `agent-chunk` events.
///
/// `patient_context` should be a pre-serialised JSON string of the patient
/// record when `scope` is `Patient`.  It is injected into the system prompt so
/// the model never needs a wasted `get_patient` call just to learn who it is
/// talking about.
pub fn run_agent_loop(
    app: &tauri::AppHandle,
    engine: &Arc<LlmEngine>,
    pool: &DbPool,
    scope: AgentScope,
    patient_context: Option<String>,
    mut history: Vec<AgentMessage>,
    user_message: String,
) -> Result<AgentLoopResult, AppError> {
    let system_prompt = build_system_prompt(&scope, patient_context.as_deref());

    history.push(AgentMessage {
        role: "user".to_string(),
        content: user_message,
    });

    let mut tool_calls_made: Vec<ExecutedToolCall> = Vec::new();

    for iteration in 0..MAX_ITERATIONS {
        // Summarize history if it is getting too large to fit in the context window
        let estimated_tokens = estimate_tokens(&format_chatml_history(&system_prompt, &history));
        if estimated_tokens > SUMMARIZE_THRESHOLD {
            log::warn!(
                "Agent: prompt ~{estimated_tokens} tokens nears context limit, summarizing history"
            );
            history = summarize_history(engine, &system_prompt, history);
        }

        let prompt = format_chatml_history(&system_prompt, &history);

        // Probe: collect output to check for tool call
        let mut probe_output = String::new();
        engine.generate_streaming_raw(&prompt, PROBE_MAX_TOKENS, PROBE_TEMP, |token| {
            probe_output.push_str(token);
            // Stop early if we see the closing tag
            !probe_output.contains("</tool_call>")
                || !probe_output.contains("\n\n") && probe_output.len() < PROBE_MAX_TOKENS * 4
        })?;

        let probe_trimmed = probe_output.trim();

        if let Some(call) = parse_tool_call(probe_trimmed) {
            log::info!(
                "Agent iteration {}: calling tool '{}'",
                iteration,
                call.name
            );

            let args_json = serde_json::to_string(&call.args).unwrap_or_default();

            let result = {
                let conn = pool.conn()?;
                super::tools::dispatch_tool(&conn, app, engine, &scope, &call)
            };

            let result_json = match result {
                Ok(val) => {
                    let s = val.to_string();
                    if s.len() > TOOL_RESULT_TRIM {
                        s[..TOOL_RESULT_TRIM].to_string()
                    } else {
                        s
                    }
                }
                Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
            };

            let _ = app.emit(
                "agent-tool-called",
                serde_json::json!({
                    "name": call.name,
                    "args_json": args_json,
                    "result_json": result_json,
                }),
            );

            tool_calls_made.push(ExecutedToolCall {
                name: call.name.clone(),
                args_json: args_json.clone(),
                result_json: result_json.clone(),
            });

            // Append tool_call and tool_result to history
            history.push(AgentMessage {
                role: "tool_call".to_string(),
                content: format!(
                    "<tool_call>{}</tool_call>",
                    serde_json::json!({"name": call.name, "args": call.args})
                ),
            });
            history.push(AgentMessage {
                role: "tool_result".to_string(),
                content: format!("<tool_result>{}</tool_result>", result_json),
            });
        } else {
            // No tool call — this is the final answer. Stream it.
            // The probe already has partial output; start fresh for full answer.
            let final_prompt = format_chatml_history(&system_prompt, &history);
            let mut final_answer = String::new();

            engine.generate_streaming_raw(
                &final_prompt,
                ANSWER_MAX_TOKENS,
                ANSWER_TEMP,
                |token| {
                    final_answer.push_str(token);
                    let _ = app.emit("agent-chunk", token);
                    true
                },
            )?;

            let _ = app.emit(
                "agent-done",
                serde_json::json!({"final_answer": final_answer}),
            );

            return Ok(AgentLoopResult {
                final_answer,
                tool_calls_made,
            });
        }
    }

    // Exceeded MAX_ITERATIONS — force a final answer from accumulated history
    let final_prompt = format_chatml_history(&system_prompt, &history);
    let mut final_answer = String::new();
    engine.generate_streaming_raw(&final_prompt, ANSWER_MAX_TOKENS, ANSWER_TEMP, |token| {
        final_answer.push_str(token);
        let _ = app.emit("agent-chunk", token);
        true
    })?;
    let _ = app.emit(
        "agent-done",
        serde_json::json!({"final_answer": final_answer}),
    );
    Ok(AgentLoopResult {
        final_answer,
        tool_calls_made,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_call_valid() {
        let output = r#"<tool_call>{"name": "list_patients", "args": {}}</tool_call>"#;
        let call = parse_tool_call(output).unwrap();
        assert_eq!(call.name, "list_patients");
    }

    #[test]
    fn test_parse_tool_call_with_args() {
        let output =
            r#"<tool_call>{"name": "get_patient", "args": {"patient_id": "abc123"}}</tool_call>"#;
        let call = parse_tool_call(output).unwrap();
        assert_eq!(call.name, "get_patient");
        assert_eq!(call.args["patient_id"], "abc123");
    }

    #[test]
    fn test_parse_tool_call_none_when_absent() {
        let output = "Hier ist meine Antwort ohne Tool-Aufruf.";
        assert!(parse_tool_call(output).is_none());
    }

    #[test]
    fn test_parse_tool_call_malformed_json() {
        let output = "<tool_call>{invalid json}</tool_call>";
        assert!(parse_tool_call(output).is_none());
    }
}
