use crate::error::AppError;
use crate::llm::{
    self, download, embed::EmbedEngine, evidence, quantization, EngineStatus, LetterType,
    LlmEngine, ModelChoice, ReportType, SYSTEM_PROMPT_DE, SYSTEM_PROMPT_FR,
};
use crate::state::{llm_lock_poisoned, AppState, AuthState};
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
    let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
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
                last_generation_stats: None,
                inference_config: None,
                context_cache: Default::default(),
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
    inference_profile: Option<String>,
) -> Result<(), AppError> {
    // Validate filename to prevent path traversal
    validate_model_filename(&model_filename)?;

    let model_path = state.data_dir.join("models").join(&model_filename);
    let verification_path = model_path.clone();
    tokio::task::spawn_blocking(move || quantization::verify_promoted_model(&verification_path))
        .await
        .map_err(|error| AppError::Llm(format!("promotion verification task failed: {error}")))??;
    let model_name = model_filename.clone();
    // "governed" is the safe default. Named profiles remain available as
    // explicit research overrides and are checked against the same budget.
    let inference_profile = inference_profile.unwrap_or_else(|| "governed".to_string());

    // Only one swap may run at a time. Drop the state-owned old engine before
    // loading the replacement so two model/context allocations cannot overlap.
    let _swap_lease = state.llm_swap.lock().await;
    let previous_engine = state.llm.lock().map_err(|_| llm_lock_poisoned())?.take();
    if let Some(previous_engine) = previous_engine {
        // Removing it from AppState prevents new leases. Existing inference
        // tasks retain an Arc and are allowed to finish before memory is freed.
        let drain = async {
            while Arc::strong_count(&previous_engine) > 1 {
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        };
        if tokio::time::timeout(std::time::Duration::from_secs(120), drain)
            .await
            .is_err()
        {
            // Keep the application usable when an inference task is stuck.
            *state.llm.lock().map_err(|_| llm_lock_poisoned())? = Some(previous_engine);
            return Err(AppError::Llm(
                "Timed out waiting for active inference leases before model swap".to_string(),
            ));
        }
        drop(previous_engine);
    }

    let engine = tokio::task::spawn_blocking(move || {
        LlmEngine::load_with_profile(model_path, model_name, &inference_profile)
    })
    .await
    .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;

    *state.llm.lock().map_err(|_| llm_lock_poisoned())? = Some(Arc::new(engine));
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
        let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
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
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn generate_report(
    app: AppHandle,
    state: State<'_, AppState>,
    patient_context: String,
    report_type: String,
    session_notes: String,
    additional_context: Option<String>,
    instructions: Option<String>,
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
        let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
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
            additional_context.as_deref(),
            instructions.as_deref(),
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
        let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
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
        let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
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
        let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
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

/// Tokens reserved for the answer when sizing the evidence block.
const EVIDENCE_COMPLETION_TOKENS: usize = 4_096;

/// An assembled evidence block together with its manifest.
#[derive(Debug, Serialize)]
pub struct EvidencePreview {
    /// The evidence block exactly as it will be sent to the model.
    pub evidence: String,
    pub manifest: evidence::EvidenceManifest,
}

/// One evidence unit re-resolved against the live record, for citation display.
#[derive(Debug, Serialize)]
pub struct ResolvedEvidenceUnit {
    pub unit_id: String,
    pub record_kind: String,
    pub record_id: String,
    pub section: String,
    pub label: String,
    pub occurred_at: String,
    pub revision: String,
    pub char_start: usize,
    pub char_end: usize,
    pub text: String,
    /// The span still resolves to identical text at the current source revision.
    pub traceable: bool,
    pub resolution: evidence::SpanResolution,
}

/// The answer to a patient-history question plus the evidence behind it.
#[derive(Debug, Serialize)]
pub struct PatientHistoryAnswer {
    pub answer: String,
    pub manifest: evidence::EvidenceManifest,
    pub audit: evidence::AnswerAudit,
}

/// Embed `question` if the embedding engine is already resident.
///
/// The query path deliberately does not initialise the engine: that can download
/// a model, and retrieval degrades to lexical plus expansions without it. Use
/// `index_patient_evidence` to warm embeddings up front.
async fn embed_question_if_available(
    state: &State<'_, AppState>,
    question: &str,
) -> Option<Vec<f32>> {
    let engine = state.try_get_embed()?;
    let question = question.to_string();
    tokio::task::spawn_blocking(move || {
        engine
            .lock()
            .ok()
            .and_then(|mut engine| engine.embed_one(&question).ok())
    })
    .await
    .ok()
    .flatten()
}

/// Query a patient's history over provenance-bearing assembled evidence.
///
/// Emits `"patient-history-chunk"` events for each token, then
/// `"patient-history-manifest"` with the evidence manifest and citation audit,
/// and finally `"patient-history-done"`.
/// `system_prompt`: optional override; falls back to the built-in German prompt.
#[tauri::command]
pub async fn query_patient_history(
    app: AppHandle,
    state: State<'_, AppState>,
    patient_id: String,
    question: String,
    system_prompt: Option<String>,
) -> Result<PatientHistoryAnswer, AppError> {
    // Check authentication before processing patient data
    check_auth(&state)?;

    // Acquire the engine handle under the mutex, but do not run inference while holding the lock.
    let engine = {
        let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
        let engine = llm
            .as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?;
        // Clone the Arc so we can release the lock before inference.
        Arc::clone(engine)
    };

    let query_vec = embed_question_if_available(&state, &question).await;

    // Assemble a budget-bounded evidence block sized against this model's
    // context, measured with the model's own tokenizer.
    let assembled = {
        let pool = state.get_db()?;
        let conn = pool.conn()?;
        let request = evidence::EvidenceRequest::new(&patient_id, &question).with_token_budget(
            evidence::budget_for_context(engine.context_size(), EVIDENCE_COMPLETION_TOKENS),
        );
        let request = match &query_vec {
            Some(vector) => request.with_query_vector(vector),
            None => request,
        };
        let assembled = evidence::assemble_patient_evidence(&conn, &request, engine.as_ref())?;
        evidence::store_manifest(&conn, &assembled.manifest)?;
        assembled
    };

    // Resolve the system prompt into an owned String we can move into the blocking task.
    let prompt: String = system_prompt.unwrap_or_else(|| SYSTEM_PROMPT_DE.to_string());

    // Run the potentially long-running patient history query on a blocking thread.
    let app_clone = app.clone();
    let engine_clone = Arc::clone(&engine);
    let evidence_block = assembled.evidence.clone();
    let question_clone = question.clone();
    let answer = tokio::task::spawn_blocking(move || {
        llm::generate_evidence_answer_streaming(
            &app_clone,
            &engine_clone,
            &evidence_block,
            &question_clone,
            &prompt,
        )
    })
    .await
    .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;

    // Check every citation in the answer against the manifest and the record.
    let audit = {
        let pool = state.get_db()?;
        let conn = pool.conn()?;
        evidence::audit_answer(&conn, &assembled.manifest, &answer)?
    };

    let _ = app.emit(
        "patient-history-manifest",
        serde_json::json!({ "manifest": &assembled.manifest, "audit": &audit }),
    );
    let _ = app.emit("patient-history-done", ());

    Ok(PatientHistoryAnswer {
        answer,
        manifest: assembled.manifest,
        audit,
    })
}

/// Assemble the evidence for a question without running inference.
///
/// Lets the UI show what would be sent, why each unit was selected, and what was
/// left out, before spending inference time.
#[tauri::command]
pub async fn preview_patient_evidence(
    state: State<'_, AppState>,
    patient_id: String,
    question: String,
    token_budget: Option<usize>,
) -> Result<EvidencePreview, AppError> {
    check_auth(&state)?;

    // The tokenizer of the loaded model is used when one is available; otherwise
    // the heuristic counter keeps the preview available before a model is loaded.
    let engine = {
        let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
        llm.as_ref().map(Arc::clone)
    };
    let query_vec = embed_question_if_available(&state, &question).await;

    let budget = token_budget.unwrap_or_else(|| match &engine {
        Some(engine) => {
            evidence::budget_for_context(engine.context_size(), EVIDENCE_COMPLETION_TOKENS)
        }
        None => evidence::budget_for_context(16_384, EVIDENCE_COMPLETION_TOKENS),
    });

    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let request = evidence::EvidenceRequest::new(&patient_id, &question).with_token_budget(budget);
    let request = match &query_vec {
        Some(vector) => request.with_query_vector(vector),
        None => request,
    };
    let counter: &dyn evidence::TokenCounter = match &engine {
        Some(engine) => engine.as_ref(),
        None => &evidence::HeuristicCounter,
    };
    let assembled = evidence::assemble_patient_evidence(&conn, &request, counter)?;

    Ok(EvidencePreview {
        evidence: assembled.evidence,
        manifest: assembled.manifest,
    })
}

/// Refresh a patient's evidence index and embed any units that lack a vector.
///
/// Initialises the embedding engine on first use (downloads ~130 MB), so this is
/// an explicit action rather than something the query path triggers.
#[tauri::command]
pub async fn index_patient_evidence(
    state: State<'_, AppState>,
    patient_id: String,
) -> Result<evidence::IndexStats, AppError> {
    check_auth(&state)?;

    let stats = {
        let pool = state.get_db()?;
        let conn = pool.conn()?;
        evidence::refresh_patient_index(&conn, &patient_id, &evidence::IndexConfig::default())?
    };

    let pending = {
        let pool = state.get_db()?;
        let conn = pool.conn()?;
        evidence::pending_embeddings(&conn, &patient_id)?
    };
    if pending.is_empty() {
        return Ok(stats);
    }

    let embed_engine = match state.try_get_embed() {
        Some(engine) => engine,
        None => {
            let embed_cache_dir = state.data_dir.join("models").join("embed");
            let engine = tokio::task::spawn_blocking(move || {
                std::fs::create_dir_all(&embed_cache_dir)?;
                EmbedEngine::new(&embed_cache_dir)
            })
            .await
            .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;
            state.set_embed(engine)?;
            state
                .try_get_embed()
                .ok_or_else(|| AppError::Llm("Embedding engine unavailable".to_string()))?
        }
    };

    for (unit_id, text) in pending {
        let engine = Arc::clone(&embed_engine);
        let vector = tokio::task::spawn_blocking(move || {
            engine
                .lock()
                .map_err(|_| AppError::Llm("Embedding engine mutex poisoned".to_string()))?
                .embed_one(&text)
        })
        .await
        .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;

        let pool = state.get_db()?;
        let conn = pool.conn()?;
        evidence::index::store_unit_embedding(&conn, &unit_id, &vector)?;
    }

    // Re-read so the returned stats reflect the embeddings just stored.
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    Ok(evidence::IndexStats {
        units_missing_embeddings: evidence::pending_embeddings(&conn, &patient_id)?.len(),
        ..stats
    })
}

/// The most recent evidence manifest assembled for a patient, if any.
#[tauri::command]
pub async fn get_patient_evidence_manifest(
    state: State<'_, AppState>,
    patient_id: String,
) -> Result<Option<evidence::EvidenceManifest>, AppError> {
    check_auth(&state)?;
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    evidence::latest_manifest(&conn, &patient_id)
}

/// Resolve cited evidence units back to their current source text.
///
/// `traceable` is false when the source record moved on since assembly, which is
/// how the UI can mark a citation as no longer current instead of showing stale
/// text as fact.
#[tauri::command]
pub async fn resolve_evidence_units(
    state: State<'_, AppState>,
    patient_id: String,
    unit_ids: Vec<String>,
) -> Result<Vec<ResolvedEvidenceUnit>, AppError> {
    check_auth(&state)?;
    let pool = state.get_db()?;
    let conn = pool.conn()?;

    let units = evidence::retrieve::load_units(&conn, &patient_id, &unit_ids)?;
    let mut resolved = Vec::with_capacity(units.len());
    for unit in units {
        let resolution = evidence::provenance::resolve_span(
            &conn,
            &unit.patient_id,
            unit.kind,
            &unit.record_id,
            &unit.section,
            &unit.revision,
            unit.char_start,
            unit.char_end,
            &unit.text,
        )?;
        resolved.push(ResolvedEvidenceUnit {
            unit_id: unit.id,
            record_kind: unit.kind.as_str().to_string(),
            record_id: unit.record_id,
            section: unit.section,
            label: unit.label,
            occurred_at: unit.occurred_at,
            revision: unit.revision,
            char_start: unit.char_start,
            char_end: unit.char_end,
            text: unit.text,
            traceable: resolution.is_traceable(),
            resolution,
        });
    }
    Ok(resolved)
}
