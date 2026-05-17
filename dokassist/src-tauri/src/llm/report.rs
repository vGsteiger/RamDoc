use super::{
    engine::LlmEngine,
    prompts::{self, LetterType, ReportType},
    sanitize::{build_delimited_prompt, sanitize_for_prompt},
};
use crate::error::AppError;
use tauri::Emitter;

/// Maximum tokens the model may spend inside a `<think>` block before the block
/// is force-closed and generation continues with the actual report/letter/summary.
const MAX_THINK_TOKENS: usize = 1024;

/// Maximum combined input tokens (prompt + system) before triggering pre-summarization.
/// Derived from N_CTX (8192) minus max generation budget (4096) minus overhead (256).
const MAX_INPUT_TOKENS: usize = 3840;

/// Max tokens for the condensed-context summary output.
const SUMMARIZE_MAX_TOKENS: usize = 800;

/// Char limits for truncating inputs before the summarizer pass (prevents summarizer overflow).
const MAX_CONTEXT_CHARS: usize = 16_000;
const MAX_NOTES_CHARS: usize = 6_000;

/// Trim `s` to at most `max_chars` bytes, always on a UTF-8 character boundary.
fn truncate_to_char_boundary(s: &str, max_chars: usize) -> &str {
    if s.len() <= max_chars {
        return s;
    }
    let mut boundary = max_chars;
    while !s.is_char_boundary(boundary) {
        boundary -= 1;
    }
    &s[..boundary]
}

/// Returns `true` when the formatted ChatML prompt for the given inputs would
/// exceed `MAX_INPUT_TOKENS`, meaning pre-summarization is required.
fn needs_summarization(
    engine: &LlmEngine,
    system_prompt: &str,
    patient_context: &str,
    session_notes: &str,
) -> bool {
    let combined = format!(
        "<|im_start|>system\n{system_prompt}<|im_end|>\n\
         <|im_start|>user\nPatientenkontext:\n{patient_context}\n\n\
         Sitzungsnotizen:\n{session_notes}<|im_end|>\n<|im_start|>assistant\n"
    );
    engine.count_tokens(&combined) > MAX_INPUT_TOKENS
}

/// Condenses `patient_context` + `session_notes` into a shorter summary string
/// that fits within the context window.
fn run_summarization(
    engine: &LlmEngine,
    system_prompt: &str,
    patient_context: &str,
    session_notes: &str,
) -> Result<String, AppError> {
    let ctx = truncate_to_char_boundary(patient_context, MAX_CONTEXT_CHARS);
    let notes = truncate_to_char_boundary(session_notes, MAX_NOTES_CHARS);
    let summarization_msg = prompts::context_summarization_prompt(ctx, notes);
    engine.generate(system_prompt, &summarization_msg, SUMMARIZE_MAX_TOKENS, 0.3)
}

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
    let mut in_think = false;
    // Rolling tail buffer to detect tags that may be split across token boundaries,
    // without rescanning the full output on every token (which would be O(n²)).
    let mut tag_tail = String::new();

    engine.generate_streaming(
        system_prompt,
        user_message,
        max_tokens,
        temperature,
        |token| {
            output.push_str(token);
            tag_tail.push_str(token);

            if !in_think && tag_tail.contains("<think>") {
                in_think = true;
            }
            if in_think && tag_tail.contains("</think>") {
                in_think = false;
            }
            // Keep tail short enough to span a split tag; "</think>" is 8 chars.
            if tag_tail.len() > 16 {
                tag_tail.drain(..tag_tail.len() - 16);
            }

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

    // Read phase-1 stats before any phase-2 call overwrites them.
    let phase1_stats = engine.last_generation_stats();

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

        engine.generate_streaming_raw(
            &continuation,
            max_tokens.saturating_sub(MAX_THINK_TOKENS),
            temperature,
            |token| {
                output.push_str(token);
                emit(token);
                true
            },
        )?;
    } else if let Some(stats) = phase1_stats {
        // Detect context-window overflow: when n_cur + 1 >= N_CTX the loop breaks,
        // so prompt_tokens + completion_tokens == N_CTX - 1.
        let ctx_size = LlmEngine::context_size();
        let was_cut_off = stats.completion_tokens > 0
            && stats.prompt_tokens + stats.completion_tokens + 10 >= ctx_size;

        if was_cut_off {
            // Anchor the continuation with the tail of the partial output.
            let tail_start = output.len().saturating_sub(800);
            let tail = &output[tail_start..];
            let continuation_msg = prompts::continuation_prompt(tail);

            engine.generate_streaming(
                system_prompt,
                &continuation_msg,
                max_tokens,
                temperature,
                |token| {
                    output.push_str(token);
                    emit(token);
                    true
                },
            )?;
        }
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
) -> Result<String, AppError> {
    let summary_opt = if needs_summarization(engine, system_prompt, patient_context, session_notes)
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
/// If inputs are too long, emits `"session-summary-summarizing"` then condenses them first.
/// Returns the full completed session summary string.
pub fn generate_session_summary_streaming_with_prompt(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    patient_context: &str,
    session_notes: &str,
    system_prompt: &str,
) -> Result<String, AppError> {
    let summary_opt = if needs_summarization(engine, system_prompt, patient_context, session_notes)
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

    generate_with_think_budget(engine, system_prompt, &user_message, 4096, 0.7, &|token| {
        let _ = app.emit("session-summary-chunk", token);
    })
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
) -> Result<String, AppError> {
    let summary_opt =
        if needs_summarization(engine, system_prompt, patient_context, clinical_summary) {
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
/// If the patient context is too long, emits `"patient-history-summarizing"` and condenses it first.
/// Returns the full completed response string.
pub fn generate_patient_history_response_streaming_with_prompt(
    app: &tauri::AppHandle,
    engine: &LlmEngine,
    patient_context: &str,
    question: &str,
    system_prompt: &str,
) -> Result<String, AppError> {
    // Use `question` as the second input; it is typically short so summarization
    // will only trigger when the patient_context itself is very large.
    let summary_opt = if needs_summarization(engine, system_prompt, patient_context, question) {
        let _ = app.emit("patient-history-summarizing", ());
        Some(run_summarization(
            engine,
            system_prompt,
            patient_context,
            question,
        )?)
    } else {
        None
    };
    let eff_ctx = match &summary_opt {
        Some(s) => s.as_str(),
        None => patient_context,
    };

    let user_message = prompts::patient_history_query_prompt(eff_ctx, question);

    generate_with_think_budget(engine, system_prompt, &user_message, 4096, 0.7, &|token| {
        let _ = app.emit("patient-history-chunk", token);
    })
}
