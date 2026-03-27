use super::{
    engine::LlmEngine,
    prompts::{self, LetterType, ReportType},
    sanitize::{build_delimited_prompt, sanitize_for_prompt},
};
use crate::error::AppError;
use tauri::Emitter;

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
        prompts::SYSTEM_PROMPT_DE,
    )
}

/// Generate a report using a caller-supplied system prompt.
/// Emits `"report-chunk"` Tauri events for each token as it is produced.
/// Returns the full completed report string.
pub fn generate_report_streaming_with_prompt(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    report_type: ReportType,
    patient_context: &str,
    session_notes: &str,
    system_prompt: &str,
) -> Result<String, AppError> {
    let user_message =
        prompts::report_generation_prompt(report_type, patient_context, session_notes);

    let mut full_report = String::new();

    engine.generate_streaming(system_prompt, &user_message, 2048, 0.7, |token| {
        full_report.push_str(token);
        let _ = app.emit("report-chunk", token);
        true
    })?;

    Ok(full_report)
}

/// Improve text based on provided instruction using the built-in system prompt.
pub fn improve_text_streaming(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    text: &str,
    instruction: &str,
) -> Result<String, AppError> {
    improve_text_streaming_with_prompt(app, engine, text, instruction, prompts::SYSTEM_PROMPT_DE)
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
) -> Result<String, AppError> {
    let safe_text = sanitize_for_prompt(text);
    let safe_instruction = sanitize_for_prompt(instruction);

    let user_message = build_delimited_prompt(&safe_instruction, &safe_text);

    let mut improved_text = String::new();

    engine.generate_streaming(system_prompt, &user_message, 2048, 0.7, |token| {
        improved_text.push_str(token);
        let _ = app.emit("text-improvement-chunk", token);
        true
    })?;

    Ok(improved_text)
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
    )
}

/// Generate a session summary using a caller-supplied system prompt.
/// Emits `"session-summary-chunk"` Tauri events for each token as it is produced.
/// Returns the full completed session summary string.
pub fn generate_session_summary_streaming_with_prompt(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    patient_context: &str,
    session_notes: &str,
    system_prompt: &str,
) -> Result<String, AppError> {
    let user_message = prompts::session_summary_prompt(patient_context, session_notes);

    let mut full_summary = String::new();

    engine.generate_streaming(system_prompt, &user_message, 2048, 0.7, |token| {
        full_summary.push_str(token);
        let _ = app.emit("session-summary-chunk", token);
        true
    })?;

    Ok(full_summary)
}

/// Generate a letter using a caller-supplied system prompt.
/// Emits `"letter-chunk"` Tauri events for each token as it is produced.
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
) -> Result<String, AppError> {
    let user_message = prompts::letter_generation_prompt(
        letter_type,
        language,
        patient_context,
        clinical_summary,
        recipient_name,
    );

    let mut full_letter = String::new();

    engine.generate_streaming(system_prompt, &user_message, 2048, 0.7, |token| {
        full_letter.push_str(token);
        let _ = app.emit("letter-chunk", token);
        true
    })?;

    Ok(full_letter)
}
