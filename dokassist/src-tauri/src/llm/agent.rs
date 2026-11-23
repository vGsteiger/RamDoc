//! Agentic loop: the LLM can call tools (patient data, calendar, search, reports)
//! and converses with the user over multiple turns.

use super::context_cache::InferenceSession;
use super::engine::{AgentMessage, LlmEngine};
use super::harness::{self, GenerationTask};
use super::thinking::{self, ThinkingEffort};
use crate::database::DbPool;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;

/// Maximum tool-call iterations per agent turn before forcing a final answer.
const MAX_ITERATIONS: usize = 8;

/// Tokens budget for the summarization call itself.
const SUMMARY_MAX_TOKENS: usize = 512;
/// Rough estimate: 1 token ≈ 3 UTF-8 bytes (conservative for German medical text).
fn estimate_tokens(s: &str) -> usize {
    (s.len() / 3).max(1)
}

/// Whether the agent is scoped to a single patient or has global access.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentScope {
    Global,
    Patient { patient_id: String },
}

/// A single tool call parsed from the LLM output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

/// The router's typed interpretation of a probe completion.
///
/// Keeping this decision separate from the text parser gives a future
/// grammar-constrained/tool-router implementation one narrow replacement
/// point.  It deliberately does not change tool dispatch or scope checks.
#[derive(Debug, Clone, PartialEq)]
enum ToolDecision {
    Call(ToolCallRequest),
    Answer,
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
    /// Router benchmark telemetry is reported by the command after the
    /// blocking turn, never embedded in patient-facing chat history.
    pub router_probe_count: u64,
    pub router_probe_duration_ms: u64,
}

pub struct AgentTurnInput {
    pub inference_session: InferenceSession,
    pub scope: AgentScope,
    pub patient_context: Option<String>,
    pub history: Vec<AgentMessage>,
    pub user_message: String,
    pub thinking_effort: ThinkingEffort,
}

/// Build a scope-specific system prompt.
///
/// * In **patient scope** `list_patients` is omitted (it is blocked at tool
///   dispatch too, but hiding it from the model avoids wasted iterations).
/// * `patient_context` is pre-fetched patient JSON injected directly so the
///   model never needs to call `get_patient` just to learn who it is talking
///   about.
///
/// Stable content comes first so the KV cache can reuse it: persona, tools
/// and tool-call format are identical for every conversation in a scope, the
/// patient section changes per patient, and the date last.
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
    build_system_prompt_for_date(scope, patient_context, &today)
}

fn build_system_prompt_for_date(
    scope: &AgentScope,
    patient_context: Option<&str>,
    today: &str,
) -> String {
    let tools = match scope {
        AgentScope::Patient { .. } => concat!(
            "get_patient(patient_id: string) - Holt Patientendaten für eine gegebene ID.\n",
            "get_calendar_events(start?: string, end?: string, patient_id?: string) - Listet Therapiesitzungen. Datum im Format YYYY-MM-DD.\n",
            "create_calendar_event(patient_id: string, date: string, session_type: string, duration_minutes: number, notes?: string) - Erstellt einen Vorschlag für eine neue Therapiesitzung; dieser wird erst nach ausdrücklicher Bestätigung durch den Behandler gespeichert.\n",
            "list_diagnoses(patient_id: string) - Listet ICD-10-Diagnosen des Patienten.\n",
            "create_diagnosis(patient_id: string, icd10_code: string, description: string, diagnosed_date: string, status?: string, notes?: string) - Erstellt nur einen Diagnosevorschlag; Speichern erfordert ausdrückliche Bestätigung durch den Behandler. status: active | resolved | chronic.\n",
            "list_medications(patient_id: string) - Listet aktuelle Medikationen des Patienten.\n",
            "create_medication(patient_id: string, substance: string, dosage: string, frequency: string, start_date: string, notes?: string) - Erstellt nur einen Medikationsvorschlag; Speichern erfordert ausdrückliche Bestätigung durch den Behandler (start_date: YYYY-MM-DD).\n",
            "lookup_medication_reference(query: string) - Sucht nach Wirkstoff- oder Handelsnamen in der lokal installierten Swissmedic-AIPS-Referenz und gibt ATC-Code, Indikation, Nebenwirkungen und Kontraindikationen zurück. Nutze dieses Tool für Fragen zu einzelnen Medikamenten.\n",
            "compare_medications(current_substance_id: string, replacement_substance_id: string) - Vergleicht zwei Medikamente anhand ihrer Kompendium-IDs. Gibt detaillierte Informationen zu beiden Medikamenten zurück (Indikation, Nebenwirkungen, Kontraindikationen) für eine fundierte Entscheidungsfindung bei Medikamentenwechsel.\n",
            "draft_email(patient_id: string, recipient_email: string, subject: string, body: string) - Erstellt nur einen E-Mail-Vorschlag; Speichern als Entwurf erfordert ausdrückliche Bestätigung durch den Behandler. Falls die E-Mail-Adresse des Patienten fehlt (null), rufe dieses Tool NICHT auf – weise den Benutzer stattdessen darauf hin, die E-Mail-Adresse zuerst in den Patientendaten zu ergänzen.\n",
            "list_treatment_plans(patient_id: string) - Listet Behandlungspläne des Patienten.\n",
            "search(query: string) - Volltextsuche über Patienten, Diagnosen, Sitzungen und Berichte.\n",
            "search_literature(query: string) - Semantische Suche in Fachliteratur (z.B. Medikamentenrichtlinien, Behandlungsleitlinien). Gibt relevante Textausschnitte zurück.\n",
            "write_report(patient_id: string, report_type: string, session_notes?: string) - Generiert nur einen Berichtsvorschlag aus der Patientenakte, die automatisch geladen wird; Speichern erfordert ausdrückliche Bestätigung durch den Behandler. report_type: Befundbericht | Verlaufsbericht | Ueberweisungsschreiben. session_notes: nur das Anliegen oder neue Angaben des Behandlers, nicht die Akte abschreiben.",
        ),
        AgentScope::Global => concat!(
            "get_patient(patient_id: string) - Holt Patientendaten für eine gegebene ID.\n",
            "list_patients() - Listet alle Patienten auf.\n",
            "get_calendar_events(start?: string, end?: string, patient_id?: string) - Listet Therapiesitzungen. Datum im Format YYYY-MM-DD.\n",
            "create_calendar_event(patient_id: string, date: string, session_type: string, duration_minutes: number, notes?: string) - Erstellt einen Vorschlag für eine neue Therapiesitzung; dieser wird erst nach ausdrücklicher Bestätigung durch den Behandler gespeichert.\n",
            "list_diagnoses(patient_id: string) - Listet ICD-10-Diagnosen des Patienten.\n",
            "create_diagnosis(patient_id: string, icd10_code: string, description: string, diagnosed_date: string, status?: string, notes?: string) - Erstellt nur einen Diagnosevorschlag; Speichern erfordert ausdrückliche Bestätigung durch den Behandler. status: active | resolved | chronic.\n",
            "list_medications(patient_id: string) - Listet aktuelle Medikationen des Patienten.\n",
            "create_medication(patient_id: string, substance: string, dosage: string, frequency: string, start_date: string, notes?: string) - Erstellt nur einen Medikationsvorschlag; Speichern erfordert ausdrückliche Bestätigung durch den Behandler (start_date: YYYY-MM-DD).\n",
            "lookup_medication_reference(query: string) - Sucht nach Wirkstoff- oder Handelsnamen in der lokal installierten Swissmedic-AIPS-Referenz und gibt ATC-Code, Indikation, Nebenwirkungen und Kontraindikationen zurück. Nutze dieses Tool für Fragen zu einzelnen Medikamenten.\n",
            "compare_medications(current_substance_id: string, replacement_substance_id: string) - Vergleicht zwei Medikamente anhand ihrer Kompendium-IDs. Gibt detaillierte Informationen zu beiden Medikamenten zurück (Indikation, Nebenwirkungen, Kontraindikationen) für eine fundierte Entscheidungsfindung bei Medikamentenwechsel.\n",
            "draft_email(patient_id: string, recipient_email: string, subject: string, body: string) - Erstellt nur einen E-Mail-Vorschlag; Speichern als Entwurf erfordert ausdrückliche Bestätigung durch den Behandler. Falls die E-Mail-Adresse des Patienten fehlt (null), rufe dieses Tool NICHT auf – weise den Benutzer stattdessen darauf hin, die E-Mail-Adresse zuerst in den Patientendaten zu ergänzen.\n",
            "list_treatment_plans(patient_id: string) - Listet Behandlungspläne des Patienten.\n",
            "search(query: string) - Volltextsuche über Patienten, Diagnosen, Sitzungen und Berichte.\n",
            "search_literature(query: string) - Semantische Suche in Fachliteratur (z.B. Medikamentenrichtlinien, Behandlungsleitlinien). Gibt relevante Textausschnitte zurück.\n",
            "write_report(patient_id: string, report_type: string, session_notes?: string) - Generiert nur einen Berichtsvorschlag aus der Patientenakte, die automatisch geladen wird; Speichern erfordert ausdrückliche Bestätigung durch den Behandler. report_type: Befundbericht | Verlaufsbericht | Ueberweisungsschreiben. session_notes: nur das Anliegen oder neue Angaben des Behandlers, nicht die Akte abschreiben.",
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
         Du antwortest auf Deutsch und hast Zugriff auf folgende Tools:\n\n\
         <tools>\n{tools}\n</tools>\n\n\
         Um ein Tool aufzurufen, antworte AUSSCHLIESSLICH mit einem JSON-Block in folgendem Format (keine anderen Texte):\n\
         <tool_call>{{\"name\": \"tool_name\", \"args\": {{...}}}}</tool_call>\n\n\
         Wenn du die endgültige Antwort geben kannst (kein Tool mehr nötig), antworte direkt auf Deutsch ohne <tool_call>-Block.\n\
         WICHTIG: Gib bei tool_call-Antworten KEINEN anderen Text aus – nur den Block.{patient_section}\n\n\
         Heutiges Datum: {today}."
    )
}

/// The conversation with a per-call instruction attached to its final turn.
///
/// Probe and answer need different instructions (tool-call format vs.
/// grounding and effort). Putting them in the system message made the two
/// prompts diverge before the history, so neither could reuse the other's
/// KV cache for it. Attached to the final turn, they share the system prompt
/// and every earlier message. The final turn is always the user's message or
/// a tool result, both rendered as user turns, so the instruction is appended
/// to it rather than added as a second consecutive user message (which some
/// templates reject).
fn with_turn_instruction(history: &[AgentMessage], instruction: &str) -> Vec<AgentMessage> {
    let mut messages = history.to_vec();
    if instruction.is_empty() {
        return messages;
    }
    match messages.last_mut() {
        Some(last) if matches!(last.role.as_str(), "user" | "tool_result") => {
            last.content.push_str(instruction);
        }
        _ => messages.push(AgentMessage {
            role: "user".to_string(),
            content: instruction.trim_start().to_string(),
        }),
    }
    messages
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
    let output = harness::strip_think_blocks(output);
    let start = output.find("<tool_call>")?;
    let end = output.find("</tool_call>")?;
    if end <= start {
        return None;
    }
    let json = &output[start + "<tool_call>".len()..end].trim();
    serde_json::from_str(json).ok()
}

const TOOL_CALL_OPEN: &str = "<tool_call>";
const TOOL_CALL_CLOSE: &str = "</tool_call>";
const THINK_OPEN: &str = "<think>";
const THINK_CLOSE: &str = "</think>";

/// Streaming stop rule for the tool probe: `true` while the output so far can
/// still become a tool call, `false` as soon as it cannot.
///
/// A probe that answers directly is discarded and the answer is generated
/// again with the chat settings, so every token after the decision is wasted
/// (up to the probe's `max_tokens`). Reasoning (canonical `<think>` blocks) is
/// skipped because a tool call may follow it; the first visible text must be a
/// `<tool_call>` (optionally behind a Markdown code fence, which small models
/// tend to add). Generation stops once that call is closed.
///
/// `starts_in_think` is set when the formatted prompt already opened a
/// `<think>` block (some Qwen templates do), so the output begins mid-reasoning
/// and only carries the closing tag.
fn probe_may_still_call_tool(output: &str, starts_in_think: bool) -> bool {
    let mut rest = output;
    if starts_in_think {
        match rest.find(THINK_CLOSE) {
            Some(end) => rest = &rest[end + THINK_CLOSE.len()..],
            None => return true,
        }
    }
    loop {
        rest = rest.trim_start();
        if let Some(after_open) = rest.strip_prefix(THINK_OPEN) {
            match after_open.find(THINK_CLOSE) {
                Some(end) => rest = &after_open[end + THINK_CLOSE.len()..],
                // Still reasoning; a tool call may follow.
                None => return true,
            }
        } else if THINK_OPEN.starts_with(rest) {
            // Empty, or a partial `<think>` tag.
            return true;
        } else {
            break;
        }
    }

    if let Some(after_fence) = rest.strip_prefix("```") {
        // Allow an optional language tag (```json / ```xml) on the fence line.
        match after_fence.find('\n') {
            Some(newline) => {
                let tag = &after_fence[..newline];
                if !tag.trim().chars().all(|c| c.is_ascii_alphanumeric()) {
                    return false;
                }
                rest = after_fence[newline + 1..].trim_start();
            }
            None => {
                return after_fence
                    .trim()
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric())
                    && after_fence.len() <= 16;
            }
        }
    } else if "```".starts_with(rest) {
        return true;
    }

    if let Some(call_body) = rest.strip_prefix(TOOL_CALL_OPEN) {
        !call_body.contains(TOOL_CALL_CLOSE)
    } else {
        TOOL_CALL_OPEN.starts_with(rest)
    }
}

/// Closes a probe's reasoning block that ran out of budget.
const PROBE_THINK_CLOSE: &str = "\n</think>\n\n";
/// Tokens for the probe continuation after a force-closed reasoning block;
/// enough for one tool call.
const PROBE_RECOVERY_MAX_TOKENS: usize = 192;

/// Whether a finished probe stopped inside a `<think>` block, i.e. it spent
/// its budget reasoning and never reached a decision.
fn probe_ended_in_reasoning(output: &str, starts_in_think: bool) -> bool {
    let last_open = output.rfind(THINK_OPEN);
    match output.rfind(THINK_CLOSE) {
        Some(close) => last_open.is_some_and(|open| open > close),
        None => starts_in_think || last_open.is_some(),
    }
}

fn route_tool_decision(output: &str) -> ToolDecision {
    parse_tool_call(output)
        .map(ToolDecision::Call)
        .unwrap_or(ToolDecision::Answer)
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
        "Fasse die folgende Gesprächshistorie in maximal 3 Sätzen auf Deutsch zusammen. \
         Antworte NUR mit der Zusammenfassung, ohne Einleitung:\n\n{digest}"
    );

    let mut summary = String::new();
    if engine
        .generate_streaming(
            system_prompt,
            &summary_prompt,
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
    router_engine: Option<&Arc<LlmEngine>>,
    pool: &DbPool,
    input: AgentTurnInput,
) -> Result<AgentLoopResult, AppError> {
    let AgentTurnInput {
        inference_session,
        scope,
        patient_context,
        mut history,
        user_message,
        thinking_effort,
    } = input;
    let session_id = inference_session.conversation_id.clone();
    let system_prompt = build_system_prompt(&scope, patient_context.as_deref());
    let probe_profile = GenerationTask::ToolProbe.profile(thinking_effort);
    // The answer uses the writing model's family defaults; the probe sampler is
    // family-independent because the probe may run on a router model.
    let chat_profile = GenerationTask::Chat.profile_for(engine.sampler_family(), thinking_effort);
    // Probe and answer share `system_prompt`; their instructions go on the
    // final turn (see `with_turn_instruction`).
    let probe_instruction = GenerationTask::ToolProbe.system_suffix();
    let answer_instruction = {
        // Families with a native reasoning switch get it through the chat
        // format instead of a `<think>`-worded instruction.
        let effort = if engine.chat_family().uses_plain_think_tags() {
            chat_profile.effort.system_suffix()
        } else {
            ""
        };
        format!("{effort}{}", GenerationTask::Chat.system_suffix())
    };
    // Only used if the answer overflows the context and is continued as a
    // fresh single-turn prompt, which has no conversation to attach to.
    let answer_system = format!("{system_prompt}{answer_instruction}");

    history.push(AgentMessage {
        role: "user".to_string(),
        content: user_message.clone(),
    });

    let mut tool_calls_made: Vec<ExecutedToolCall> = Vec::new();
    let mut probe_engine = router_engine.unwrap_or(engine);
    let mut using_dedicated_router = router_engine.is_some();
    let mut router_probe_count = 0_u64;
    let mut router_probe_duration_ms = 0_u64;
    let summarize_threshold = probe_engine
        .context_size()
        .saturating_sub(probe_profile.max_tokens + chat_profile.max_tokens + 512);

    for iteration in 0..MAX_ITERATIONS {
        // Summarize history if it is getting too large to fit in the context window
        let formatted = engine.format_chat_history_with_effort(
            &system_prompt,
            &with_turn_instruction(&history, &answer_instruction),
            chat_profile.effort,
        )?;
        let estimated_tokens = estimate_tokens(&formatted);
        if estimated_tokens > summarize_threshold {
            log::warn!(
                "Agent: prompt ~{estimated_tokens} tokens nears context limit, summarizing history"
            );
            history = summarize_history(engine, &system_prompt, history);
        }

        // Reasoning is switched off for every family (the empty think block
        // for Qwen-style templates, the native switch for Gemma 4 / Harmony),
        // so the probe's small budget is not spent in a <think> block.
        let prompt = probe_engine.format_chat_history_reasoning_off(
            &system_prompt,
            &with_turn_instruction(&history, probe_instruction),
        )?;

        // Probe: collect output to check for tool call.
        // The probe stops as soon as its output can no longer be a tool call,
        // so a direct answer costs a few tokens instead of `max_tokens`.
        let starts_in_think = prompt.trim_end().ends_with(THINK_OPEN);
        let mut probe_output = String::new();
        let mut probe_tokens = 0_usize;
        let probe_started = std::time::Instant::now();
        let probe_result = probe_engine.generate_streaming_session_with_sampler(
            &inference_session,
            &prompt,
            probe_profile.max_tokens,
            probe_profile.sampler,
            |token| {
                probe_tokens += 1;
                probe_output.push_str(token);
                probe_may_still_call_tool(&probe_output, starts_in_think)
            },
        );
        let probe_result = probe_result.and_then(|()| {
            if !probe_ended_in_reasoning(&probe_output, starts_in_think) {
                return Ok(());
            }
            // The model reasoned despite the switch and hit the budget inside
            // the block. Treating that as "no tool needed" would answer
            // without the data the tool would fetch, so close the block and
            // let the model decide.
            log::warn!(
                "Agent iteration {iteration}: probe ended inside an unclosed <think> block \
                 after {probe_tokens} tokens; force-closing it"
            );
            probe_output.push_str(PROBE_THINK_CLOSE);
            let continued = format!(
                "{prompt}{}",
                probe_engine
                    .chat_family()
                    .native_assistant_prefix(&probe_output)
            );
            let mut visible = String::new();
            probe_engine.continue_streaming_session_with_sampler(
                &inference_session,
                &continued,
                PROBE_RECOVERY_MAX_TOKENS,
                probe_profile.sampler,
                |token| {
                    probe_tokens += 1;
                    visible.push_str(token);
                    probe_may_still_call_tool(&visible, false)
                },
            )?;
            probe_output.push_str(&visible);
            Ok(())
        });
        if let Err(error) = probe_result {
            if using_dedicated_router {
                log::warn!(
                    "Dedicated tool router probe failed; retrying with writing engine: {error}"
                );
                probe_engine = engine;
                using_dedicated_router = false;
                continue;
            }
            return Err(error);
        }
        router_probe_count = router_probe_count.saturating_add(1);
        router_probe_duration_ms = router_probe_duration_ms.saturating_add(
            probe_started
                .elapsed()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
        );

        let probe_trimmed = probe_output.trim();
        let decision = route_tool_decision(probe_trimmed);
        log::info!(
            "Agent iteration {iteration}: probe produced {probe_tokens} tokens ({})",
            if matches!(decision, ToolDecision::Call(_)) {
                "tool call"
            } else {
                "answer"
            }
        );

        if let ToolDecision::Call(call) = decision {
            log::info!(
                "Agent iteration {}: calling tool '{}'",
                iteration,
                call.name
            );

            let args_json = serde_json::to_string(&call.args).unwrap_or_default();

            let _ = app.emit(
                "agent-tool-started",
                serde_json::json!({
                    "name": call.name,
                    "args_json": args_json,
                    "session_id": session_id,
                }),
            );

            let result = {
                let conn = pool.conn()?;
                super::tools::dispatch_tool(&conn, app, engine, &scope, &call, thinking_effort)
            };

            // The full result is what the UI, persistence and draft versions
            // consume (a `write_report` proposal carries the whole report).
            // Only the model-facing copy in `history` is shortened, and that
            // structurally, so it stays valid JSON (#552).
            let result_json = match result {
                Ok(val) => val.to_string(),
                Err(e) => serde_json::json!({"error": e.to_string()}).to_string(),
            };
            let model_result_json = super::tool_result::model_facing(&result_json);

            let _ = app.emit(
                "agent-tool-called",
                serde_json::json!({
                    "name": call.name,
                    "args_json": args_json,
                    "result_json": result_json,
                    "session_id": session_id,
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
                content: format!("<tool_result>{}</tool_result>", model_result_json),
            });
        } else {
            // No tool call — this is the final answer. Stream it with the
            // user-selected think budget. The probe stays short so tool
            // selection is not spent on a long <think> block.
            let final_prompt = engine.format_chat_history_with_effort(
                &system_prompt,
                &with_turn_instruction(&history, &answer_instruction),
                chat_profile.effort,
            )?;
            let final_answer = thinking::generate_with_think_budget_from_prompt(
                engine,
                &answer_system,
                Some(&final_prompt),
                &user_message,
                chat_profile,
                Some(&inference_session),
                &|token| {
                    // Keep the legacy event for existing specialised surfaces,
                    // and provide the session-bound event for the main chat.
                    let _ = app.emit("agent-chunk", token);
                    let _ = app.emit(
                        "agent-chunk-session",
                        serde_json::json!({"session_id": session_id, "token": token}),
                    );
                },
            )?;

            return Ok(AgentLoopResult {
                final_answer,
                tool_calls_made,
                router_probe_count,
                router_probe_duration_ms,
            });
        }
    }

    // Exceeded MAX_ITERATIONS — force a final answer from accumulated history
    let final_prompt = engine.format_chat_history_with_effort(
        &system_prompt,
        &with_turn_instruction(&history, &answer_instruction),
        chat_profile.effort,
    )?;
    let final_answer = thinking::generate_with_think_budget_from_prompt(
        engine,
        &answer_system,
        Some(&final_prompt),
        &user_message,
        chat_profile,
        Some(&inference_session),
        &|token| {
            let _ = app.emit("agent-chunk", token);
            let _ = app.emit(
                "agent-chunk-session",
                serde_json::json!({"session_id": session_id, "token": token}),
            );
        },
    )?;
    Ok(AgentLoopResult {
        final_answer,
        tool_calls_made,
        router_probe_count,
        router_probe_duration_ms,
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

    #[test]
    fn test_parse_tool_call_after_think_block() {
        let output = "<think>need the record</think>\n<tool_call>{\"name\": \"get_patient\", \"args\": {\"patient_id\": \"p1\"}}</tool_call>";
        let call = parse_tool_call(output).unwrap();
        assert_eq!(call.name, "get_patient");
        assert_eq!(call.args["patient_id"], "p1");
    }

    /// Feed `output` to the stop rule one char at a time, as the stream would,
    /// and return the prefix at which generation stops (or all of it).
    fn probe_stop_point(output: &str) -> &str {
        stop_point_with(output, false)
    }

    fn stop_point_with(output: &str, starts_in_think: bool) -> &str {
        for (idx, ch) in output.char_indices() {
            let end = idx + ch.len_utf8();
            if !probe_may_still_call_tool(&output[..end], starts_in_think) {
                return &output[..end];
            }
        }
        output
    }

    #[test]
    fn probe_stops_on_the_first_char_of_a_direct_answer() {
        assert_eq!(
            probe_stop_point("Herr Muster hat nächste Woche keinen Termin."),
            "H"
        );
        assert_eq!(probe_stop_point("\n\n  Gerne."), "\n\n  G");
    }

    #[test]
    fn probe_runs_until_the_tool_call_is_closed() {
        let call = r#"<tool_call>{"name": "list_patients", "args": {}}</tool_call>"#;
        assert_eq!(probe_stop_point(&format!("{call}\nTrailing text")), call);
        assert!(parse_tool_call(probe_stop_point(call)).is_some());
    }

    #[test]
    fn probe_skips_reasoning_before_deciding() {
        let with_call = "<think>Ich brauche die Akte.</think>\n<tool_call>{\"name\": \"get_patient\", \"args\": {}}</tool_call>";
        assert_eq!(probe_stop_point(with_call), with_call);

        let with_answer = "<think>\n\n</think>\n\nDie Antwort";
        assert_eq!(probe_stop_point(with_answer), "<think>\n\n</think>\n\nD");
    }

    #[test]
    fn probe_handles_a_think_block_opened_by_the_prompt() {
        let with_call = "Ich brauche die Akte.\n</think>\n\n<tool_call>{\"name\": \"get_patient\", \"args\": {}}</tool_call>";
        assert_eq!(stop_point_with(with_call, true), with_call);
        assert!(parse_tool_call(with_call).is_some());

        assert_eq!(
            stop_point_with("Kurz überlegt.</think>Antwort", true),
            "Kurz überlegt.</think>A"
        );
    }

    #[test]
    fn probe_detects_a_budget_spent_inside_reasoning() {
        assert!(probe_ended_in_reasoning("<think>Ich überlege noch", false));
        assert!(probe_ended_in_reasoning("Ich überlege noch", true));
        assert!(probe_ended_in_reasoning(
            "<think>a</think>\n<think>b",
            false
        ));
        assert!(!probe_ended_in_reasoning("<think>a</think>Antwort", false));
        assert!(!probe_ended_in_reasoning("a</think><tool_call>", true));
        assert!(!probe_ended_in_reasoning("Antwort", false));
        assert!(!probe_ended_in_reasoning("", false));
    }

    #[test]
    fn probe_tolerates_a_code_fence_around_the_call() {
        let fenced =
            "```json\n<tool_call>{\"name\": \"list_patients\", \"args\": {}}</tool_call>\n```";
        let stopped = probe_stop_point(fenced);
        assert!(stopped.ends_with("</tool_call>"));
        assert!(parse_tool_call(stopped).is_some());

        // A fence that opens ordinary content is still an answer.
        assert_eq!(probe_stop_point("```\nfoo"), "```\nf");
        assert_eq!(probe_stop_point("``` ja klar"), "``` ja k");
    }

    #[test]
    fn probe_rejects_text_that_diverges_from_the_tool_tag() {
        assert_eq!(probe_stop_point("<b>Hinweis</b>"), "<b");
        assert_eq!(probe_stop_point("<tool>"), "<tool>");
    }

    fn message(role: &str, content: &str) -> AgentMessage {
        AgentMessage {
            role: role.into(),
            content: content.into(),
        }
    }

    #[test]
    fn system_prompt_is_stable_across_dates_up_to_the_date_line() {
        let scope = AgentScope::Patient {
            patient_id: "p1".into(),
        };
        let monday = build_system_prompt_for_date(&scope, Some("{\"name\":\"A\"}"), "2026-11-23");
        let tuesday = build_system_prompt_for_date(&scope, Some("{\"name\":\"A\"}"), "2026-11-24");
        let date_line = monday.rfind("Heutiges Datum").unwrap();
        assert_eq!(monday[..date_line], tuesday[..date_line]);
        // Tools and the patient section both precede the date.
        assert!(monday.find("<tools>").unwrap() < date_line);
        assert!(monday.find("Patientendaten").unwrap() < date_line);
        assert!(monday.trim_end().ends_with("Heutiges Datum: 2026-11-23."));
    }

    #[test]
    fn probe_and_answer_share_everything_but_the_final_instruction() {
        let history = vec![
            message("user", "Wie geht es Frau A?"),
            message("assistant", "Stabil."),
            message("user", "Welche Dosis Sertralin?"),
        ];
        let probe = with_turn_instruction(&history, GenerationTask::ToolProbe.system_suffix());
        let answer = with_turn_instruction(&history, GenerationTask::Chat.system_suffix());

        assert_eq!(probe.len(), history.len());
        assert_eq!(probe[..2], answer[..2]);
        assert_eq!(probe[..2], history[..2]);
        for messages in [&probe, &answer] {
            assert!(messages[2].content.starts_with("Welche Dosis Sertralin?"));
        }
        assert_ne!(probe[2].content, answer[2].content);
    }

    #[test]
    fn turn_instruction_follows_a_tool_result() {
        let history = vec![
            message("user", "Termine?"),
            message("tool_call", "<tool_call>{}</tool_call>"),
            message("tool_result", "<tool_result>[]</tool_result>"),
        ];
        let messages = with_turn_instruction(&history, "\n\nHinweis.");
        assert_eq!(messages.len(), 3);
        assert_eq!(
            messages[2].content,
            "<tool_result>[]</tool_result>\n\nHinweis."
        );
        assert_eq!(with_turn_instruction(&history, ""), history);
    }

    #[test]
    fn tool_router_keeps_a_non_tool_probe_as_an_answer() {
        assert_eq!(route_tool_decision("Direkte Antwort"), ToolDecision::Answer);
    }

    #[test]
    fn tool_router_returns_a_typed_call() {
        let decision =
            route_tool_decision(r#"<tool_call>{"name":"list_patients","args":{}}</tool_call>"#);
        assert!(matches!(decision, ToolDecision::Call(call) if call.name == "list_patients"));
    }
}
