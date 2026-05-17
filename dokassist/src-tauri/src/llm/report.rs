use super::{
    engine::LlmEngine,
    prompts::{self, LetterType, ReportType},
    sanitize::{build_delimited_prompt, sanitize_for_prompt},
};
use crate::error::AppError;
use tauri::Emitter;

/// Maximum tokens the model may spend inside a `<think>` block before the block
/// is force-closed and generation continues with the actual report/letter/summary.
/// 1 024 tokens ≈ 700–900 words of reasoning — sufficient for complex cases while
/// leaving the bulk of the 4 096-token budget for the written output.
const MAX_THINK_TOKENS: usize = 1024;

/// Runs two-phase generation that caps the `<think>` block.
///
/// **Phase 1** – normal streaming; if `MAX_THINK_TOKENS` thinking tokens are
/// consumed before `</think>` appears, generation stops early.
///
/// **Phase 2** (only if budget was hit) – a `</think>` marker is injected into
/// the output stream, then generation resumes from the accumulated context using
/// `generate_streaming_raw` so the model can write the actual content.
fn generate_with_think_budget(
    engine: &LlmEngine,
    system_prompt: &str,
    user_message: &str,
    max_tokens: usize,
    temperature: f32,
    emit: &dyn Fn(&str),
) -> Result<String, AppError> {
    let mut output = String::new();
    let mut think_tokens: usize = 0;
    let mut budget_hit = false;

    engine.generate_streaming(
        system_prompt,
        user_message,
        max_tokens,
        temperature,
        |token| {
            output.push_str(token);

            // Detect if we are currently inside an unclosed <think> block.
            let in_think = output.contains("<think>") && !output.contains("</think>");
            if in_think {
                think_tokens += 1;
                if think_tokens >= MAX_THINK_TOKENS {
                    budget_hit = true;
                    return false; // stop phase 1
                }
            }

            emit(token);
            true
        },
    )?;

    if budget_hit {
        // Inject the closing tag so the frontend renders thinking as complete.
        let close_tag = "</think>\n\n";
        output.push_str(close_tag);
        emit(close_tag);

        // Resume generation from the full context including the injected close tag.
        // ChatML format: system → user → assistant (partial, everything generated so far).
        let continuation = format!(
            "<|im_start|>system\n{system_prompt}<|im_end|>\n\
             <|im_start|>user\n{user_message}<|im_end|>\n\
             <|im_start|>assistant\n{output}",
        );

        engine.generate_streaming_raw(&continuation, 2048, temperature, |token| {
            output.push_str(token);
            emit(token);
            true
        })?;
    }

    Ok(output)
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
    )
}

/// Generate a report using a caller-supplied system prompt.
/// Emits `"report-chunk"` Tauri events for each token as it is produced.
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
) -> Result<String, AppError> {
    let user_message = prompts::report_generation_prompt(
        report_type,
        patient_context,
        session_notes,
        additional_context,
        instructions,
    );

    generate_with_think_budget(engine, system_prompt, &user_message, 4096, 0.7, &|token| {
        let _ = app.emit("report-chunk", token);
    })
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

    generate_with_think_budget(engine, system_prompt, &user_message, 4096, 0.7, &|token| {
        let _ = app.emit("text-improvement-chunk", token);
    })
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

    generate_with_think_budget(engine, system_prompt, &user_message, 4096, 0.7, &|token| {
        let _ = app.emit("session-summary-chunk", token);
    })
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

    generate_with_think_budget(engine, system_prompt, &user_message, 4096, 0.7, &|token| {
        let _ = app.emit("letter-chunk", token);
    })
}

/// Generate a response to a patient history query using RAG with streaming.
pub fn generate_patient_history_response_streaming(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    patient_context: &str,
    question: &str,
) -> Result<String, AppError> {
    generate_patient_history_response_streaming_with_prompt(
        app,
        engine,
        patient_context,
        question,
        prompts::SYSTEM_PROMPT_DE,
    )
}

/// Generate a response to a patient history query using a caller-supplied system prompt.
/// Emits `"patient-history-chunk"` Tauri events for each token as it is produced.
/// Returns the full completed response string.
pub fn generate_patient_history_response_streaming_with_prompt(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    patient_context: &str,
    question: &str,
    system_prompt: &str,
) -> Result<String, AppError> {
    let user_message = prompts::patient_history_query_prompt(patient_context, question);

    generate_with_think_budget(engine, system_prompt, &user_message, 4096, 0.7, &|token| {
        let _ = app.emit("patient-history-chunk", token);
    })
}
