use crate::error::AppError;
use crate::llm::harness::SamplerConfig;
use crate::llm::{
    self, embed::EmbedEngine, evidence, quantization, DesiredModelStatus, EngineStatus, LetterType,
    LlmEngine, ModelChoice, ReportType, ThinkingEffort, SYSTEM_PROMPT_DE, SYSTEM_PROMPT_FR,
};
use crate::models::model;
use crate::state::{llm_lock_poisoned, AppState, AuthState, LlmLoadDisposition};
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
    let desired_model = resolve_desired_model(&state);
    let lifecycle = state.llm_lifecycle();
    let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
    match &*llm {
        Some(engine) => {
            let mut status = engine.status();
            status.desired_model = desired_model;
            status.lifecycle = lifecycle;
            Ok(status)
        }
        None => {
            let recommended = LlmEngine::recommended_model();
            let desired_filename = desired_model
                .as_ref()
                .filter(|model| model.exists_on_disk)
                .map(|model| model.filename.clone());
            let fallback_path = state.data_dir.join("models").join(&recommended.filename);
            let downloaded_filename =
                desired_filename.or_else(|| fallback_path.exists().then_some(recommended.filename));
            Ok(EngineStatus {
                is_loaded: false,
                model_name: None,
                model_path: None,
                total_ram_bytes: LlmEngine::total_ram(),
                is_downloaded: downloaded_filename.is_some(),
                downloaded_filename,
                last_generation_stats: None,
                inference_config: None,
                context_cache: Default::default(),
                desired_model,
                lifecycle: if matches!(lifecycle.phase, crate::llm::EngineLifecyclePhase::Ready) {
                    // Lock/reset/window-close clear the engine independently
                    // of an explicit user unload. Status remains truthful
                    // without changing those security-owned transitions.
                    crate::llm::EngineLifecycleStatus::default()
                } else {
                    lifecycle
                },
            })
        }
    }
}

fn resolve_desired_model(state: &AppState) -> Option<DesiredModelStatus> {
    let db = state.get_db().ok()?;
    let conn = db.conn().ok()?;
    let model = model::get_default_model(&conn).ok()??;
    let exists_on_disk = state
        .data_dir
        .join("models")
        .join(&model.filename)
        .is_file();
    Some(DesiredModelStatus {
        id: model.id,
        name: model.name,
        filename: model.filename,
        exists_on_disk,
    })
}

#[derive(Clone)]
struct RouterPlan {
    managed_model: Option<model::Model>,
    configured_override: Option<model::Model>,
    selected_model: Option<model::Model>,
    selection_source: llm::router::RouterSelectionSource,
}

/// Resolve the durable optional override first. Without one, the managed
/// policy chooses the smallest *installed and compatible* allow-listed model,
/// rather than an arbitrary registry row or the writing default.
fn resolve_router_plan(state: &AppState) -> Result<RouterPlan, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;
    let configured_override = model::get_router_override(&conn)?;
    let models_dir = state.data_dir.join("models");

    let managed_model = model::list_models(&conn)?
        .into_iter()
        .filter(|candidate| models_dir.join(&candidate.filename).is_file())
        .filter(|candidate| {
            llm::download::find_model(&candidate.filename)
                .map(|entry| {
                    entry.min_ram_gb.saturating_mul(1024 * 1024 * 1024) <= LlmEngine::total_ram()
                })
                .unwrap_or(false)
        })
        .min_by_key(|candidate| candidate.size_bytes);

    let (selected_model, selection_source) = match configured_override.as_ref() {
        Some(candidate) if models_dir.join(&candidate.filename).is_file() => (
            Some(candidate.clone()),
            llm::router::RouterSelectionSource::ExpertOverride,
        ),
        Some(_) => (None, llm::router::RouterSelectionSource::Unavailable),
        None => match managed_model.as_ref() {
            Some(candidate) => (
                Some(candidate.clone()),
                llm::router::RouterSelectionSource::SmallestCompatibleInstalled,
            ),
            None => (None, llm::router::RouterSelectionSource::Unavailable),
        },
    };

    Ok(RouterPlan {
        managed_model,
        configured_override,
        selected_model,
        selection_source,
    })
}

fn model_weight_bytes(candidate: Option<&model::Model>) -> u64 {
    candidate
        .and_then(|model| u64::try_from(model.size_bytes).ok())
        .unwrap_or_default()
}

fn active_writing_model(state: &AppState) -> Option<model::Model> {
    let filename = state.llm_lifecycle().active_filename?;
    let db = state.get_db().ok()?;
    let conn = db.conn().ok()?;
    model::get_model_by_filename(&conn, &filename).ok()
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

    load_model_request(&state, model_filename, inference_profile).await
}

/// Ensure the configured writing model is resident. The durable registry
/// default is the sole automatic model-selection policy; task-specific rows
/// are intentionally not consulted because they never selected an engine.
#[tauri::command]
pub async fn ensure_writing_model_loaded(
    state: State<'_, AppState>,
) -> Result<EngineStatus, AppError> {
    let desired = resolve_desired_model(&state).ok_or_else(|| {
        AppError::Validation(
            "Choose an installed default writing model before starting a chat".to_string(),
        )
    })?;
    if !desired.exists_on_disk {
        return Err(AppError::Validation(format!(
            "The configured writing model '{}' is missing from disk",
            desired.filename
        )));
    }
    load_model_request(&state, desired.filename, None).await?;
    get_engine_status(state).await
}

/// Explicitly release the resident writing model. Existing inference leases
/// keep their Arc until they finish; this command only prevents new work from
/// acquiring the engine and releases the state-owned allocation.
#[tauri::command]
pub async fn unload_model(state: State<'_, AppState>) -> Result<(), AppError> {
    state.begin_llm_unload()?;
    let _swap_lease = state.llm_swap.lock().await;
    let old_engine = state.llm.lock().map_err(|_| llm_lock_poisoned())?.take();
    drop(old_engine);
    state.mark_llm_unloaded();
    Ok(())
}

async fn load_model_request(
    state: &AppState,
    model_filename: String,
    inference_profile: Option<String>,
) -> Result<(), AppError> {
    match state.begin_llm_load(&model_filename)? {
        LlmLoadDisposition::Join => return state.wait_for_llm_load(&model_filename).await,
        LlmLoadDisposition::AlreadyReady => return Ok(()),
        LlmLoadDisposition::Start => {}
    }

    let result = load_model_after_lifecycle_start(state, &model_filename, inference_profile).await;
    match &result {
        Ok(()) => state.mark_llm_ready(model_filename),
        Err(error) => state.mark_llm_load_failed(model_filename, error.to_string()),
    }
    result
}

async fn load_model_after_lifecycle_start(
    state: &AppState,
    model_filename: &str,
    inference_profile: Option<String>,
) -> Result<(), AppError> {
    let model_path = state.data_dir.join("models").join(model_filename);
    let verification_path = model_path.clone();
    tokio::task::spawn_blocking(move || quantization::verify_promoted_model(&verification_path))
        .await
        .map_err(|error| AppError::Llm(format!("promotion verification task failed: {error}")))??;
    let model_name = model_filename.to_string();
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

/// Return an optional dedicated router engine. A memory admission failure is
/// not a chat failure: callers must use the writing engine for the probe and
/// diagnostics records that explicitly.
pub async fn ensure_router_engine(state: &AppState) -> Result<Option<Arc<LlmEngine>>, AppError> {
    let plan = resolve_router_plan(state)?;
    let Some(selected) = plan.selected_model else {
        unload_router_if_resident(state).await?;
        return Ok(None);
    };

    let writing = active_writing_model(state);
    let writing_weight = model_weight_bytes(writing.as_ref());
    let router_weight = model_weight_bytes(Some(&selected));
    let dual_safe =
        llm::router::dual_residency_is_safe(LlmEngine::total_ram(), writing_weight, router_weight);
    // A second instance of the writing engine cannot improve routing and
    // wastes the exact capacity the admission policy protects.
    if !dual_safe
        || writing
            .as_ref()
            .is_some_and(|model| model.id == selected.id)
    {
        unload_router_if_resident(state).await?;
        return Ok(None);
    }

    match state.begin_router_load(&selected.filename)? {
        LlmLoadDisposition::Join => {
            state.wait_for_router_load(&selected.filename).await?;
        }
        LlmLoadDisposition::AlreadyReady => {}
        LlmLoadDisposition::Start => {
            let result = load_router_after_lifecycle_start(state, &selected).await;
            match &result {
                Ok(()) => state.mark_router_ready(selected.filename.clone()),
                Err(error) => {
                    state.mark_router_load_failed(selected.filename.clone(), error.to_string())
                }
            }
            result?;
        }
    }

    state
        .router_llm
        .lock()
        .map_err(|_| AppError::Llm("Router engine mutex poisoned".to_string()))?
        .as_ref()
        .map(Arc::clone)
        .ok_or_else(|| {
            AppError::Llm("Router finished loading without becoming resident".to_string())
        })
        .map(Some)
}

async fn unload_router_if_resident(state: &AppState) -> Result<(), AppError> {
    let resident = state
        .router_llm
        .lock()
        .map_err(|_| AppError::Llm("Router engine mutex poisoned".to_string()))?
        .is_some();
    if resident
        || matches!(
            state.router_lifecycle().phase,
            crate::llm::EngineLifecyclePhase::Ready
        )
    {
        unload_router_engine(state).await?;
    }
    Ok(())
}

async fn load_router_after_lifecycle_start(
    state: &AppState,
    selected: &model::Model,
) -> Result<(), AppError> {
    let model_path = state.data_dir.join("models").join(&selected.filename);
    let verification_path = model_path.clone();
    tokio::task::spawn_blocking(move || quantization::verify_promoted_model(&verification_path))
        .await
        .map_err(|error| {
            AppError::Llm(format!(
                "router promotion verification task failed: {error}"
            ))
        })??;

    let _swap_lease = state.router_llm_swap.lock().await;
    let previous = state
        .router_llm
        .lock()
        .map_err(|_| AppError::Llm("Router engine mutex poisoned".to_string()))?
        .take();
    if let Some(previous) = previous {
        let drain = async {
            while Arc::strong_count(&previous) > 1 {
                tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            }
        };
        if tokio::time::timeout(std::time::Duration::from_secs(120), drain)
            .await
            .is_err()
        {
            *state
                .router_llm
                .lock()
                .map_err(|_| AppError::Llm("Router engine mutex poisoned".to_string()))? =
                Some(previous);
            return Err(AppError::Llm(
                "Timed out waiting for active router inference leases before swap".to_string(),
            ));
        }
        drop(previous);
    }

    let model_name = selected.filename.clone();
    let engine = tokio::task::spawn_blocking(move || {
        // Tool probes are short; governed remains the same conservative
        // profile used for the writing runtime and enforces its own budget.
        LlmEngine::load_with_profile(model_path, model_name, "governed")
    })
    .await
    .map_err(|error| AppError::Llm(format!("router spawn_blocking error: {error}")))??;
    *state
        .router_llm
        .lock()
        .map_err(|_| AppError::Llm("Router engine mutex poisoned".to_string()))? =
        Some(Arc::new(engine));
    Ok(())
}

#[tauri::command]
pub async fn unload_router_model(state: State<'_, AppState>) -> Result<(), AppError> {
    unload_router_engine(&state).await
}

async fn unload_router_engine(state: &AppState) -> Result<(), AppError> {
    state.begin_router_unload()?;
    let _swap_lease = state.router_llm_swap.lock().await;
    let old = state
        .router_llm
        .lock()
        .map_err(|_| AppError::Llm("Router engine mutex poisoned".to_string()))?
        .take();
    drop(old);
    state.mark_router_unloaded();
    Ok(())
}

/// Update the durable expert override. Clearing it restores the managed
/// smallest-compatible policy. Overrides must reference a downloaded registry
/// model, never a caller-supplied path.
#[tauri::command]
pub async fn set_router_model_override(
    state: State<'_, AppState>,
    model_id: Option<String>,
) -> Result<(), AppError> {
    {
        let db = state.get_db()?;
        let conn = db.conn()?;
        if let Some(model_id) = model_id.as_deref() {
            let candidate = model::get_model(&conn, model_id)?;
            if !state
                .data_dir
                .join("models")
                .join(&candidate.filename)
                .is_file()
            {
                return Err(AppError::Validation(format!(
                    "Router override '{}' is missing from disk",
                    candidate.filename
                )));
            }
        }
        model::set_router_override(&conn, model_id.as_deref())?;
    }

    // Invalidate the old role immediately. The next agent turn resolves the
    // durable policy and lazily loads the selected runtime.
    unload_router_if_resident(&state).await?;
    Ok(())
}

#[tauri::command]
pub async fn get_router_diagnostics(
    state: State<'_, AppState>,
) -> Result<llm::router::RouterDiagnostics, AppError> {
    let plan = resolve_router_plan(&state)?;
    let writing = active_writing_model(&state);
    let writing_weight = model_weight_bytes(writing.as_ref());
    let router_weight = model_weight_bytes(plan.selected_model.as_ref());
    let total_ram_bytes = LlmEngine::total_ram();
    let dual_safe = plan.selected_model.as_ref().is_some_and(|selected| {
        llm::router::dual_residency_is_safe(
            total_ram_bytes,
            writing_weight,
            model_weight_bytes(Some(selected)),
        ) && writing
            .as_ref()
            .is_none_or(|active| active.id != selected.id)
    });
    let router_resident = state
        .router_llm
        .lock()
        .map_err(|_| AppError::Llm("Router engine mutex poisoned".to_string()))?
        .is_some();
    let lifecycle = state.router_lifecycle();
    // Lock/reset clears model slots at the application ML boundary. Do not
    // expose a stale Ready transition as a still-resident router afterwards.
    let lifecycle =
        if !router_resident && matches!(lifecycle.phase, crate::llm::EngineLifecyclePhase::Ready) {
            crate::llm::EngineLifecycleStatus::default()
        } else {
            lifecycle
        };
    let active_model_filename = lifecycle.active_filename.clone();
    let selected_model_filename = plan
        .selected_model
        .as_ref()
        .map(|model| model.filename.clone());
    let residency = if active_model_filename.is_some() {
        llm::router::RouterResidency::DualResident
    } else if plan.selected_model.is_none() {
        llm::router::RouterResidency::Unavailable
    } else if matches!(lifecycle.phase, crate::llm::EngineLifecyclePhase::Error) {
        llm::router::RouterResidency::WritingFallbackError
    } else if !dual_safe {
        llm::router::RouterResidency::WritingFallbackMemory
    } else {
        llm::router::RouterResidency::Unloaded
    };
    let fallback_reason = match residency {
        llm::router::RouterResidency::WritingFallbackMemory => Some(
            "A separate router would exceed the conservative dual-residency weight budget; probes use the writing engine.".to_string(),
        ),
        llm::router::RouterResidency::Unavailable => Some(
            "No compatible downloaded router model is available.".to_string(),
        ),
        llm::router::RouterResidency::WritingFallbackError => lifecycle.error.clone(),
        _ => None,
    };
    let writing_resident = state.llm.lock().map_err(|_| llm_lock_poisoned())?.is_some();
    let mode = match plan.selection_source {
        llm::router::RouterSelectionSource::ExpertOverride => {
            llm::router::RouterMode::AdvancedOverride
        }
        _ => llm::router::RouterMode::Managed,
    };
    Ok(llm::router::RouterDiagnostics {
        role: "tool_router".to_string(),
        mode,
        selection_source: plan.selection_source,
        managed_model_filename: plan.managed_model.as_ref().map(|model| model.filename.clone()),
        selected_model_filename,
        active_model_filename,
        resident_engine_count: u8::from(writing_resident) + u8::from(router_resident),
        lifecycle,
        residency,
        dual_residency_safe: dual_safe,
        total_ram_bytes,
        dual_residency_weight_budget_bytes: llm::router::dual_residency_weight_budget(total_ram_bytes),
        estimated_resident_weight_bytes: writing_weight.saturating_add(router_weight),
        fallback_reason,
        benchmark: state.router_benchmark(),
        advanced_override: llm::router::AdvancedRouterOverrideMetadata {
            available: true,
            configured_filename: plan.configured_override.as_ref().map(|model| model.filename.clone()),
            requires_separate_engine: true,
            limitation: "The override is used only for tool probes when the conservative dual-residency budget admits a second engine; final prose always uses the writing engine.".to_string(),
        },
    })
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
    thinking_effort: Option<ThinkingEffort>,
    sampler: Option<SamplerConfig>,
    generation_id: Option<String>,
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
    let generation_id = generation_id.unwrap_or_else(|| uuid::Uuid::now_v7().to_string());
    let generation_id_for_task = generation_id.clone();
    let report = tokio::task::spawn_blocking(move || {
        llm::generate_report_streaming_with_sampler_and_stream_id(
            &app_clone,
            &engine,
            rt,
            &patient_context,
            &session_notes,
            additional_context.as_deref(),
            instructions.as_deref(),
            &prompt,
            thinking_effort.unwrap_or_default(),
            sampler,
            Some(&generation_id_for_task),
        )
    })
    .await
    .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;

    let _ = app.emit("report-done", ());
    let _ = app.emit(
        "report-done-session",
        serde_json::json!({"generation_id": generation_id}),
    );
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
    thinking_effort: Option<ThinkingEffort>,
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
        llm::improve_text_streaming_with_prompt(
            &app_clone,
            &engine,
            &text,
            &instruction,
            &prompt,
            thinking_effort.unwrap_or_default(),
        )
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
    thinking_effort: Option<ThinkingEffort>,
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
            thinking_effort.unwrap_or_default(),
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
    thinking_effort: Option<ThinkingEffort>,
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
            thinking_effort.unwrap_or_default(),
        )
    })
    .await
    .map_err(|e| AppError::Llm(format!("spawn_blocking error: {e}")))??;

    let _ = app.emit("letter-done", ());
    Ok(letter)
}

/// Tokens reserved for the answer when sizing the evidence block.
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
    thinking_effort: Option<ThinkingEffort>,
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
    let thinking = thinking_effort.unwrap_or_default();

    // Assemble a budget-bounded evidence block sized against this model's
    // context, measured with the model's own tokenizer.
    let assembled = {
        let pool = state.get_db()?;
        let conn = pool.conn()?;
        let request = evidence::EvidenceRequest::new(&patient_id, &question).with_token_budget(
            evidence::budget_for_context(engine.context_size(), thinking.max_tokens()),
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
            thinking,
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
    thinking_effort: Option<ThinkingEffort>,
) -> Result<EvidencePreview, AppError> {
    check_auth(&state)?;

    // The tokenizer of the loaded model is used when one is available; otherwise
    // the heuristic counter keeps the preview available before a model is loaded.
    let engine = {
        let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
        llm.as_ref().map(Arc::clone)
    };
    let query_vec = embed_question_if_available(&state, &question).await;

    let completion_tokens = thinking_effort.unwrap_or_default().max_tokens();
    let budget = token_budget.unwrap_or_else(|| match &engine {
        Some(engine) => evidence::budget_for_context(engine.context_size(), completion_tokens),
        None => evidence::budget_for_context(16_384, completion_tokens),
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
