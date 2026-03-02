use tauri::Emitter;
use crate::error::AppError;
use super::{engine::LlmEngine, prompts::{self, ReportType}};

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
