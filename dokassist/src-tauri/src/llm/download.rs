use futures_util::StreamExt;
use reqwest::header::{CONTENT_RANGE, RANGE};
use ring::digest::{Context as DigestContext, SHA256};
use std::path::Path;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use crate::error::AppError;

/// CRIT-4: Hard maximum download size — 60 GiB.
/// Aborts the stream if the server sends more bytes than this.
const MAX_DOWNLOAD_BYTES: u64 = 60 * 1024 * 1024 * 1024;

/// CRIT-3: Expected SHA-256 digests (hex) for each known model file.
/// Hashes sourced from HuggingFace LFS object IDs on the model card pages.
/// Update this table when releasing a new model version.
fn expected_sha256(filename: &str) -> Option<&'static str> {
    match filename {
        "Qwen3-30B-A3B-Q4_K_M.gguf" => {
            Some("9f1a24700a339b09c06009b729b5c809e0b64c213b8af5b711b3dbdfd0c5ba48")
        }
        "Qwen3-8B-Q4_K_M.gguf" => {
            Some("120307ba529eb2439d6c430d94104dabd578497bc7bfe7e322b5d9933b449bd4")
        }
        "Phi-4-mini-instruct-Q4_K_M.gguf" => {
            Some("88c00229914083cd112853aab84ed51b87bdf6b9ce42f532d8c85c7c63b1730a")
        }
        _ => None,
    }
}

/// CRIT-4: Map a GGUF filename to its HuggingFace (Unsloth mirror) download URL.
/// Only explicitly whitelisted filenames are allowed — the fallback arm has been
/// removed to prevent SSRF and arbitrary URL construction.
pub fn model_url(filename: &str) -> Result<String, AppError> {
    match filename {
        "Qwen3-30B-A3B-Q4_K_M.gguf" => Ok(
            "https://huggingface.co/unsloth/Qwen3-30B-A3B-GGUF/resolve/main/Qwen3-30B-A3B-Q4_K_M.gguf"
                .to_string(),
        ),
        "Qwen3-8B-Q4_K_M.gguf" => Ok(
            "https://huggingface.co/unsloth/Qwen3-8B-GGUF/resolve/main/Qwen3-8B-Q4_K_M.gguf"
                .to_string(),
        ),
        "Phi-4-mini-instruct-Q4_K_M.gguf" => Ok(
            "https://huggingface.co/unsloth/Phi-4-mini-instruct-GGUF/resolve/main/Phi-4-mini-instruct-Q4_K_M.gguf"
                .to_string(),
        ),
        other => Err(AppError::Validation(format!(
            "Unknown model filename '{}'. Only explicitly whitelisted models may be downloaded.",
            other
        ))),
    }
}

/// Download a model file, resuming from where it left off if a partial file exists.
/// Emits `"model-download-progress"` (f64 0.0–1.0) and `"model-download-done"` Tauri events.
///
/// CRIT-3: Computes SHA-256 of downloaded bytes and verifies against the known digest.
/// HIGH-2: Aborts download if total bytes exceed MAX_DOWNLOAD_BYTES.
pub async fn download_model_with_progress(
    app: &AppHandle,
    url: &str,
    dest_path: &Path,
    filename: &str,
) -> Result<(), AppError> {
    let client = reqwest::Client::new();

    // Check for an existing partial download.
    let existing_size = if dest_path.exists() {
        tokio::fs::metadata(dest_path).await?.len()
    } else {
        0
    };

    let mut request = client.get(url);
    if existing_size > 0 {
        request = request.header(RANGE, format!("bytes={}-", existing_size));
    }

    let response = request
        .send()
        .await
        .map_err(|e| AppError::Llm(format!("Download request failed: {e}")))?;

    // Total size: from Content-Range when resuming, from Content-Length otherwise.
    let total_size = if existing_size > 0 {
        response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split('/').next_back())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    } else {
        response.content_length().unwrap_or(0)
    };

    // HIGH-2: Reject if the declared size already exceeds our cap
    if total_size > MAX_DOWNLOAD_BYTES {
        return Err(AppError::Validation(format!(
            "Declared content size {} bytes exceeds maximum allowed {} bytes",
            total_size, MAX_DOWNLOAD_BYTES
        )));
    }

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(existing_size > 0)
        .write(true)
        .open(dest_path)
        .await?;

    let mut downloaded = existing_size;
    let mut stream = response.bytes_stream();

    // CRIT-3: Hash context for the *entire* file (existing bytes + new bytes).
    // When resuming, re-hash already-downloaded bytes from disk first.
    let mut sha256 = DigestContext::new(&SHA256);
    if existing_size > 0 {
        let existing_bytes = tokio::fs::read(dest_path).await.map_err(|e| {
            AppError::Llm(format!("Failed to read partial download for hashing: {e}"))
        })?;
        // existing_bytes includes existing_size bytes
        sha256.update(&existing_bytes);
    }

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Llm(format!("Stream error: {e}")))?;

        // HIGH-2: Hard cap — abort if we're receiving more data than expected
        downloaded += chunk.len() as u64;
        if downloaded > MAX_DOWNLOAD_BYTES {
            // Remove the partial file so it isn't loaded
            let _ = tokio::fs::remove_file(dest_path).await;
            return Err(AppError::Validation(format!(
                "Download exceeded maximum size cap of {} bytes — aborting",
                MAX_DOWNLOAD_BYTES
            )));
        }

        // CRIT-3: Feed chunk into the running hash before writing
        sha256.update(&chunk);

        file.write_all(&chunk).await?;

        if total_size > 0 {
            let progress = downloaded as f64 / total_size as f64;
            let _ = app.emit("model-download-progress", progress);
        }
    }

    // Flush and close the file before verifying
    file.flush().await?;
    drop(file);

    // CRIT-3: Verify SHA-256 digest
    let digest = sha256.finish();
    let computed_hex = hex::encode(digest.as_ref());

    match expected_sha256(filename) {
        Some(expected) => {
            if computed_hex != expected {
                // Remove the corrupted/tampered file
                let _ = tokio::fs::remove_file(dest_path).await;
                return Err(AppError::Validation(format!(
                    "SHA-256 mismatch for '{}': expected {}, got {}. \
                     File removed — possible MITM or corrupted download.",
                    filename, expected, computed_hex
                )));
            }
            log::info!("SHA-256 verified for '{}': {}", filename, computed_hex);
        }
        None => {
            // Should not happen after CRIT-4 (model_url rejects unknown filenames),
            // but log a warning in case download_model_with_progress is called directly.
            log::warn!(
                "No expected SHA-256 for '{}' — skipping integrity check. \
                 Computed hash: {}",
                filename,
                computed_hex
            );
        }
    }

    let _ = app.emit("model-download-done", ());
    Ok(())
}
