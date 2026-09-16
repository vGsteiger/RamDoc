use super::{
    engine::{AgentMessage, LlmEngine},
    prompts::{self, LetterType, ReportType},
    sanitize::{build_delimited_prompt, sanitize_for_prompt},
    thinking::{self, ThinkingEffort},
    utf8,
};
use crate::error::AppError;
use tauri::Emitter;

/// Max tokens for the condensed-context summary output.
const SUMMARIZE_MAX_TOKENS: usize = 800;

/// Char limits for truncating inputs before the summarizer pass (prevents summarizer overflow).
const MAX_CONTEXT_CHARS: usize = 16_000;
const MAX_NOTES_CHARS: usize = 6_000;

/// Returns `true` when the model-formatted prompt would leave insufficient
/// output headroom in the active context window.
fn needs_summarization(
    engine: &LlmEngine,
    system_prompt: &str,
    patient_context: &str,
    session_notes: &str,
) -> Result<bool, AppError> {
    let message = AgentMessage {
        role: "user".to_string(),
        content: format!(
            "Patientenkontext:\n{patient_context}\n\nSitzungsnotizen:\n{session_notes}"
        ),
    };
    let formatted = engine.format_chat_history(system_prompt, &[message])?;
    let max_input_tokens = engine.context_size().saturating_sub(4_096 + 256);
    Ok(engine.count_tokens(&formatted) > max_input_tokens)
}

/// Condenses `patient_context` + `session_notes` into a shorter summary string
/// that fits within the context window.
fn run_summarization(
    engine: &LlmEngine,
    system_prompt: &str,
    patient_context: &str,
    session_notes: &str,
) -> Result<String, AppError> {
    let ctx = utf8::truncate_to_boundary(patient_context, MAX_CONTEXT_CHARS);
    let notes = utf8::truncate_to_boundary(session_notes, MAX_NOTES_CHARS);
    let summarization_msg = prompts::context_summarization_prompt(ctx, notes);
    engine.generate(system_prompt, &summarization_msg, SUMMARIZE_MAX_TOKENS, 0.3)
}

/// Generate a report using the built-in system prompt.
pub fn generate_report_streaming(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    report_type: ReportType,
    patient_context: &str,
    session_notes: &str,
) -> Result<String, AppError> {
    generate_report_streaming_with_prompt(
        app,
        engine,
        report_type,
        patient_context,
        session_notes,
        None,
        None,
        prompts::SYSTEM_PROMPT_DE,
        ThinkingEffort::Medium,
    )
}

/// Generate a report using a caller-supplied system prompt.
/// Emits `"report-chunk"` Tauri events for each token as it is produced.
/// If inputs are too long, emits `"report-summarizing"` then condenses them first.
/// Returns the full completed report string.
#[allow(clippy::too_many_arguments)]
pub fn generate_report_streaming_with_prompt(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    report_type: ReportType,
    patient_context: &str,
    session_notes: &str,
    additional_context: Option<&str>,
    instructions: Option<&str>,
    system_prompt: &str,
    thinking_effort: ThinkingEffort,
) -> Result<String, AppError> {
    let summary_opt = if needs_summarization(engine, system_prompt, patient_context, session_notes)?
    {
        let _ = app.emit("report-summarizing", ());
        Some(run_summarization(
            engine,
            system_prompt,
            patient_context,
            session_notes,
        )?)
    } else {
        None
    };
    let (eff_ctx, eff_notes) = match &summary_opt {
        Some(s) => (s.as_str(), ""),
        None => (patient_context, session_notes),
    };

    let user_message = prompts::report_generation_prompt(
        report_type,
        eff_ctx,
        eff_notes,
        additional_context,
        instructions,
    );

    thinking::generate_with_think_budget(
        engine,
        system_prompt,
        &user_message,
        thinking_effort,
        0.7,
        &|token| {
            let _ = app.emit("report-chunk", token);
        },
    )
}

/// Improve text based on provided instruction using the built-in system prompt.
pub fn improve_text_streaming(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    text: &str,
    instruction: &str,
) -> Result<String, AppError> {
    improve_text_streaming_with_prompt(
        app,
        engine,
        text,
        instruction,
        prompts::SYSTEM_PROMPT_DE,
        ThinkingEffort::Medium,
    )
}

/// Improve text based on provided instruction using a caller-supplied system prompt.
/// Emits `"text-improvement-chunk"` Tauri events for each token as it is produced.
/// Returns the full improved text string.
pub fn improve_text_streaming_with_prompt(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    text: &str,
    instruction: &str,
    system_prompt: &str,
    thinking_effort: ThinkingEffort,
) -> Result<String, AppError> {
    let safe_text = sanitize_for_prompt(text);
    let safe_instruction = sanitize_for_prompt(instruction);
    let user_message = build_delimited_prompt(&safe_instruction, &safe_text);

    thinking::generate_with_think_budget(
        engine,
        system_prompt,
        &user_message,
        thinking_effort,
        0.7,
        &|token| {
            let _ = app.emit("text-improvement-chunk", token);
        },
    )
}

/// Generate a session summary using the built-in system prompt.
pub fn generate_session_summary_streaming(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    patient_context: &str,
    session_notes: &str,
) -> Result<String, AppError> {
    generate_session_summary_streaming_with_prompt(
        app,
        engine,
        patient_context,
        session_notes,
        prompts::SYSTEM_PROMPT_DE,
        ThinkingEffort::Medium,
    )
}

/// Generate a session summary using a caller-supplied system prompt.
/// Emits `"session-summary-chunk"` Tauri events for each token as it is produced.
/// If inputs are too long, emits `"session-summary-summarizing"` then condenses them first.
/// Returns the full completed session summary string.
pub fn generate_session_summary_streaming_with_prompt(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    patient_context: &str,
    session_notes: &str,
    system_prompt: &str,
    thinking_effort: ThinkingEffort,
) -> Result<String, AppError> {
    let summary_opt = if needs_summarization(engine, system_prompt, patient_context, session_notes)?
    {
        let _ = app.emit("session-summary-summarizing", ());
        Some(run_summarization(
            engine,
            system_prompt,
            patient_context,
            session_notes,
        )?)
    } else {
        None
    };
    let (eff_ctx, eff_notes) = match &summary_opt {
        Some(s) => (s.as_str(), ""),
        None => (patient_context, session_notes),
    };

    let user_message = prompts::session_summary_prompt(eff_ctx, eff_notes);

    thinking::generate_with_think_budget(
        engine,
        system_prompt,
        &user_message,
        thinking_effort,
        0.7,
        &|token| {
            let _ = app.emit("session-summary-chunk", token);
        },
    )
}

/// Generate a letter using a caller-supplied system prompt.
/// Emits `"letter-chunk"` Tauri events for each token as it is produced.
/// If inputs are too long, emits `"letter-summarizing"` then condenses them first.
/// Returns the full completed letter string.
#[allow(clippy::too_many_arguments)]
pub fn generate_letter_streaming_with_prompt(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    letter_type: LetterType,
    language: &str,
    patient_context: &str,
    clinical_summary: &str,
    recipient_name: Option<&str>,
    system_prompt: &str,
    thinking_effort: ThinkingEffort,
) -> Result<String, AppError> {
    let summary_opt =
        if needs_summarization(engine, system_prompt, patient_context, clinical_summary)? {
            let _ = app.emit("letter-summarizing", ());
            Some(run_summarization(
                engine,
                system_prompt,
                patient_context,
                clinical_summary,
            )?)
        } else {
            None
        };
    let (eff_ctx, eff_summary) = match &summary_opt {
        Some(s) => (s.as_str(), ""),
        None => (patient_context, clinical_summary),
    };

    let user_message = prompts::letter_generation_prompt(
        letter_type,
        language,
        eff_ctx,
        eff_summary,
        recipient_name,
    );

    thinking::generate_with_think_budget(
        engine,
        system_prompt,
        &user_message,
        thinking_effort,
        0.7,
        &|token| {
            let _ = app.emit("letter-chunk", token);
        },
    )
}

/// Answer a patient-history question from an assembled evidence block.
///
/// No summarisation pass is needed or wanted here: `llm::evidence` already fit
/// the evidence into the model's budget, and condensing it would be exactly the
/// lossy step the evidence layer exists to avoid. Emits
/// `"patient-history-chunk"` Tauri events for each token.
pub fn generate_evidence_answer_streaming(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    evidence: &str,
    question: &str,
    system_prompt: &str,
    thinking_effort: ThinkingEffort,
) -> Result<String, AppError> {
    let user_message = prompts::evidence_query_prompt(evidence, question);
    thinking::generate_with_think_budget(
        engine,
        system_prompt,
        &user_message,
        thinking_effort,
        0.3,
        &|token| {
            let _ = app.emit("patient-history-chunk", token);
        },
    )
}
