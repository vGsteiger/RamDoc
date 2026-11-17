//! Tauri commands for local dictation (live microphone and recorded audio).

use crate::dictation::{self, models, worker::Transcript, ActiveDictation};
use crate::error::AppError;
use crate::state::{AppState, AuthState};
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, State};

static NEXT_DICTATION_ID: AtomicU64 = AtomicU64::new(1);

fn require_unlocked(state: &AppState) -> Result<(), AppError> {
    let auth = state
        .auth
        .lock()
        .map_err(|_| AppError::Dictation("Auth state poisoned".to_string()))?;
    if !matches!(*auth, AuthState::Unlocked { .. }) {
        return Err(AppError::AuthRequired);
    }
    Ok(())
}

fn dictation_poisoned() -> AppError {
    AppError::Dictation("Dictation state poisoned".to_string())
}

#[derive(Debug, Serialize)]
pub struct DictationStatus {
    pub models: Vec<models::AsrModelInfo>,
    /// The `dokassist-asr` worker binary is present in this installation.
    pub worker_available: bool,
    /// Ready to dictate: worker present and a model installed.
    pub ready: bool,
    /// Id of the live dictation in progress, if any.
    pub recording_id: Option<u64>,
}

#[tauri::command]
pub async fn get_dictation_status(state: State<'_, AppState>) -> Result<DictationStatus, AppError> {
    let models = models::list(&state.data_dir);
    let worker_available = dictation::worker::locate_worker().is_some();
    let recording_id = state
        .dictation
        .lock()
        .map_err(|_| dictation_poisoned())?
        .as_ref()
        .map(|d| d.id);
    Ok(DictationStatus {
        ready: worker_available && models.iter().any(|m| m.installed),
        models,
        worker_available,
        recording_id,
    })
}

#[tauri::command]
pub async fn download_dictation_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), AppError> {
    let entry = models::find(&model_id)
        .ok_or_else(|| AppError::Validation(format!("Unknown dictation model '{model_id}'")))?;
    if models::is_installed(&state.data_dir, entry) {
        return Ok(());
    }
    tokio::fs::create_dir_all(models::models_dir(&state.data_dir)).await?;
    let partial = models::partial_path(&state.data_dir, entry);
    crate::llm::download::download_pinned_file(
        &app,
        crate::llm::download::PinnedDownload {
            url: entry.download_url,
            lfs_pointer_url: entry.lfs_pointer_url,
            pinned_sha256: entry.pinned_sha256,
            filename: entry.filename,
            progress_event: "dictation-model-download-progress",
            done_event: "dictation-model-download-done",
        },
        &partial,
    )
    .await?;
    tokio::fs::rename(&partial, models::model_path(&state.data_dir, entry)).await?;
    log::info!("Dictation model '{}' installed", entry.id);
    Ok(())
}

#[tauri::command]
pub async fn delete_dictation_model(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), AppError> {
    let entry = models::find(&model_id)
        .ok_or_else(|| AppError::Validation(format!("Unknown dictation model '{model_id}'")))?;
    for path in [
        models::model_path(&state.data_dir, entry),
        models::partial_path(&state.data_dir, entry),
    ] {
        match tokio::fs::remove_file(&path).await {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn set_active_dictation_model(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), AppError> {
    models::set_active_model(&state.data_dir, &model_id)
}

#[derive(Clone, Serialize)]
struct LevelEvent {
    id: u64,
    level: f32,
}

/// Open the microphone and start loading the model in the worker. Returns
/// the dictation id to pass to `stop_dictation` / `cancel_dictation`.
/// Emits `dictation-level` (`{ id, level }`, 0–1) about ten times a second.
#[tauri::command]
pub async fn start_dictation(app: AppHandle, state: State<'_, AppState>) -> Result<u64, AppError> {
    require_unlocked(&state)?;
    if state
        .dictation
        .lock()
        .map_err(|_| dictation_poisoned())?
        .is_some()
    {
        return Err(AppError::Dictation(
            "A dictation is already in progress".into(),
        ));
    }

    let data_dir = state.data_dir.clone();
    let id = NEXT_DICTATION_ID.fetch_add(1, Ordering::Relaxed);

    let active = tokio::task::spawn_blocking(move || -> Result<ActiveDictation, AppError> {
        let job = dictation::prepare_job(&data_dir)?;
        let recorder = dictation::capture::Recorder::start(move |level| {
            let _ = app.emit("dictation-level", LevelEvent { id, level });
        })?;
        Ok(ActiveDictation { id, recorder, job })
    })
    .await
    .map_err(|e| AppError::Dictation(format!("spawn_blocking error: {e}")))??;

    let mut slot = state.dictation.lock().map_err(|_| dictation_poisoned())?;
    if slot.is_some() {
        // Lost a race with a concurrent start; dropping `active` cleans up.
        return Err(AppError::Dictation(
            "A dictation is already in progress".into(),
        ));
    }
    *slot = Some(active);
    Ok(id)
}

#[derive(Debug, Serialize)]
pub struct DictationResult {
    pub text: String,
    pub language: Option<String>,
    pub audio_ms: u64,
    /// The recording hit the 30 minute cap and was cut.
    pub truncated: bool,
}

/// Stop recording and return the transcript.
#[tauri::command]
pub async fn stop_dictation(
    state: State<'_, AppState>,
    id: u64,
) -> Result<DictationResult, AppError> {
    let active = {
        let mut slot = state.dictation.lock().map_err(|_| dictation_poisoned())?;
        match slot.as_ref() {
            Some(d) if d.id == id => slot.take().expect("checked above"),
            _ => {
                return Err(AppError::Dictation(
                    "This dictation is no longer active".into(),
                ))
            }
        }
    };
    require_unlocked(&state)?;

    let (transcript, truncated) =
        tokio::task::spawn_blocking(move || -> Result<(Transcript, bool), AppError> {
            let ActiveDictation { recorder, job, .. } = active;
            let recording = recorder.stop();
            let pcm = dictation::audio::resample_to_16k(&recording.samples, recording.sample_rate)?;
            Ok((job.transcribe(&pcm, |_, _| {})?, recording.truncated))
        })
        .await
        .map_err(|e| AppError::Dictation(format!("spawn_blocking error: {e}")))??;

    // The app may have been locked while we were transcribing.
    require_unlocked(&state)?;
    Ok(DictationResult {
        text: transcript.text,
        language: transcript.language,
        audio_ms: transcript.audio_ms,
        truncated,
    })
}

/// Discard the live dictation (if `id` matches, or any when `id` is omitted).
#[tauri::command]
pub async fn cancel_dictation(state: State<'_, AppState>, id: Option<u64>) -> Result<(), AppError> {
    let active = {
        let mut slot = state.dictation.lock().map_err(|_| dictation_poisoned())?;
        match (slot.as_ref(), id) {
            (Some(d), Some(id)) if d.id != id => None,
            _ => slot.take(),
        }
    };
    // Drop off the async runtime: joins the capture thread and kills the worker.
    if let Some(active) = active {
        tokio::task::spawn_blocking(move || drop(active))
            .await
            .map_err(|e| AppError::Dictation(format!("spawn_blocking error: {e}")))?;
    }
    Ok(())
}
