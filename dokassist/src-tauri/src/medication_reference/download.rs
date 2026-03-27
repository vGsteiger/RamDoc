use futures_util::StreamExt;
use ring::digest::{Context as DigestContext, SHA256};
use std::path::Path;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use crate::error::AppError;

/// Hard maximum download size for the reference DB — 256 MiB.
const MAX_REF_DB_BYTES: u64 = 256 * 1024 * 1024;
/// Hard maximum for the .minisig file (they're < 200 bytes in practice).
const MAX_MINISIG_BYTES: usize = 4096;

/// Hardcoded download URLs for the medication reference DB and its detached minisign signature.
///
/// Both files are published as assets on the `medication-ref` GitHub release in this repo.
/// Update these constants whenever a new release is cut.
///
/// ⚠️  These are the ONLY URLs this module ever contacts — there is no user-supplied URL path.
const REF_DB_URL: &str =
    "https://github.com/vGsteiger/RamDoc/releases/latest/download/medication_ref_de.sqlite";
const REF_DB_SIG_URL: &str =
    "https://github.com/vGsteiger/RamDoc/releases/latest/download/medication_ref_de.sqlite.minisig";

/// Ed25519 public key used to verify the medication reference database.
///
/// This key was generated offline with `minisign -G`.  The matching private key is stored
/// **only** in GitHub Actions secrets (`MEDICATION_REF_PRIVATE_KEY`) and is never committed.
/// Replace the placeholder below with the real public key after the CI keypair is generated:
///
///   minisign -G -p medication_ref.pub -s medication_ref.key
///   cat medication_ref.pub   # copy the second line here
///
/// SECURITY NOTE: even if GitHub releases are fully compromised an attacker cannot forge a
/// valid signature without the private key, so the public key hardcoded here is the root of
/// trust for the reference DB.
const REF_DB_PUBLIC_KEY: &str = "RWSfnrRB0cL2sWFA/bAJbZa8mvXCcVjjVq6N50oz6KA65wW9MkM4Vjv9";

/// Download and verify the medication reference SQLite.
///
/// Steps:
/// 1. Download the `.minisig` signature file (tiny, always first).
/// 2. Download the SQLite file, streaming it to a temp path while computing SHA-256.
/// 3. Verify the minisign Ed25519 signature over the file bytes.
/// 4. Remove any existing DB file and rename the temp file to `dest_path` (atomic on most OS).
///
/// Emits `"medication-ref-download-progress"` (f64 0.0–1.0) during step 2.
pub async fn download_reference_db(app: &AppHandle, dest_path: &Path) -> Result<(), AppError> {
    let client = reqwest::Client::new();

    // Step 1 — fetch the detached signature (tiny file, get it first so we fail fast)
    let sig_bytes = fetch_signature(&client).await?;

    // Step 2 — stream the SQLite to a temp path beside the final destination
    let tmp_path = dest_path.with_extension("sqlite.tmp");
    let sha256_hex = stream_to_file(&client, app, &tmp_path)
        .await
        .inspect_err(|_e| {
            // Clean up on error
            let _ = std::fs::remove_file(&tmp_path);
        })?;

    // Step 3 — verify the minisign signature
    verify_minisign_signature(&sig_bytes, &sha256_hex, &tmp_path)
        .await
        .inspect_err(|_e| {
            let _ = std::fs::remove_file(&tmp_path);
        })?;

    // Step 4 — remove existing DB file if present, then atomic rename to final path
    if dest_path.exists() {
        std::fs::remove_file(dest_path).map_err(AppError::Filesystem)?;
    }
    tokio::fs::rename(&tmp_path, dest_path).await.map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        AppError::Filesystem(e)
    })?;

    log::info!(
        "Medication reference DB installed at '{}' (sha256={})",
        dest_path.display(),
        sha256_hex
    );
    let _ = app.emit("medication-ref-download-done", ());
    Ok(())
}

async fn fetch_signature(client: &reqwest::Client) -> Result<Vec<u8>, AppError> {
    let response = client
        .get(REF_DB_SIG_URL)
        .send()
        .await
        .map_err(|e| AppError::Validation(format!("Failed to fetch .minisig: {e}")))?
        .error_for_status()
        .map_err(|e| AppError::Validation(format!("Failed to fetch .minisig: {e}")))?;

    if let Some(len) = response.content_length() {
        if len > MAX_MINISIG_BYTES as u64 {
            return Err(AppError::Validation(format!(
                ".minisig response too large ({len} bytes)"
            )));
        }
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| AppError::Validation(format!("Failed to read .minisig body: {e}")))?;

    if bytes.len() > MAX_MINISIG_BYTES {
        return Err(AppError::Validation(
            ".minisig body exceeds size limit".to_string(),
        ));
    }

    Ok(bytes.to_vec())
}

async fn stream_to_file(
    client: &reqwest::Client,
    app: &AppHandle,
    tmp_path: &Path,
) -> Result<String, AppError> {
    let response = client
        .get(REF_DB_URL)
        .send()
        .await
        .map_err(|e| AppError::Validation(format!("Failed to start reference DB download: {e}")))?
        .error_for_status()
        .map_err(|e| AppError::Validation(format!("Failed to start reference DB download: {e}")))?;

    let total_size = response.content_length().unwrap_or(0);

    if total_size > MAX_REF_DB_BYTES {
        return Err(AppError::Validation(format!(
            "Reference DB declared size {total_size} bytes exceeds maximum {MAX_REF_DB_BYTES}"
        )));
    }

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(tmp_path)
        .await
        .map_err(AppError::Filesystem)?;

    let mut downloaded: u64 = 0;
    let mut sha256 = DigestContext::new(&SHA256);
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Validation(format!("Stream error: {e}")))?;

        downloaded += chunk.len() as u64;
        if downloaded > MAX_REF_DB_BYTES {
            return Err(AppError::Validation(
                "Reference DB exceeded maximum size during download — aborting".to_string(),
            ));
        }

        sha256.update(&chunk);
        file.write_all(&chunk).await.map_err(AppError::Filesystem)?;

        if total_size > 0 {
            let _ = app.emit(
                "medication-ref-download-progress",
                downloaded as f64 / total_size as f64,
            );
        }
    }

    file.flush().await.map_err(AppError::Filesystem)?;
    drop(file);

    let digest = sha256.finish();
    Ok(hex::encode(digest.as_ref()))
}

async fn verify_minisign_signature(
    sig_bytes: &[u8],
    _sha256_hex: &str,
    db_path: &Path,
) -> Result<(), AppError> {
    let sig_str = std::str::from_utf8(sig_bytes)
        .map_err(|_| AppError::Validation("Signature file is not valid UTF-8".to_string()))?;

    let public_key = minisign_verify::PublicKey::from_base64(REF_DB_PUBLIC_KEY)
        .map_err(|e| AppError::Validation(format!("Invalid hardcoded public key: {e}")))?;

    let signature = minisign_verify::Signature::decode(sig_str)
        .map_err(|e| AppError::Validation(format!("Failed to decode .minisig: {e}")))?;

    // NOTE: The minisign-verify crate requires the full file bytes for verification.
    // This means we must read the entire file into memory (up to MAX_REF_DB_BYTES).
    // While this causes a temporary memory spike, it's necessary for cryptographic
    // verification. The file is bounded to 256 MiB and verification is a one-time
    // operation during download/update, making this acceptable for the security benefit.
    let file_bytes = tokio::fs::read(db_path)
        .await
        .map_err(AppError::Filesystem)?;

    public_key
        .verify(&file_bytes, &signature, false)
        .map_err(|e| {
            AppError::Validation(format!(
                "Medication reference DB signature verification FAILED: {e}. \
                 The file has been removed. Possible tampering or corrupted download."
            ))
        })?;

    log::info!("Medication reference DB signature verified OK.");
    Ok(())
}
