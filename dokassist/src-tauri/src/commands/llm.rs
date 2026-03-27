use crate::error::AppError;
use crate::llm::{
    self, download, embed::EmbedEngine, EngineStatus, LetterType, LlmEngine, ModelChoice,
    ReportType, SYSTEM_PROMPT_DE, SYSTEM_PROMPT_FR,
};
use crate::state::{AppState, AuthState};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Validate a model filename to prevent path traversal attacks.
/// Returns an error if the filename contains path separators or parent directory components.
fn validate_model_filename(filename: &str) -> Result<(), AppError> {
    if filename.is_empty() {
        return Err(AppError::Validation(
            "Model filename cannot be empty".to_string(),
        ));
    }

    // Check for path separators
    if filename.contains('/') || filename.contains('\\') {
        return Err(AppError::Validation(
            "Model filename cannot contain path separators".to_string(),
        ));
    }

    // Check for parent directory components
    if filename.contains("..") {
        return Err(AppError::Validation(
            "Model filename cannot contain parent directory references".to_string(),
        ));
    }

    // Ensure it ends with .gguf
    if !filename.ends_with(".gguf") {
        return Err(AppError::Validation(
            "Model filename must end with .gguf".to_string(),
        ));
    }

    Ok(())
}

/// Check that the user is authenticated before processing sensitive patient data.
fn check_auth(state: &AppState) -> Result<(), AppError> {
    let auth = state
        .auth
        .lock()
        .map_err(|_| AppError::Llm("Auth state mutex poisoned".to_string()))?;

    if !matches!(*auth, AuthState::Unlocked { .. }) {
        return Err(AppError::AuthRequired);
    }

    Ok(())
}

/// Return the current engine status (safe to call before a model is loaded).
#[tauri::command]
pub async fn get_engine_status(state: State<'_, AppState>) -> Result<EngineStatus, AppError> {
    let llm = state.llm.lock().unwrap();
    match &*llm {
        Some(engine) => Ok(engine.status()),
        None => {
            let recommended = LlmEngine::recommended_model();
            let model_path = state.data_dir.join("models").join(&recommended.filename);
            let is_downloaded = model_path.exists();
            Ok(EngineStatus {
                is_loaded: false,
                model_name: None,
                model_path: None,
                total_ram_bytes: LlmEngine::total_ram(),
                is_downloaded,
                downloaded_filename: if is_downloaded {
                    Some(recommended.filename)
                } else {
                    None
                },
            })
        }
    }
}

/// Return the model tier recommended for this machine's RAM.
#[tauri::command]
pub async fn get_recommended_model() -> Result<ModelChoice, AppError> {
    Ok(LlmEngine::recommended_model())
}

/// Return the built-in German system prompt so the frontend can pre-populate its editor.
#[tauri::command]
pub async fn get_default_system_prompt() -> Result<String, AppError> {
    Ok(SYSTEM_PROMPT_DE.to_string())
}

/// Download a GGUF model from HuggingFace to ~/DokAssist/models/.
/// Streams progress via `"model-download-progress"` (f64) and `"model-download-done"` events.
#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model: ModelChoice,
) -> Result<(), AppError> {
    // Validate filename to prevent path traversal
    validate_model_filename(&model.filename)?;

    let dest_dir = state.data_dir.join("models");
    tokio::fs::create_dir_all(&dest_dir).await?;

    let dest_path = dest_dir.join(&model.filename);
    let url = download::model_url(&model.filename)?;
    download::download_model_with_progress(&app, &url, &dest_path, &model.filename).await?;
    Ok(())
}

/// Load a GGUF model from ~/DokAssist/models/ into memory (Metal-accelerated).
/// Uses spawn_blocking because model loading is a long blocking C-FFI operation.
#[tauri::command]
pub async fn load_model(
    state: State<'_, AppState>,
    model_filename: String,
) -> Result<(), AppError> {
    // Validate filename to prevent path traversal
    validate_model_filename(&model_filename)?;

    let model_path = state.data_dir.join("models").join(&model_filename);
    let model_name = model_filename.clone();

    let engine = tokio::task::spawn_blocking(move || LlmEngine::load(model_path, model_name))
        .await
        .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;

    *state.llm.lock().unwrap() = Some(Arc::new(engine));
    Ok(())
}

/// Extract structured metadata from a document using the loaded LLM.
/// `system_prompt`: optional override; falls back to the built-in German prompt.
#[tauri::command]
pub async fn extract_file_metadata(
    state: State<'_, AppState>,
    document_text: String,
    system_prompt: Option<String>,
) -> Result<llm::FileMetadata, AppError> {
    // Check authentication before processing patient data
    check_auth(&state)?;

    // Acquire the engine handle under the mutex, but do not run inference while holding the lock.
    let engine = {
        let llm = state.llm.lock().unwrap();
        let engine = llm
            .as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?;
        // Clone the Arc so we can release the lock before inference.
        Arc::clone(engine)
    };

    // Resolve the system prompt into an owned String we can move into the blocking task.
    let prompt: String = system_prompt.unwrap_or_else(|| SYSTEM_PROMPT_DE.to_string());

    // Run the potentially long-running metadata extraction on a blocking thread.
    let metadata = tokio::task::spawn_blocking(move || {
        llm::extract_metadata_with_prompt(&engine, &document_text, &prompt)
    })
    .await
    .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;

    Ok(metadata)
}

/// Generate a psychiatric report with streaming output.
/// Emits `"report-chunk"` events for each token and `"report-done"` on completion.
/// `system_prompt`: optional override; falls back to the built-in German prompt.
#[tauri::command]
pub async fn generate_report(
    app: AppHandle,
    state: State<'_, AppState>,
    patient_context: String,
    report_type: String,
    session_notes: String,
    system_prompt: Option<String>,
) -> Result<String, AppError> {
    // Check authentication before processing patient data
    check_auth(&state)?;

    let rt = match report_type.as_str() {
        "Befundbericht" => ReportType::Befundbericht,
        "Verlaufsbericht" => ReportType::Verlaufsbericht,
        "Ueberweisungsschreiben" => ReportType::Ueberweisungsschreiben,
        other => {
            return Err(AppError::Validation(format!(
                "Unknown report type: {other}"
            )))
        }
    };

    // Acquire the engine handle under the mutex, but do not run inference while holding the lock.
    let engine = {
        let llm = state.llm.lock().unwrap();
        let engine = llm
            .as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?;
        // Clone the Arc so we can release the lock before inference.
        Arc::clone(engine)
    };

    // Resolve the system prompt into an owned String we can move into the blocking task.
    let prompt: String = system_prompt.unwrap_or_else(|| SYSTEM_PROMPT_DE.to_string());

    // Run the potentially long-running report generation on a blocking thread.
    let app_clone = app.clone();
    let report = tokio::task::spawn_blocking(move || {
        llm::generate_report_streaming_with_prompt(
            &app_clone,
            &engine,
            rt,
            &patient_context,
            &session_notes,
            &prompt,
        )
    })
    .await
    .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;

    let _ = app.emit("report-done", ());
    Ok(report)
}

/// Status of the embedding model (used for literature semantic search).
#[derive(Debug, Serialize)]
pub struct EmbedStatus {
    /// Whether the engine is initialised in memory and ready to use.
    pub is_loaded: bool,
    /// Whether the ONNX model files exist on disk (cached from a previous run).
    pub is_downloaded: bool,
}

/// Return the current embed-engine status.
#[tauri::command]
pub async fn get_embed_status(state: State<'_, AppState>) -> Result<EmbedStatus, AppError> {
    let is_loaded = state.try_get_embed().is_some();
    let embed_cache_dir = state.data_dir.join("models").join("embed");
    let is_downloaded = embed_cache_dir
        .exists()
        .then(|| std::fs::read_dir(&embed_cache_dir).map(|mut d| d.next().is_some()))
        .and_then(|r| r.ok())
        .unwrap_or(false);
    Ok(EmbedStatus {
        is_loaded,
        is_downloaded,
    })
}

/// Download and initialise the embedding engine (idempotent — no-op if already loaded).
/// This is a long blocking operation; progress is not streamed.
#[tauri::command]
pub async fn initialize_embed_engine(state: State<'_, AppState>) -> Result<(), AppError> {
    if state.try_get_embed().is_some() {
        return Ok(());
    }
    let embed_cache_dir = state.data_dir.join("models").join("embed");
    let engine = tokio::task::spawn_blocking(move || -> Result<EmbedEngine, AppError> {
        std::fs::create_dir_all(&embed_cache_dir)?;
        EmbedEngine::new(&embed_cache_dir)
    })
    .await
    .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;
    state.set_embed(engine)?;
    Ok(())
}

/// Improve or provide suggestions for a piece of text with streaming output.
/// Emits `"text-improvement-chunk"` events for each token and `"text-improvement-done"` on completion.
/// `system_prompt`: optional override; falls back to the built-in German prompt.
#[tauri::command]
pub async fn improve_text(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    instruction: String,
    system_prompt: Option<String>,
) -> Result<String, AppError> {
    // Check authentication before processing patient data
    check_auth(&state)?;

    // Acquire the engine handle under the mutex, but do not run inference while holding the lock.
    let engine = {
        let llm = state.llm.lock().unwrap();
        let engine = llm
            .as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?;
        // Clone the Arc so we can release the lock before inference.
        Arc::clone(engine)
    };

    // Resolve the system prompt into an owned String we can move into the blocking task.
    let prompt: String = system_prompt.unwrap_or_else(|| SYSTEM_PROMPT_DE.to_string());

    // Run the potentially long-running text improvement on a blocking thread.
    let app_clone = app.clone();
    let improved = tokio::task::spawn_blocking(move || {
        llm::improve_text_streaming_with_prompt(&app_clone, &engine, &text, &instruction, &prompt)
    })
    .await
    .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;

    let _ = app.emit("text-improvement-done", ());
    Ok(improved)
}

/// Generate a session summary with streaming output.
/// Emits `"session-summary-chunk"` events for each token and `"session-summary-done"` on completion.
/// `system_prompt`: optional override; falls back to the built-in German prompt.
#[tauri::command]
pub async fn generate_session_summary(
    app: AppHandle,
    state: State<'_, AppState>,
    patient_context: String,
    session_notes: String,
    system_prompt: Option<String>,
) -> Result<String, AppError> {
    check_auth(&state)?;

    let engine = {
        let llm = state.llm.lock().unwrap();
        let engine = llm
            .as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?;
        Arc::clone(engine)
    };

    let prompt: String = system_prompt.unwrap_or_else(|| SYSTEM_PROMPT_DE.to_string());

    let app_clone = app.clone();
    let summary = tokio::task::spawn_blocking(move || {
        llm::generate_session_summary_streaming_with_prompt(
            &app_clone,
            &engine,
            &patient_context,
            &session_notes,
            &prompt,
        )
    })
    .await
    .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;

    let _ = app.emit("session-summary-done", ());
    Ok(summary)
}

/// Generate a formal letter (referral, insurance authorization, or therapy extension) with streaming output.
/// Emits `"letter-chunk"` events for each token and `"letter-done"` on completion.
/// `system_prompt`: optional override; falls back to the built-in German or French prompt based on language.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn generate_letter(
    app: AppHandle,
    state: State<'_, AppState>,
    letter_type: String,
    language: String,
    patient_context: String,
    clinical_summary: String,
    recipient_name: Option<String>,
    system_prompt: Option<String>,
) -> Result<String, AppError> {
    check_auth(&state)?;

    let lt = match letter_type.as_str() {
        "referral" => LetterType::Referral,
        "insurance_authorization" => LetterType::InsuranceAuthorization,
        "therapy_extension" => LetterType::TherapyExtension,
        other => {
            return Err(AppError::Validation(format!(
                "Unknown letter type: {other}"
            )))
        }
    };

    if language != "de" && language != "fr" {
        return Err(AppError::Validation(format!(
            "Unsupported language: {language}. Must be 'de' or 'fr'"
        )));
    }

    let engine = {
        let llm = state.llm.lock().unwrap();
        let engine = llm
            .as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?;
        Arc::clone(engine)
    };

    let prompt: String = system_prompt.unwrap_or_else(|| {
        if language == "fr" {
            SYSTEM_PROMPT_FR.to_string()
        } else {
            SYSTEM_PROMPT_DE.to_string()
        }
    });

    let app_clone = app.clone();
    let recipient_name_clone = recipient_name.clone();
    let letter = tokio::task::spawn_blocking(move || {
        llm::generate_letter_streaming_with_prompt(
            &app_clone,
            &engine,
            lt,
            &language,
            &patient_context,
            &clinical_summary,
            recipient_name_clone.as_deref(),
            &prompt,
        )
    })
    .await
    .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;

    let _ = app.emit("letter-done", ());
    Ok(letter)
}
