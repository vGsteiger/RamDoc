use crate::error::AppError;
use crate::llm::{download, quantization, ModelChoice};
use crate::models::model::{self, Model};
use crate::state::{llm_lock_poisoned, AppState};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

/// Response for list_models command with additional context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub downloaded_at: String,
    pub last_used: Option<String>,
    pub is_default: bool,
    pub is_loaded: bool,
    pub exists_on_disk: bool,
    pub quantization_promotion: Option<quantization::QuantizationPromotionSummary>,
}

fn promotion_summary(
    model_path: &std::path::Path,
) -> Option<quantization::QuantizationPromotionSummary> {
    match quantization::promotion_summary_for_model(model_path) {
        Ok(summary) => summary,
        Err(error) => {
            // A malformed or tampered sidecar must never earn the UI badge, but
            // it should not make every other registered model disappear.
            log::warn!(
                "Ignoring invalid quantization promotion sidecar for '{}': {}",
                model_path.display(),
                error
            );
            None
        }
    }
}

/// List all registered models
#[tauri::command]
pub async fn list_models(state: State<'_, AppState>) -> Result<Vec<ModelInfo>, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;

    let models = model::list_models(&conn)?;

    // Check which model is currently loaded
    let loaded_filename = {
        let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
        llm.as_ref()
            .and_then(|engine| engine.status().downloaded_filename)
    };

    // Convert to ModelInfo with additional context
    let model_infos: Vec<ModelInfo> = models
        .into_iter()
        .map(|m| {
            let model_path = state.data_dir.join("models").join(&m.filename);
            ModelInfo {
                is_loaded: loaded_filename.as_ref() == Some(&m.filename),
                exists_on_disk: model_path.exists(),
                quantization_promotion: promotion_summary(&model_path),
                id: m.id,
                name: m.name,
                filename: m.filename,
                sha256: m.sha256,
                size_bytes: m.size_bytes,
                downloaded_at: m.downloaded_at,
                last_used: m.last_used,
                is_default: m.is_default,
            }
        })
        .collect();

    Ok(model_infos)
}

/// Get a single model by ID
#[tauri::command]
pub async fn get_model_info(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<ModelInfo, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;

    let m = model::get_model(&conn, &model_id)?;

    let loaded_filename = {
        let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
        llm.as_ref()
            .and_then(|engine| engine.status().downloaded_filename)
    };

    let model_path = state.data_dir.join("models").join(&m.filename);

    Ok(ModelInfo {
        is_loaded: loaded_filename.as_ref() == Some(&m.filename),
        exists_on_disk: model_path.exists(),
        quantization_promotion: promotion_summary(&model_path),
        id: m.id,
        name: m.name,
        filename: m.filename,
        sha256: m.sha256,
        size_bytes: m.size_bytes,
        downloaded_at: m.downloaded_at,
        last_used: m.last_used,
        is_default: m.is_default,
    })
}

/// Preview a promotion record without hashing the GGUF.
#[tauri::command]
pub fn inspect_promoted_model(
    promotion_path: String,
    artifact_path: Option<String>,
) -> Result<quantization::PromotionPreview, AppError> {
    if promotion_path.trim().is_empty() {
        return Err(AppError::Validation(
            "Quantization promotion path cannot be empty".to_string(),
        ));
    }
    let artifact = artifact_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(std::path::PathBuf::from);
    quantization::inspect_promotion(std::path::Path::new(&promotion_path), artifact.as_deref())
}

/// Import a GGUF that passed the offline clinical quantization promotion gate.
///
/// The GGUF may sit next to the promotion JSON, or the caller can pass an
/// explicit `artifact_path`. The blocking verifier copies into the app-owned
/// model directory while hashing, rejects any mismatch, and persists the
/// evidence sidecar before the model enters the registry.
#[tauri::command]
pub async fn import_promoted_model(
    app: AppHandle,
    state: State<'_, AppState>,
    promotion_path: String,
    artifact_path: Option<String>,
) -> Result<Model, AppError> {
    if promotion_path.trim().is_empty() {
        return Err(AppError::Validation(
            "Quantization promotion path cannot be empty".to_string(),
        ));
    }
    let source = std::path::PathBuf::from(promotion_path);
    let artifact = artifact_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(std::path::PathBuf::from);
    let destination_dir = state.data_dir.join("models");
    let installed = tokio::task::spawn_blocking(move || {
        quantization::install_promotion(
            &source,
            &destination_dir,
            artifact.as_deref(),
            |progress| {
                let _ = app.emit("promoted-model-import-progress", progress);
            },
        )
    })
    .await
    .map_err(|error| AppError::Llm(format!("promotion import task failed: {error}")))??;

    let filename = installed.record.artifact.filename.clone();
    let sha256 = installed.record.artifact.sha256.clone();
    let size_bytes = i64::try_from(installed.record.artifact.size_bytes)
        .map_err(|_| AppError::Validation("Promoted model is too large".to_string()))?;
    let display_name = installed.record.display_name.clone();

    let db = state.get_db()?;
    let conn = db.conn()?;
    match model::get_model_by_filename(&conn, &filename) {
        Ok(existing) => {
            if existing.sha256 != sha256 || existing.size_bytes != size_bytes {
                return Err(AppError::Validation(format!(
                    "Registered model '{}' has different content; refusing to replace it",
                    filename
                )));
            }
            Ok(existing)
        }
        Err(AppError::NotFound(_)) => {
            let model_id = Uuid::new_v4().to_string();
            model::create_model(
                &conn,
                &model_id,
                &display_name,
                &filename,
                &sha256,
                size_bytes,
            )
        }
        Err(error) => Err(error),
    }
}

/// Download and register a model
#[tauri::command]
pub async fn download_and_register_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model: ModelChoice,
) -> Result<Model, AppError> {
    // Validate filename
    if model.filename.is_empty() || model.filename.contains('/') || model.filename.contains('\\') {
        return Err(AppError::Validation("Invalid model filename".to_string()));
    }

    // Download the model first
    let dest_dir = state.data_dir.join("models");
    tokio::fs::create_dir_all(&dest_dir).await?;

    let dest_path = dest_dir.join(&model.filename);
    let url = download::model_url(&model.filename)?;

    // Download with progress and get verified SHA-256
    let sha256 =
        download::download_model_with_progress(&app, &url, &dest_path, &model.filename).await?;

    let size_bytes = tokio::fs::metadata(&dest_path).await?.len() as i64;

    // Register in database
    let db = state.get_db()?;
    let conn = db.conn()?;

    // Check if model already exists
    match model::get_model_by_filename(&conn, &model.filename) {
        Ok(existing) => {
            // Model already registered, just return it
            Ok(existing)
        }
        Err(AppError::NotFound(_)) => {
            // Create new model record
            let model_id = Uuid::new_v4().to_string();
            model::create_model(
                &conn,
                &model_id,
                &model.name,
                &model.filename,
                &sha256,
                size_bytes,
            )
        }
        Err(e) => Err(e),
    }
}

/// Delete a model (removes file and database record)
#[tauri::command]
pub async fn delete_model(state: State<'_, AppState>, model_id: String) -> Result<(), AppError> {
    // Share the swap coordinator with loading so a status snapshot cannot
    // delete a file while it is becoming resident.
    let _swap_lease = state.llm_swap.lock().await;
    if matches!(
        state.llm_lifecycle().phase,
        crate::llm::EngineLifecyclePhase::Loading | crate::llm::EngineLifecyclePhase::Unloading
    ) {
        return Err(AppError::Validation(
            "Cannot delete a model while its runtime lifecycle is changing".to_string(),
        ));
    }
    let db = state.get_db()?;

    // Use a block so conn (MutexGuard, not Send) is dropped before any await points
    let (model_path, promotion_path) = {
        let conn = db.conn()?;
        let model = model::get_model(&conn, &model_id)?;

        let is_loaded = {
            let llm = state.llm.lock().map_err(|_| llm_lock_poisoned())?;
            llm.as_ref()
                .and_then(|engine| engine.status().downloaded_filename)
                .as_ref()
                == Some(&model.filename)
        };

        if is_loaded {
            return Err(AppError::Validation(
                "Cannot delete currently loaded model. Please load a different model first."
                    .to_string(),
            ));
        }

        let model_path = state.data_dir.join("models").join(&model.filename);
        let promotion_path = quantization::promotion_path_for_model(&model_path);
        (model_path, promotion_path)
        // conn dropped here
    };

    // Delete the file (async — conn must not be held)
    if model_path.exists() {
        tokio::fs::remove_file(&model_path).await?;
    }
    if promotion_path.exists() {
        tokio::fs::remove_file(&promotion_path).await?;
    }

    // Re-acquire connection for database delete
    let conn = db.conn()?;
    model::delete_model(&conn, &model_id)?;

    Ok(())
}

/// Set a model as the default
#[tauri::command]
pub async fn set_default_model(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;

    let selected = model::get_model(&conn, &model_id)?;
    if !state
        .data_dir
        .join("models")
        .join(&selected.filename)
        .is_file()
    {
        return Err(AppError::Validation(format!(
            "Cannot use '{}' as the writing model because its file is missing",
            selected.name
        )));
    }

    model::set_default_model(&conn, &model_id)?;

    Ok(())
}

/// Get the default model
#[tauri::command]
pub async fn get_default_model(state: State<'_, AppState>) -> Result<Option<Model>, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;

    model::get_default_model(&conn)
}

/// Information about an available model (downloaded or not)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableModel {
    pub name: String,
    pub filename: String,
    pub size_bytes: u64,
    pub min_ram_gb: u64,
    pub description: String,
    pub context_window_tokens: u64,
    pub parameters: String,
    pub license: String,
    pub disclaimer: Option<String>,
    pub is_downloaded: bool,
    pub model_id: Option<String>,
}

/// List all available models from the whitelist with their requirements
#[tauri::command]
pub async fn list_available_models(
    state: State<'_, AppState>,
) -> Result<Vec<AvailableModel>, AppError> {
    // The download whitelist is the single source of truth for picker metadata.
    let available_models: Vec<AvailableModel> = download::MODELS
        .iter()
        .map(|entry| AvailableModel {
            name: entry.name.to_string(),
            filename: entry.filename.to_string(),
            size_bytes: entry.size_bytes,
            min_ram_gb: entry.min_ram_gb,
            description: entry.description.to_string(),
            context_window_tokens: entry.context_window_tokens,
            parameters: entry.parameters.to_string(),
            license: entry.license.to_string(),
            disclaimer: entry.disclaimer.map(str::to_string),
            is_downloaded: false,
            model_id: None,
        })
        .collect();

    // Check which models are already downloaded
    let db = state.get_db()?;
    let conn = db.conn()?;
    let installed_models = model::list_models(&conn)?;

    let mut result: Vec<AvailableModel> = available_models
        .into_iter()
        .map(|mut am| {
            // A registry row alone is not a usable download. Keep the picker
            // able to repair a model whose file was removed externally.
            if let Some(installed) = installed_models.iter().find(|m| m.filename == am.filename) {
                if state.data_dir.join("models").join(&installed.filename).is_file() {
                    am.is_downloaded = true;
                    am.model_id = Some(installed.id.clone());
                }
            }
            am
        })
        .collect();

    // Sort by RAM requirement (descending) so best models appear first.
    // Add deterministic tie-breakers for models with the same RAM requirement.
    result.sort_by(|a, b| {
        b.min_ram_gb
            .cmp(&a.min_ram_gb)
            .then_with(|| b.size_bytes.cmp(&a.size_bytes))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(result)
}
