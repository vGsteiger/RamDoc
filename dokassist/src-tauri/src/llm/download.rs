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

/// Single source of truth for all whitelisted models.
/// Both the download URL and the LFS pointer URL are co-located so they
/// cannot diverge — adding a model in one place without the other is a
/// compile error (missing struct field).
struct ModelEntry {
    filename: &'static str,
    /// CRIT-4: HuggingFace blob URL (resolve/main) — used for the actual download.
    download_url: &'static str,
    /// CRIT-3: HuggingFace raw-git URL (raw/main) — returns the LFS pointer text
    /// (~130 bytes) containing the authoritative SHA-256 of the blob.
    lfs_pointer_url: &'static str,
}

const MODELS: &[ModelEntry] = &[
    ModelEntry {
        filename: "Qwen3-30B-A3B-Q4_K_M.gguf",
        download_url: "https://huggingface.co/unsloth/Qwen3-30B-A3B-GGUF/resolve/main/Qwen3-30B-A3B-Q4_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/unsloth/Qwen3-30B-A3B-GGUF/raw/main/Qwen3-30B-A3B-Q4_K_M.gguf",
    },
    ModelEntry {
        filename: "Qwen3-8B-Q4_K_M.gguf",
        download_url: "https://huggingface.co/unsloth/Qwen3-8B-GGUF/resolve/main/Qwen3-8B-Q4_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/unsloth/Qwen3-8B-GGUF/raw/main/Qwen3-8B-Q4_K_M.gguf",
    },
    ModelEntry {
        filename: "Phi-4-mini-instruct-Q4_K_M.gguf",
        download_url: "https://huggingface.co/unsloth/Phi-4-mini-instruct-GGUF/resolve/main/Phi-4-mini-instruct-Q4_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/unsloth/Phi-4-mini-instruct-GGUF/raw/main/Phi-4-mini-instruct-Q4_K_M.gguf",
    },
];

fn find_model(filename: &str) -> Option<&'static ModelEntry> {
    MODELS.iter().find(|m| m.filename == filename)
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
    parse_lfs_pointer_text(&text)
}

/// Parse an LFS pointer text body and return the lowercase hex SHA-256 digest.
/// Extracted for unit testability — the HTTP fetching stays in `fetch_lfs_sha256`.
fn parse_lfs_pointer_text(text: &str) -> Result<String, AppError> {
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
    find_model(filename)
        .map(|m| m.download_url.to_string())
        .ok_or_else(|| {
            AppError::Validation(format!(
                "Unknown model filename '{}'. Only explicitly whitelisted models may be downloaded.",
                filename
            ))
        })
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
    // Every filename that passes model_url() is guaranteed to have an lfs_pointer_url entry
    // in MODELS, so the error branch below is unreachable in normal operation.
    let model = find_model(filename).ok_or_else(|| {
        AppError::Validation(format!(
            "No model entry for '{}' — integrity check cannot proceed",
            filename
        ))
    })?;
    log::info!("Fetching LFS pointer for '{}'…", filename);
    let expected_hex = fetch_lfs_sha256(&client, model.lfs_pointer_url).await?;

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

    if computed_hex != expected_hex {
        let _ = tokio::fs::remove_file(dest_path).await;
        return Err(AppError::Validation(format!(
            "SHA-256 mismatch for '{}': expected {}, got {}. \
             File removed — possible MITM or corrupted download.",
            filename, expected_hex, computed_hex
        )));
    }
    log::info!("SHA-256 verified for '{}': {}", filename, computed_hex);

    let _ = app.emit("model-download-done", ());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- model_url whitelist ----

    #[test]
    fn test_model_url_known_filenames() {
        let cases = [
            (
                "Qwen3-30B-A3B-Q4_K_M.gguf",
                "resolve/main/Qwen3-30B-A3B-Q4_K_M.gguf",
            ),
            ("Qwen3-8B-Q4_K_M.gguf", "resolve/main/Qwen3-8B-Q4_K_M.gguf"),
            (
                "Phi-4-mini-instruct-Q4_K_M.gguf",
                "resolve/main/Phi-4-mini-instruct-Q4_K_M.gguf",
            ),
        ];
        for (filename, expected_suffix) in cases {
            let url = model_url(filename).unwrap();
            assert!(
                url.contains(expected_suffix),
                "URL for '{}' should contain '{}', got '{}'",
                filename,
                expected_suffix,
                url
            );
            assert!(
                url.starts_with("https://huggingface.co/"),
                "URL should be HF: {}",
                url
            );
        }
    }

    #[test]
    fn test_model_url_unknown_filename() {
        let result = model_url("evil-model.gguf");
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    // ---- parse_lfs_pointer_text ----

    fn valid_hex() -> String {
        "a".repeat(64)
    }

    #[test]
    fn test_parse_lfs_pointer_valid() {
        let hex = valid_hex();
        let text = format!(
            "version https://git-lfs.github.com/spec/v1\noid sha256:{}\nsize 1234567890\n",
            hex
        );
        let result = parse_lfs_pointer_text(&text).unwrap();
        assert_eq!(result, hex);
    }

    #[test]
    fn test_parse_lfs_pointer_extra_whitespace() {
        let hex = valid_hex();
        // Simulate Windows line endings — trailing \r on the hex line
        let text = format!(
            "version https://git-lfs.github.com/spec/v1\r\noid sha256:{}\r\nsize 1234\r\n",
            hex
        );
        let result = parse_lfs_pointer_text(&text).unwrap();
        assert_eq!(result, hex);
    }

    #[test]
    fn test_parse_lfs_pointer_missing_oid_line() {
        let text = "version https://git-lfs.github.com/spec/v1\nsize 1234\n";
        let result = parse_lfs_pointer_text(text);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn test_parse_lfs_pointer_malformed_hex_short() {
        // 63 chars instead of 64
        let hex = "a".repeat(63);
        let text = format!("oid sha256:{}\nsize 1234\n", hex);
        let result = parse_lfs_pointer_text(&text);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn test_parse_lfs_pointer_non_hex_chars() {
        // 64 chars but contains non-hex character 'Z'
        let hex = format!("{}Z{}", "a".repeat(32), "a".repeat(31));
        let text = format!("oid sha256:{}\nsize 1234\n", hex);
        let result = parse_lfs_pointer_text(&text);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn test_parse_lfs_pointer_body_too_large() {
        // Build a body that exceeds MAX_POINTER_BYTES (4096)
        let large_text = "x".repeat(MAX_POINTER_BYTES + 1);
        let result = parse_lfs_pointer_text(&large_text);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn test_parse_lfs_pointer_hex_returned_lowercase() {
        // Even if the hex were uppercase (non-standard), trim/to_lowercase is applied
        let hex = "A".repeat(64);
        let text = format!("oid sha256:{}\nsize 1234\n", hex);
        let result = parse_lfs_pointer_text(&text).unwrap();
        assert_eq!(result, "a".repeat(64));
    }
}
