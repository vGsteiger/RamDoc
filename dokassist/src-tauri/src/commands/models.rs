use crate::error::AppError;
use crate::llm::{download, LlmEngine, ModelChoice};
use crate::models::model::{self, Model, TaskModel, TaskType};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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
}

/// List all registered models
#[tauri::command]
pub async fn list_models(state: State<'_, AppState>) -> Result<Vec<ModelInfo>, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;

    let models = model::list_models(&conn)?;

    // Check which model is currently loaded
    let loaded_filename = {
        let llm = state.llm.lock().unwrap();
        llm.as_ref().and_then(|engine| {
            Some(engine.status().downloaded_filename?)
        })
    };

    // Convert to ModelInfo with additional context
    let model_infos: Vec<ModelInfo> = models
        .into_iter()
        .map(|m| {
            let model_path = state.data_dir.join("models").join(&m.filename);
            ModelInfo {
                is_loaded: loaded_filename.as_ref() == Some(&m.filename),
                exists_on_disk: model_path.exists(),
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
        let llm = state.llm.lock().unwrap();
        llm.as_ref().and_then(|engine| {
            Some(engine.status().downloaded_filename?)
        })
    };

    let model_path = state.data_dir.join("models").join(&m.filename);

    Ok(ModelInfo {
        is_loaded: loaded_filename.as_ref() == Some(&m.filename),
        exists_on_disk: model_path.exists(),
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

/// Download and register a model
#[tauri::command]
pub async fn download_and_register_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model: ModelChoice,
) -> Result<Model, AppError> {
    // Validate filename
    if model.filename.is_empty() || model.filename.contains('/') || model.filename.contains('\\') {
        return Err(AppError::Validation(
            "Invalid model filename".to_string(),
        ));
    }

    // Download the model first
    let dest_dir = state.data_dir.join("models");
    tokio::fs::create_dir_all(&dest_dir).await?;

    let dest_path = dest_dir.join(&model.filename);
    let url = download::model_url(&model.filename)?;

    // Download with progress
    download::download_model_with_progress(&app, &url, &dest_path, &model.filename).await?;

    // After successful download, get the actual SHA-256 from the file
    let sha256 = {
        use ring::digest::{Context as DigestContext, SHA256};
        let bytes = tokio::fs::read(&dest_path).await?;
        let mut context = DigestContext::new(&SHA256);
        context.update(&bytes);
        let digest = context.finish();
        hex::encode(digest.as_ref())
    };

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
pub async fn delete_model(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;

    // Get the model to find its filename
    let model = model::get_model(&conn, &model_id)?;

    // Check if this model is currently loaded
    let is_loaded = {
        let llm = state.llm.lock().unwrap();
        llm.as_ref()
            .and_then(|engine| engine.status().downloaded_filename)
            .as_ref()
            == Some(&model.filename)
    };

    if is_loaded {
        return Err(AppError::Validation(
            "Cannot delete currently loaded model. Please load a different model first.".to_string(),
        ));
    }

    // Delete the file
    let model_path = state.data_dir.join("models").join(&model.filename);
    if model_path.exists() {
        tokio::fs::remove_file(&model_path).await?;
    }

    // Delete from database
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

/// Set the model for a specific task type
#[tauri::command]
pub async fn set_task_model(
    state: State<'_, AppState>,
    task_type: String,
    model_id: String,
) -> Result<(), AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;

    let task = TaskType::from_str(&task_type)?;
    model::set_task_model(&conn, task, &model_id)?;

    Ok(())
}

/// Get the model assigned to a specific task type
#[tauri::command]
pub async fn get_task_model(
    state: State<'_, AppState>,
    task_type: String,
) -> Result<Option<Model>, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;

    let task = TaskType::from_str(&task_type)?;
    model::get_task_model(&conn, task)
}

/// List all task model assignments
#[tauri::command]
pub async fn list_task_models(state: State<'_, AppState>) -> Result<Vec<TaskModel>, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;

    model::list_task_models(&conn)
}

/// Clear the model assignment for a specific task type
#[tauri::command]
pub async fn clear_task_model(
    state: State<'_, AppState>,
    task_type: String,
) -> Result<(), AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;

    let task = TaskType::from_str(&task_type)?;
    model::clear_task_model(&conn, task)?;

    Ok(())
}

/// Get the appropriate model for a given task type
/// Falls back to default model if no task-specific model is set
#[tauri::command]
pub async fn get_model_for_task(
    state: State<'_, AppState>,
    task_type: String,
) -> Result<Option<Model>, AppError> {
    let db = state.get_db()?;
    let conn = db.conn()?;

    let task = TaskType::from_str(&task_type)?;

    // Try to get task-specific model
    if let Some(model) = model::get_task_model(&conn, task)? {
        return Ok(Some(model));
    }

    // Fall back to default model
    model::get_default_model(&conn)
}
