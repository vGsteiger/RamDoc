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

/// Maximum size we'll accept for an LFS pointer body (they're ~130 bytes).
const MAX_POINTER_BYTES: usize = 4096;

/// CRIT-3: Map each whitelisted GGUF filename to its HuggingFace raw-git LFS pointer URL.
/// The pointer file is a tiny text file (~130 bytes) containing the true SHA-256 of the blob,
/// served over TLS from HuggingFace's git metadata endpoint.
fn lfs_pointer_url(filename: &str) -> Option<&'static str> {
    match filename {
        "Qwen3-30B-A3B-Q4_K_M.gguf" => Some(
            "https://huggingface.co/unsloth/Qwen3-30B-A3B-GGUF/raw/main/Qwen3-30B-A3B-Q4_K_M.gguf",
        ),
        "Qwen3-8B-Q4_K_M.gguf" => Some(
            "https://huggingface.co/unsloth/Qwen3-8B-GGUF/raw/main/Qwen3-8B-Q4_K_M.gguf",
        ),
        "Phi-4-mini-instruct-Q4_K_M.gguf" => Some(
            "https://huggingface.co/unsloth/Phi-4-mini-instruct-GGUF/raw/main/Phi-4-mini-instruct-Q4_K_M.gguf",
        ),
        _ => None,
    }
}

/// CRIT-3: Fetch the expected SHA-256 digest from a HuggingFace LFS pointer file.
///
/// Git LFS pointer format:
/// ```
/// version https://git-lfs.github.com/spec/v1
/// oid sha256:<64-char-hex>
/// size <bytes>
/// ```
async fn fetch_lfs_sha256(client: &reqwest::Client, pointer_url: &str) -> Result<String, AppError> {
    let response = client
        .get(pointer_url)
        .send()
        .await
        .map_err(|e| AppError::Llm(format!("Failed to fetch LFS pointer: {e}")))?;

    // Reject unexpectedly large responses before reading the body.
    if let Some(len) = response.content_length() {
        if len > MAX_POINTER_BYTES as u64 {
            return Err(AppError::Validation(format!(
                "LFS pointer response too large ({len} bytes); expected ~130 bytes"
            )));
        }
    }

    let text = response
        .text()
        .await
        .map_err(|e| AppError::Llm(format!("Failed to read LFS pointer body: {e}")))?;

    // Guard against no Content-Length but still oversized body.
    if text.len() > MAX_POINTER_BYTES {
        return Err(AppError::Validation(
            "LFS pointer body too large".to_string(),
        ));
    }

    // Parse the "oid sha256:<hex>" line.
    for line in text.lines() {
        if let Some(hex) = line.strip_prefix("oid sha256:") {
            let hex = hex.trim();
            if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Ok(hex.to_lowercase());
            }
            return Err(AppError::Validation(format!(
                "LFS pointer contained malformed sha256 oid: '{hex}'"
            )));
        }
    }

    Err(AppError::Validation(
        "LFS pointer did not contain an 'oid sha256:' line".to_string(),
    ))
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
/// CRIT-3: Fetches the expected SHA-256 from HuggingFace's LFS pointer before downloading,
///         then verifies the completed file against it.
/// HIGH-2: Aborts download if total bytes exceed MAX_DOWNLOAD_BYTES.
pub async fn download_model_with_progress(
    app: &AppHandle,
    url: &str,
    dest_path: &Path,
    filename: &str,
) -> Result<(), AppError> {
    let client = reqwest::Client::new();

    // CRIT-3: Fetch expected SHA-256 from HuggingFace LFS pointer before the download begins.
    // This fails fast (before any large transfer) if the pointer is unavailable or malformed.
    let expected_hex = match lfs_pointer_url(filename) {
        Some(ptr_url) => {
            log::info!("Fetching LFS pointer for '{}'…", filename);
            Some(fetch_lfs_sha256(&client, ptr_url).await?)
        }
        None => {
            log::warn!(
                "No LFS pointer URL for '{}' — integrity check will be skipped",
                filename
            );
            None
        }
    };

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
        sha256.update(&existing_bytes);
    }

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Llm(format!("Stream error: {e}")))?;

        // HIGH-2: Hard cap — abort if we're receiving more data than expected
        downloaded += chunk.len() as u64;
        if downloaded > MAX_DOWNLOAD_BYTES {
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

    // CRIT-3: Verify SHA-256 digest against the value fetched from the LFS pointer
    let digest = sha256.finish();
    let computed_hex = hex::encode(digest.as_ref());

    match expected_hex {
        Some(ref expected) => {
            if computed_hex != *expected {
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
            log::warn!(
                "No expected SHA-256 for '{}' — skipping integrity check. Computed: {}",
                filename,
                computed_hex
            );
        }
    }

    let _ = app.emit("model-download-done", ());
    Ok(())
}
