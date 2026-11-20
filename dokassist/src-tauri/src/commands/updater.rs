use crate::error::AppError;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub body: Option<String>,
    pub date: Option<String>,
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateInfo, AppError> {
    let current_version = app.package_info().version.to_string();

    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => {
                log::info!(
                    "Update available: {} -> {}",
                    current_version,
                    update.version
                );
                Ok(UpdateInfo {
                    current_version: current_version.clone(),
                    latest_version: Some(update.version.clone()),
                    update_available: true,
                    body: update.body.clone(),
                    date: update.date.map(|d| d.to_string()),
                })
            }
            Ok(None) => {
                log::info!("No update available, current version: {}", current_version);
                Ok(UpdateInfo {
                    current_version,
                    latest_version: None,
                    update_available: false,
                    body: None,
                    date: None,
                })
            }
            Err(e) => {
                let msg = e.to_string();
                log::error!("Failed to check for updates: {}", msg);
                // Never report a failed check as "up to date": that hid a
                // release published without its latest.json (404) and left
                // users on old versions believing they were current.
                if is_manifest_unavailable(&msg) {
                    return Err(AppError::UpdateInfoUnavailable(msg));
                }
                // Signature/pubkey and other failures surface verbatim so the
                // user knows the update cannot be trusted.
                Err(AppError::Update(format!(
                    "Failed to check for updates: {}",
                    msg
                )))
            }
        },
        Err(e) => {
            log::error!("Failed to get updater: {}", e);
            Err(AppError::Update(format!("Updater not available: {}", e)))
        }
    }
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), AppError> {
    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => {
                log::info!("Installing update: {}", update.version);

                // Emit progress events
                let app_clone = app.clone();
                let mut downloaded = 0;

                match update
                    .download_and_install(
                        |chunk_length, content_length| {
                            downloaded += chunk_length;
                            if let Some(total) = content_length {
                                let progress = downloaded as f64 / total as f64;
                                let _ = app_clone.emit("updater-download-progress", progress);
                            }
                        },
                        || {
                            let _ = app_clone.emit("updater-download-complete", ());
                        },
                    )
                    .await
                {
                    Ok(_) => {
                        log::info!("Update installed successfully, restarting");
                        app.restart();
                    }
                    Err(e) => {
                        log::error!("Failed to download and install update: {}", e);
                        Err(AppError::Update(format!("Failed to install update: {}", e)))
                    }
                }
            }
            Ok(None) => Err(AppError::Update("No update available".to_string())),
            Err(e) => {
                log::error!("Failed to check for updates: {}", e);
                Err(AppError::Update(format!(
                    "Failed to check for updates: {}",
                    e
                )))
            }
        },
        Err(e) => {
            log::error!("Failed to get updater: {}", e);
            Err(AppError::Update(format!("Updater not available: {}", e)))
        }
    }
}

#[tauri::command]
pub fn get_app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

/// The update manifest (latest.json) could not be fetched or read: offline,
/// or the newest release is still being published (404). Signature and
/// public-key failures are deliberately excluded — they are security errors.
fn is_manifest_unavailable(msg: &str) -> bool {
    if msg.contains("pubkey") || msg.contains("signature") {
        return false;
    }
    msg.contains("404")
        || msg.contains("release JSON")
        || msg.contains("status code")
        || msg.contains("relative URL")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_manifest_is_reported_not_swallowed() {
        assert!(is_manifest_unavailable(
            "Could not fetch a valid release JSON from the remote"
        ));
        assert!(is_manifest_unavailable(
            "Download request failed with status code 404"
        ));
    }

    #[test]
    fn signature_failures_are_not_manifest_unavailable() {
        assert!(!is_manifest_unavailable(
            "The signature verification failed"
        ));
        assert!(!is_manifest_unavailable("invalid pubkey, status code 404"));
    }
}
