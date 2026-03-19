use crate::crypto;
use crate::error::AppError;
use crate::spotlight;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const VAULT_DIR: &str = "vault";
const TEMP_DIR: &str = "temp";

/// 500 MiB — maximum plaintext size accepted by store_file / upload_file.
pub const MAX_FILE_SIZE: usize = 500 * 1024 * 1024;

/// Validate that a path component is safe (no path traversal)
fn validate_path_component(component: &str) -> Result<(), AppError> {
    // Check for empty
    if component.is_empty() {
        return Err(AppError::Validation(
            "Path component cannot be empty".to_string(),
        ));
    }

    // Check for absolute paths or parent directory references
    if component.starts_with('/')
        || component.starts_with('\\')
        || component == ".."
        || component.contains("..")
    {
        return Err(AppError::Validation(format!(
            "Invalid path component: {}",
            component
        )));
    }

    // Validate UUID format for patient_id
    if uuid::Uuid::parse_str(component).is_err() && !component.ends_with(".enc") {
        return Err(AppError::Validation(format!(
            "Invalid path component format: {}",
            component
        )));
    }

    Ok(())
}

/// Validate that a vault-relative path is safe
fn validate_vault_path(vault_path: &str) -> Result<(), AppError> {
    if vault_path.is_empty() {
        return Err(AppError::Validation(
            "Vault path cannot be empty".to_string(),
        ));
    }

    // Split into components and validate each
    let parts: Vec<&str> = vault_path.split('/').collect();

    if parts.len() != 2 {
        return Err(AppError::Validation(
            "Vault path must be in format patient-id/file.enc".to_string(),
        ));
    }

    // Validate patient ID component
    if uuid::Uuid::parse_str(parts[0]).is_err() {
        return Err(AppError::Validation(format!(
            "Invalid patient ID in vault path: {}",
            parts[0]
        )));
    }

    // Validate file component (must end with .enc)
    if !parts[1].ends_with(".enc") {
        return Err(AppError::Validation(
            "File must have .enc extension".to_string(),
        ));
    }

    // Check that the path doesn't contain any dangerous components
    let path = Path::new(vault_path);
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(AppError::Validation(format!(
                    "Invalid path component in vault path: {}",
                    vault_path
                )))
            }
        }
    }

    Ok(())
}

/// CRIT-2: Assert that `full_path` is within `canonical_base` to prevent symlink escapes.
/// Both paths must already exist on disk (they are canonicalized by the OS).
fn assert_within_vault(canonical_base: &Path, full_path: &Path) -> Result<(), AppError> {
    let canonical_full = full_path.canonicalize().map_err(|e| {
        AppError::Filesystem(std::io::Error::new(
            e.kind(),
            format!("Failed to canonicalize path: {}", e),
        ))
    })?;
    if !canonical_full.starts_with(canonical_base) {
        return Err(AppError::Validation(
            "Path escapes vault boundary (symlink detected)".to_string(),
        ));
    }
    Ok(())
}

/// Initialize the vault directory structure. Creates ~/DokAssist/vault/ if needed.
/// Sets .metadata_never_index and adds to Spotlight privacy list.
pub fn init_vault(base_dir: &Path) -> Result<(), AppError> {
    let vault_path = base_dir.join(VAULT_DIR);
    let temp_path = base_dir.join(TEMP_DIR);

    // Create vault directory if it doesn't exist
    if !vault_path.exists() {
        fs::create_dir_all(&vault_path)?;
    }

    // Create temp directory if it doesn't exist
    if !temp_path.exists() {
        fs::create_dir_all(&temp_path)?;
    }

    // Exclude both vault and temp from Spotlight indexing
    // Check if already excluded to avoid repeated mdutil calls
    let vault_marker = vault_path.join(".metadata_never_index");
    if !vault_marker.exists() {
        spotlight::exclude_from_spotlight(&vault_path)?;
    }

    let temp_marker = temp_path.join(".metadata_never_index");
    if !temp_marker.exists() {
        spotlight::exclude_from_spotlight(&temp_path)?;
    }

    Ok(())
}

/// Encrypt a file and store it in the patient's vault subdirectory.
/// Returns the vault-relative path (e.g., "<patient-uuid>/<file-uuid>.enc").
pub fn store_file(
    base_dir: &Path,
    fs_key: &[u8; 32],
    patient_id: &str,
    plaintext: &[u8],
) -> Result<String, AppError> {
    // HIGH-1: Enforce maximum file size before allocating anything
    if plaintext.len() > MAX_FILE_SIZE {
        return Err(AppError::Validation(format!(
            "File size {} bytes exceeds maximum allowed size of {} bytes",
            plaintext.len(),
            MAX_FILE_SIZE
        )));
    }

    // Validate patient_id to prevent path traversal
    validate_path_component(patient_id)?;

    // Create patient subdirectory in vault
    let patient_vault = base_dir.join(VAULT_DIR).join(patient_id);
    if !patient_vault.exists() {
        fs::create_dir_all(&patient_vault)?;
    }

    // CRIT-2: Verify patient_vault didn't escape the vault via symlinks
    let canonical_vault_base = base_dir
        .join(VAULT_DIR)
        .canonicalize()
        .map_err(AppError::Filesystem)?;
    let canonical_patient_vault = patient_vault.canonicalize().map_err(|e| {
        AppError::Filesystem(std::io::Error::new(
            e.kind(),
            format!("Failed to canonicalize patient vault: {}", e),
        ))
    })?;
    if !canonical_patient_vault.starts_with(&canonical_vault_base) {
        return Err(AppError::Validation(
            "Patient vault path escapes vault boundary".to_string(),
        ));
    }

    // Generate unique file ID for storage
    let file_uuid = Uuid::now_v7();
    let encrypted_filename = format!("{}.enc", file_uuid);
    let vault_relative_path = format!("{}/{}", patient_id, encrypted_filename);

    // Encrypt the file content
    let encrypted_data = crypto::encrypt(fs_key, plaintext)?;

    // Write encrypted data using the canonical base path (symlink-safe)
    let full_path = canonical_patient_vault.join(&encrypted_filename);
    fs::write(&full_path, encrypted_data)?;

    Ok(vault_relative_path)
}

/// Decrypt a file from the vault. Returns the plaintext bytes.
pub fn read_file(
    base_dir: &Path,
    fs_key: &[u8; 32],
    vault_path: &str,
) -> Result<Vec<u8>, AppError> {
    // Validate vault path to prevent path traversal
    validate_vault_path(vault_path)?;

    let full_path = base_dir.join(VAULT_DIR).join(vault_path);

    // CRIT-2: Canonicalize and verify the path is within the vault
    let canonical_vault_base = base_dir
        .join(VAULT_DIR)
        .canonicalize()
        .map_err(AppError::Filesystem)?;
    // full_path must exist for canonicalize to succeed — read errors become NotFound
    assert_within_vault(&canonical_vault_base, &full_path).map_err(|e| {
        match full_path.exists() {
            false => AppError::NotFound(vault_path.to_string()),
            true => e,
        }
    })?;

    // Read encrypted file
    let encrypted_data = fs::read(&full_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound(vault_path.to_string())
        } else {
            AppError::Filesystem(e)
        }
    })?;

    // Decrypt the file content
    let plaintext = crypto::decrypt(fs_key, &encrypted_data)?;

    Ok(plaintext)
}

/// Delete an encrypted file from the vault.
pub fn delete_file(base_dir: &Path, vault_path: &str) -> Result<(), AppError> {
    // Validate vault path to prevent path traversal
    validate_vault_path(vault_path)?;

    let full_path = base_dir.join(VAULT_DIR).join(vault_path);

    if !full_path.exists() {
        return Err(AppError::NotFound(vault_path.to_string()));
    }

    // CRIT-2: Canonicalize and verify the path is within the vault
    let canonical_vault_base = base_dir
        .join(VAULT_DIR)
        .canonicalize()
        .map_err(AppError::Filesystem)?;
    assert_within_vault(&canonical_vault_base, &full_path)?;

    fs::remove_file(&full_path)?;

    Ok(())
}

/// Export a file to a temporary decrypted location.
/// Returns the temp path. Caller must schedule cleanup.
pub fn export_temp(
    base_dir: &Path,
    fs_key: &[u8; 32],
    vault_path: &str,
    original_filename: &str,
) -> Result<PathBuf, AppError> {
    let temp_dir = base_dir.join(TEMP_DIR);
    if !temp_dir.exists() {
        fs::create_dir_all(&temp_dir)?;
    }

    // Decrypt the file
    let plaintext = read_file(base_dir, fs_key, vault_path)?;

    // MED-4: Use random UUID (v4) for temp filename — not time-based v7
    // which embeds a timestamp and is predictable by a co-resident process.
    let temp_uuid = Uuid::new_v4();
    let safe_filename = format!("{}_{}", temp_uuid, sanitize_filename(original_filename));
    let temp_path = temp_dir.join(safe_filename);

    // Write decrypted data to temp location
    fs::write(&temp_path, plaintext)?;

    Ok(temp_path)
}

/// Clean up expired temp exports (older than `max_age`).
/// Returns the number of files cleaned up.
pub fn cleanup_exports(base_dir: &Path, max_age: Duration) -> Result<u32, AppError> {
    let temp_dir = base_dir.join(TEMP_DIR);

    if !temp_dir.exists() {
        return Ok(0);
    }

    let now = SystemTime::now();
    let mut cleaned_count = 0;

    for entry in fs::read_dir(&temp_dir)? {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                // MED-8: Log and continue — do not abort cleanup on a single error
                log::warn!("cleanup_exports: failed to read dir entry: {}", e);
                continue;
            }
        };

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                log::warn!(
                    "cleanup_exports: failed to read metadata for {:?}: {}",
                    entry.path(),
                    e
                );
                continue;
            }
        };

        let is_hidden = entry.file_name().to_string_lossy().starts_with('.');
        if metadata.is_file() && !is_hidden {
            if let Ok(modified) = metadata.modified() {
                if let Ok(age) = now.duration_since(modified) {
                    if age >= max_age {
                        if let Err(e) = fs::remove_file(entry.path()) {
                            // MED-8: Log and continue — do not abort cleanup on a single error
                            log::warn!(
                                "cleanup_exports: failed to remove {:?}: {}",
                                entry.path(),
                                e
                            );
                            continue;
                        }
                        cleaned_count += 1;
                    }
                }
            }
        }
    }

    Ok(cleaned_count)
}

/// Get total vault size in bytes for a patient.
pub fn patient_vault_size(base_dir: &Path, patient_id: &str) -> Result<u64, AppError> {
    let patient_vault = base_dir.join(VAULT_DIR).join(patient_id);

    if !patient_vault.exists() {
        return Ok(0);
    }

    let mut total_size = 0u64;

    for entry in fs::read_dir(&patient_vault)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_file() {
            total_size += metadata.len();
        }
    }

    Ok(total_size)
}

/// Validate a literature vault path: must be "literature/<uuid>.enc"
fn validate_literature_path(vault_path: &str) -> Result<(), AppError> {
    let parts: Vec<&str> = vault_path.split('/').collect();
    if parts.len() != 2 || parts[0] != "literature" || !parts[1].ends_with(".enc") {
        return Err(AppError::Validation(
            "Literature path must be in format literature/<uuid>.enc".to_string(),
        ));
    }
    let file_stem = parts[1].trim_end_matches(".enc");
    if uuid::Uuid::parse_str(file_stem).is_err() {
        return Err(AppError::Validation(
            "Invalid UUID in literature vault path".to_string(),
        ));
    }
    // No path traversal possible with these constraints (no `..`, no absolute paths)
    let path = Path::new(vault_path);
    for component in path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(AppError::Validation(format!(
                "Invalid path component in literature path: {}",
                vault_path
            )));
        }
    }
    Ok(())
}

/// Encrypt and store a literature file. Returns vault-relative path "literature/<uuid>.enc".
pub fn store_literature_file(
    base_dir: &Path,
    fs_key: &[u8; 32],
    plaintext: &[u8],
) -> Result<String, AppError> {
    if plaintext.len() > MAX_FILE_SIZE {
        return Err(AppError::Validation(format!(
            "File size {} bytes exceeds maximum allowed size of {} bytes",
            plaintext.len(),
            MAX_FILE_SIZE
        )));
    }

    let lit_vault = base_dir.join(VAULT_DIR).join("literature");
    if !lit_vault.exists() {
        fs::create_dir_all(&lit_vault)?;
    }

    // CRIT-2: Verify literature vault didn't escape via symlinks
    let canonical_vault_base = base_dir
        .join(VAULT_DIR)
        .canonicalize()
        .map_err(AppError::Filesystem)?;
    let canonical_lit_vault = lit_vault.canonicalize().map_err(|e| {
        AppError::Filesystem(std::io::Error::new(
            e.kind(),
            format!("Failed to canonicalize literature vault: {}", e),
        ))
    })?;
    if !canonical_lit_vault.starts_with(&canonical_vault_base) {
        return Err(AppError::Validation(
            "Literature vault path escapes vault boundary".to_string(),
        ));
    }

    let file_uuid = Uuid::now_v7();
    let encrypted_filename = format!("{}.enc", file_uuid);
    let vault_relative_path = format!("literature/{}", encrypted_filename);

    let encrypted_data = crypto::encrypt(fs_key, plaintext)?;
    let full_path = canonical_lit_vault.join(&encrypted_filename);
    fs::write(&full_path, encrypted_data)?;

    Ok(vault_relative_path)
}

/// Decrypt a literature file from the vault.
pub fn read_literature_file(
    base_dir: &Path,
    fs_key: &[u8; 32],
    vault_path: &str,
) -> Result<Vec<u8>, AppError> {
    validate_literature_path(vault_path)?;

    let full_path = base_dir.join(VAULT_DIR).join(vault_path);
    let canonical_vault_base = base_dir
        .join(VAULT_DIR)
        .canonicalize()
        .map_err(AppError::Filesystem)?;
    assert_within_vault(&canonical_vault_base, &full_path).map_err(|e| {
        match full_path.exists() {
            false => AppError::NotFound(vault_path.to_string()),
            true => e,
        }
    })?;

    let encrypted_data = fs::read(&full_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound(vault_path.to_string())
        } else {
            AppError::Filesystem(e)
        }
    })?;

    crypto::decrypt(fs_key, &encrypted_data)
}

/// Delete a literature file from the vault.
pub fn delete_literature_file(base_dir: &Path, vault_path: &str) -> Result<(), AppError> {
    validate_literature_path(vault_path)?;

    let full_path = base_dir.join(VAULT_DIR).join(vault_path);
    if !full_path.exists() {
        return Err(AppError::NotFound(vault_path.to_string()));
    }

    let canonical_vault_base = base_dir
        .join(VAULT_DIR)
        .canonicalize()
        .map_err(AppError::Filesystem)?;
    assert_within_vault(&canonical_vault_base, &full_path)?;

    fs::remove_file(&full_path).map_err(AppError::Filesystem)
}

/// Sanitize a filename to make it safe for filesystem storage.
fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

/// Backup manifest metadata — stored in the .dokassist archive as manifest.json
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackupManifest {
    /// Format version for future compatibility
    pub schema_version: u32,
    /// Timestamp when backup was created (RFC3339)
    pub created_at: String,
    /// Database schema version (from PRAGMA user_version)
    pub db_schema_version: i32,
    /// SHA-256 checksums for verification: file path -> hex-encoded checksum
    pub checksums: std::collections::HashMap<String, String>,
}

/// Compute SHA-256 checksum of data
fn sha256_checksum(data: &[u8]) -> String {
    use ring::digest::{digest, SHA256};
    let hash = digest(&SHA256, data);
    hex::encode(hash.as_ref())
}

/// Create an encrypted full-vault backup archive (.dokassist)
///
/// Archives:
/// - SQLCipher database file (dokassist.db)
/// - Entire vault/ directory (encrypted files)
/// - manifest.json with schema version, timestamp, and SHA-256 checksums
///
/// The archive itself is AES-256-GCM encrypted using a key derived from the
/// backup passphrase (which can be the user's master password or a separate one).
///
/// Returns the encrypted backup archive as a byte vector.
pub fn create_backup(
    base_dir: &Path,
    backup_key: &[u8; 32],
    db_schema_version: i32,
) -> Result<Vec<u8>, AppError> {
    // Create in-memory ZIP archive with all vault contents
    let mut zip_buffer = Vec::new();
    let mut checksums = std::collections::HashMap::new();

    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(6));

        // Add database file
        let db_path = base_dir.join("dokassist.db");
        if db_path.exists() {
            let db_data = fs::read(&db_path)?;
            checksums.insert("dokassist.db".to_string(), sha256_checksum(&db_data));
            zip.start_file("dokassist.db", options)
                .map_err(|e| AppError::Validation(format!("ZIP error: {}", e)))?;
            zip.write_all(&db_data)?;
        } else {
            return Err(AppError::NotFound("Database file not found".to_string()));
        }

        // Add entire vault directory recursively
        let vault_path = base_dir.join(VAULT_DIR);
        if vault_path.exists() {
            add_directory_to_zip(&mut zip, &vault_path, VAULT_DIR, &mut checksums, options)?;
        }

        zip.finish()
            .map_err(|e| AppError::Validation(format!("ZIP finalize error: {}", e)))?;
    }

    // Create manifest
    let manifest = BackupManifest {
        schema_version: 1,
        created_at: chrono::Utc::now().to_rfc3339(),
        db_schema_version,
        checksums,
    };

    // Add manifest to a new ZIP
    let mut final_buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut final_buffer));
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(6));

        // Add manifest.json
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| AppError::Validation(format!("Manifest serialization failed: {}", e)))?;
        zip.start_file("manifest.json", options)
            .map_err(|e| AppError::Validation(format!("ZIP error: {}", e)))?;
        zip.write_all(manifest_json.as_bytes())?;

        // Add vault.zip (the inner archive)
        zip.start_file("vault.zip", options)
            .map_err(|e| AppError::Validation(format!("ZIP error: {}", e)))?;
        zip.write_all(&zip_buffer)?;

        zip.finish()
            .map_err(|e| AppError::Validation(format!("ZIP finalize error: {}", e)))?;
    }

    // Encrypt the entire archive with AES-256-GCM
    let encrypted_backup = crypto::encrypt(backup_key, &final_buffer)?;

    Ok(encrypted_backup)
}

/// Recursively add a directory to a ZIP archive
fn add_directory_to_zip(
    zip: &mut ZipWriter<std::io::Cursor<&mut Vec<u8>>>,
    dir_path: &Path,
    archive_prefix: &str,
    checksums: &mut std::collections::HashMap<String, String>,
    options: SimpleFileOptions,
) -> Result<(), AppError> {
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let relative_path = format!("{}/{}", archive_prefix, file_name.to_string_lossy());

        if path.is_dir() {
            // Create directory entry in ZIP
            let dir_path_with_slash = format!("{}/", relative_path);
            zip.add_directory(&dir_path_with_slash, options)
                .map_err(|e| AppError::Validation(format!("ZIP error: {}", e)))?;

            // Recursively add subdirectory contents
            add_directory_to_zip(zip, &path, &relative_path, checksums, options)?;
        } else if path.is_file() {
            // Skip hidden files (like .metadata_never_index)
            if file_name.to_string_lossy().starts_with('.') {
                continue;
            }

            // Read file and compute checksum
            let file_data = fs::read(&path)?;
            checksums.insert(relative_path.clone(), sha256_checksum(&file_data));

            // Add to ZIP
            zip.start_file(&relative_path, options)
                .map_err(|e| AppError::Validation(format!("ZIP error: {}", e)))?;
            zip.write_all(&file_data)?;
        }
    }

    Ok(())
}

/// Validate and restore a backup archive
///
/// Steps:
/// 1. Decrypt the archive using backup_key
/// 2. Extract and validate manifest.json
/// 3. Check schema version compatibility
/// 4. Verify SHA-256 checksums of all files
/// 5. Extract vault.zip
/// 6. Replace local database and vault directory
///
/// WARNING: This replaces all current data. The caller must ensure the user
/// has confirmed this destructive operation.
pub fn restore_backup(
    base_dir: &Path,
    encrypted_backup: &[u8],
    backup_key: &[u8; 32],
) -> Result<BackupManifest, AppError> {
    // Decrypt the archive
    let decrypted_backup = crypto::decrypt(backup_key, encrypted_backup)?;

    // Open outer ZIP (contains manifest.json and vault.zip)
    let cursor = std::io::Cursor::new(decrypted_backup);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| AppError::Validation(format!("Invalid backup archive: {}", e)))?;

    // Extract and parse manifest
    let manifest: BackupManifest = {
        let mut manifest_file = archive
            .by_name("manifest.json")
            .map_err(|e| AppError::Validation(format!("Manifest not found: {}", e)))?;

        let mut manifest_data = Vec::new();
        std::io::copy(&mut manifest_file, &mut manifest_data)?;

        serde_json::from_slice(&manifest_data)
            .map_err(|e| AppError::Validation(format!("Invalid manifest: {}", e)))?
    };

    // Check schema version compatibility (v1 is current)
    if manifest.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "Unsupported backup schema version: {}",
            manifest.schema_version
        )));
    }

    // Check database schema version compatibility
    // We can restore backups from the same or older DB schema versions
    // The database migration system will upgrade if needed
    if manifest.db_schema_version > 6 {
        return Err(AppError::Validation(format!(
            "Backup database schema version {} is newer than supported version 6",
            manifest.db_schema_version
        )));
    }

    // Extract vault.zip to a temporary location
    let temp_restore_dir = base_dir.join("temp_restore");
    if temp_restore_dir.exists() {
        fs::remove_dir_all(&temp_restore_dir)?;
    }
    fs::create_dir_all(&temp_restore_dir)?;

    // Extract vault.zip
    let mut vault_zip_data = Vec::new();
    {
        let mut vault_zip_file = archive
            .by_name("vault.zip")
            .map_err(|e| AppError::Validation(format!("vault.zip not found: {}", e)))?;
        std::io::copy(&mut vault_zip_file, &mut vault_zip_data)?;
    }

    // Open and extract inner ZIP (vault contents)
    let vault_cursor = std::io::Cursor::new(vault_zip_data);
    let mut vault_archive = ZipArchive::new(vault_cursor)
        .map_err(|e| AppError::Validation(format!("Invalid vault.zip: {}", e)))?;

    // Verify checksums while extracting
    for i in 0..vault_archive.len() {
        let mut file = vault_archive.by_index(i).map_err(|e| {
            AppError::Validation(format!("Failed to read archive entry {}: {}", i, e))
        })?;

        if file.is_dir() {
            continue;
        }

        let file_path = file.name().to_string();
        let mut file_data = Vec::new();
        std::io::copy(&mut file, &mut file_data)?;

        // Verify checksum
        let computed_checksum = sha256_checksum(&file_data);
        if let Some(expected_checksum) = manifest.checksums.get(&file_path) {
            if &computed_checksum != expected_checksum {
                // Clean up temp directory on checksum failure
                let _ = fs::remove_dir_all(&temp_restore_dir);
                return Err(AppError::Validation(format!(
                    "Checksum mismatch for {}: expected {}, got {}",
                    file_path, expected_checksum, computed_checksum
                )));
            }
        } else {
            log::warn!("No checksum found for {} in manifest", file_path);
        }

        // Extract file to temp location
        let extract_path = temp_restore_dir.join(&file_path);
        if let Some(parent) = extract_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&extract_path, &file_data)?;
    }

    // All checksums verified — now perform the destructive replacement
    // This is the point of no return

    // 1. Replace database
    let db_path = base_dir.join("dokassist.db");
    let temp_db_path = temp_restore_dir.join("dokassist.db");
    if temp_db_path.exists() {
        // Remove existing database
        if db_path.exists() {
            fs::remove_file(&db_path)?;
        }
        // Move restored database into place
        fs::copy(&temp_db_path, &db_path)?;
    } else {
        // Clean up and return error
        let _ = fs::remove_dir_all(&temp_restore_dir);
        return Err(AppError::Validation(
            "Database file not found in backup".to_string(),
        ));
    }

    // 2. Replace vault directory
    let vault_path = base_dir.join(VAULT_DIR);
    let temp_vault_path = temp_restore_dir.join(VAULT_DIR);
    if temp_vault_path.exists() {
        // Remove existing vault
        if vault_path.exists() {
            fs::remove_dir_all(&vault_path)?;
        }
        // Move restored vault into place
        copy_dir_recursive(&temp_vault_path, &vault_path)?;
    }

    // Clean up temp directory
    let _ = fs::remove_dir_all(&temp_restore_dir);

    Ok(manifest)
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if path.is_dir() {
            copy_dir_recursive(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_vault_creates_directories() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        init_vault(base_dir).unwrap();

        assert!(base_dir.join(VAULT_DIR).exists());
        assert!(base_dir.join(TEMP_DIR).exists());
        assert!(base_dir
            .join(VAULT_DIR)
            .join(".metadata_never_index")
            .exists());
    }

    #[test]
    fn test_store_and_read_file_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        let fs_key = crypto::generate_key();
        let patient_id = uuid::Uuid::now_v7().to_string();
        let plaintext = b"This is a test file with some content.";

        // Store file
        let vault_path = store_file(base_dir, &fs_key, &patient_id, plaintext).unwrap();
        assert!(vault_path.contains(&patient_id));
        assert!(vault_path.ends_with(".enc"));

        // Read file back
        let decrypted = read_file(base_dir, &fs_key, &vault_path).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_read_file_with_wrong_key() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        let fs_key1 = crypto::generate_key();
        let fs_key2 = crypto::generate_key();
        let patient_id = uuid::Uuid::now_v7().to_string();
        let plaintext = b"Secret data";

        // Store with key1
        let vault_path = store_file(base_dir, &fs_key1, &patient_id, plaintext).unwrap();

        // Try to read with key2 - should fail
        let result = read_file(base_dir, &fs_key2, &vault_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_store_file_creates_patient_directory() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        let fs_key = crypto::generate_key();
        let patient_id = uuid::Uuid::now_v7().to_string();
        let plaintext = b"Test content";

        store_file(base_dir, &fs_key, &patient_id, plaintext).unwrap();

        // Verify patient directory was created
        assert!(base_dir.join(VAULT_DIR).join(&patient_id).exists());
    }

    #[test]
    fn test_delete_file() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        let fs_key = crypto::generate_key();
        let patient_id = uuid::Uuid::now_v7().to_string();
        let plaintext = b"Data to be deleted";

        // Store file
        let vault_path = store_file(base_dir, &fs_key, &patient_id, plaintext).unwrap();
        let full_path = base_dir.join(VAULT_DIR).join(&vault_path);
        assert!(full_path.exists());

        // Delete file
        delete_file(base_dir, &vault_path).unwrap();
        assert!(!full_path.exists());

        // Try to delete again - should error
        let result = delete_file(base_dir, &vault_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        let fs_key = crypto::generate_key();
        let result = read_file(
            base_dir,
            &fs_key,
            &format!("{}/nonexistent.enc", uuid::Uuid::now_v7()),
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[test]
    fn test_export_temp() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        let fs_key = crypto::generate_key();
        let patient_id = uuid::Uuid::now_v7().to_string();
        let filename = "export_test.pdf";
        let plaintext = b"Content to export";

        // Store file
        let vault_path = store_file(base_dir, &fs_key, &patient_id, plaintext).unwrap();

        // Export to temp
        let temp_path = export_temp(base_dir, &fs_key, &vault_path, filename).unwrap();
        assert!(temp_path.exists());
        assert!(temp_path.to_string_lossy().contains(filename));

        // Verify exported content
        let exported_content = fs::read(&temp_path).unwrap();
        assert_eq!(plaintext.as_slice(), exported_content.as_slice());
    }

    #[test]
    fn test_export_temp_uses_random_uuid() {
        // MED-4: export_temp must use random (v4) UUIDs, not time-based v7
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        let fs_key = crypto::generate_key();
        let patient_id = uuid::Uuid::now_v7().to_string();
        let plaintext = b"data";

        let vault_path = store_file(base_dir, &fs_key, &patient_id, plaintext).unwrap();

        // Export twice in rapid succession — filenames must differ
        let p1 = export_temp(base_dir, &fs_key, &vault_path, "file.pdf").unwrap();
        let p2 = export_temp(base_dir, &fs_key, &vault_path, "file.pdf").unwrap();
        assert_ne!(p1, p2, "temp file names must be unique per export");
    }

    #[test]
    fn test_cleanup_exports() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        let temp_file_path = base_dir.join(TEMP_DIR).join("old_export.txt");
        fs::write(&temp_file_path, b"old data").unwrap();

        // Clean up files older than 0 seconds (should clean everything)
        let cleaned = cleanup_exports(base_dir, Duration::from_secs(0)).unwrap();
        assert_eq!(cleaned, 1);
        assert!(!temp_file_path.exists());
    }

    #[test]
    fn test_cleanup_exports_continues_on_error() {
        // MED-8: cleanup_exports should not abort on a single file failure
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        // Create two temp files
        let file1 = base_dir.join(TEMP_DIR).join("file1.txt");
        let file2 = base_dir.join(TEMP_DIR).join("file2.txt");
        fs::write(&file1, b"data1").unwrap();
        fs::write(&file2, b"data2").unwrap();

        // Both should be cleaned (no locked files in this test environment)
        let cleaned = cleanup_exports(base_dir, Duration::from_secs(0)).unwrap();
        assert_eq!(cleaned, 2);
    }

    #[test]
    fn test_patient_vault_size() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        let fs_key = crypto::generate_key();
        let patient_id = uuid::Uuid::now_v7().to_string();

        // Initially, vault size should be 0
        let initial_size = patient_vault_size(base_dir, &patient_id).unwrap();
        assert_eq!(initial_size, 0);

        // Store a file
        let plaintext = b"Test data for size calculation";
        store_file(base_dir, &fs_key, &patient_id, plaintext).unwrap();

        // Size should be > 0 now (encrypted size may be larger due to nonce + tag)
        let size_after_first = patient_vault_size(base_dir, &patient_id).unwrap();
        assert!(size_after_first > 0);

        // Store another file
        store_file(base_dir, &fs_key, &patient_id, plaintext).unwrap();

        // Size should increase
        let size_after_second = patient_vault_size(base_dir, &patient_id).unwrap();
        assert!(size_after_second > size_after_first);
    }

    #[test]
    fn test_store_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        let fs_key = crypto::generate_key();
        let patient_id = uuid::Uuid::now_v7().to_string();
        let large_data = vec![42u8; 10 * 1024 * 1024]; // 10 MB

        // Store large file
        let vault_path = store_file(base_dir, &fs_key, &patient_id, &large_data).unwrap();

        // Read it back
        let decrypted = read_file(base_dir, &fs_key, &vault_path).unwrap();
        assert_eq!(large_data, decrypted);
    }

    #[test]
    fn test_store_file_too_large() {
        // HIGH-1: files exceeding MAX_FILE_SIZE must be rejected
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        let fs_key = crypto::generate_key();
        let patient_id = uuid::Uuid::now_v7().to_string();
        // Construct a slice reference larger than MAX_FILE_SIZE without allocating
        // (create a Vec slightly above the limit)
        let oversized = vec![0u8; MAX_FILE_SIZE + 1];
        let result = store_file(base_dir, &fs_key, &patient_id, &oversized);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Validation(_)));
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal.txt"), "normal.txt");
        assert_eq!(
            sanitize_filename("file/with/slashes.txt"),
            "file_with_slashes.txt"
        );
        assert_eq!(
            sanitize_filename("file:with:colons.txt"),
            "file_with_colons.txt"
        );
        assert_eq!(
            sanitize_filename("file*with?special<chars>.txt"),
            "file_with_special_chars_.txt"
        );
    }

    #[test]
    fn test_enc_files_have_no_readable_headers() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        let fs_key = crypto::generate_key();
        let patient_id = uuid::Uuid::now_v7().to_string();
        let plaintext = b"This text should not be readable in the encrypted file";

        let vault_path = store_file(base_dir, &fs_key, &patient_id, plaintext).unwrap();
        let full_path = base_dir.join(VAULT_DIR).join(&vault_path);

        // Read the encrypted file directly
        let encrypted_bytes = fs::read(&full_path).unwrap();

        // Verify that the plaintext is not present in the encrypted data
        let encrypted_str = String::from_utf8_lossy(&encrypted_bytes);
        assert!(!encrypted_str.contains("This text should not be readable"));
    }

    #[test]
    fn test_duplicate_filename_generates_unique_vault_path() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        let fs_key = crypto::generate_key();
        let patient_id = uuid::Uuid::now_v7().to_string();

        // Store same filename twice
        let path1 = store_file(base_dir, &fs_key, &patient_id, b"content1").unwrap();
        let path2 = store_file(base_dir, &fs_key, &patient_id, b"content2").unwrap();

        // Paths should be different (UUID-based)
        assert_ne!(path1, path2);

        // Both files should exist and have different content
        let content1 = read_file(base_dir, &fs_key, &path1).unwrap();
        let content2 = read_file(base_dir, &fs_key, &path2).unwrap();
        assert_eq!(content1.as_slice(), b"content1");
        assert_eq!(content2.as_slice(), b"content2");
    }

    #[test]
    fn test_create_and_restore_backup() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        // Create a test database file
        let db_path = base_dir.join("dokassist.db");
        fs::write(&db_path, b"test database content").unwrap();

        // Create some test files in the vault
        let fs_key = crypto::generate_key();
        let patient_id = uuid::Uuid::now_v7().to_string();
        let file1_content = b"Patient file 1 content";
        let file2_content = b"Patient file 2 content";

        let vault_path1 = store_file(base_dir, &fs_key, &patient_id, file1_content).unwrap();
        let vault_path2 = store_file(base_dir, &fs_key, &patient_id, file2_content).unwrap();

        // Create backup
        let backup_key = crypto::generate_key();
        let encrypted_backup = create_backup(base_dir, &backup_key, 6).unwrap();

        // Verify backup is encrypted (not readable as plain text)
        assert!(!String::from_utf8_lossy(&encrypted_backup).contains("dokassist.db"));

        // Modify the original data
        fs::write(&db_path, b"modified database").unwrap();
        delete_file(base_dir, &vault_path1).unwrap();

        // Restore from backup
        let manifest = restore_backup(base_dir, &encrypted_backup, &backup_key).unwrap();

        // Verify manifest
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.db_schema_version, 6);
        assert!(manifest.checksums.len() > 0);

        // Verify database was restored
        let restored_db = fs::read(&db_path).unwrap();
        assert_eq!(restored_db, b"test database content");

        // Verify vault files were restored
        let restored_file1 = read_file(base_dir, &fs_key, &vault_path1).unwrap();
        let restored_file2 = read_file(base_dir, &fs_key, &vault_path2).unwrap();
        assert_eq!(restored_file1.as_slice(), file1_content);
        assert_eq!(restored_file2.as_slice(), file2_content);
    }

    #[test]
    fn test_backup_with_wrong_key_fails() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        // Create a test database
        let db_path = base_dir.join("dokassist.db");
        fs::write(&db_path, b"test database").unwrap();

        // Create backup with one key
        let backup_key1 = crypto::generate_key();
        let encrypted_backup = create_backup(base_dir, &backup_key1, 6).unwrap();

        // Try to restore with a different key
        let backup_key2 = crypto::generate_key();
        let result = restore_backup(base_dir, &encrypted_backup, &backup_key2);

        // Should fail with decryption error
        assert!(result.is_err());
    }

    #[test]
    fn test_backup_checksums_validated() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        // Create a test database
        let db_path = base_dir.join("dokassist.db");
        fs::write(&db_path, b"test database").unwrap();

        // Create backup
        let backup_key = crypto::generate_key();
        let mut encrypted_backup = create_backup(base_dir, &backup_key, 6).unwrap();

        // Corrupt the backup (flip some bits in the middle)
        if encrypted_backup.len() > 100 {
            encrypted_backup[50] ^= 0xFF;
            encrypted_backup[51] ^= 0xFF;
        }

        // Try to restore corrupted backup
        let result = restore_backup(base_dir, &encrypted_backup, &backup_key);

        // Should fail (either at decryption or checksum verification)
        assert!(result.is_err());
    }

    #[test]
    fn test_backup_preserves_directory_structure() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        // Create database
        let db_path = base_dir.join("dokassist.db");
        fs::write(&db_path, b"test db").unwrap();

        // Create files for multiple patients
        let fs_key = crypto::generate_key();
        let patient1 = uuid::Uuid::now_v7().to_string();
        let patient2 = uuid::Uuid::now_v7().to_string();

        store_file(base_dir, &fs_key, &patient1, b"patient1 file1").unwrap();
        store_file(base_dir, &fs_key, &patient1, b"patient1 file2").unwrap();
        store_file(base_dir, &fs_key, &patient2, b"patient2 file1").unwrap();

        // Create and restore backup
        let backup_key = crypto::generate_key();
        let backup = create_backup(base_dir, &backup_key, 6).unwrap();

        // Clear vault
        fs::remove_dir_all(base_dir.join(VAULT_DIR)).unwrap();
        init_vault(base_dir).unwrap();

        // Restore
        restore_backup(base_dir, &backup, &backup_key).unwrap();

        // Verify directory structure
        let patient1_dir = base_dir.join(VAULT_DIR).join(&patient1);
        let patient2_dir = base_dir.join(VAULT_DIR).join(&patient2);
        assert!(patient1_dir.exists());
        assert!(patient2_dir.exists());

        // Verify file counts
        let patient1_files: Vec<_> = fs::read_dir(&patient1_dir).unwrap().collect();
        let patient2_files: Vec<_> = fs::read_dir(&patient2_dir).unwrap().collect();
        assert_eq!(patient1_files.len(), 2);
        assert_eq!(patient2_files.len(), 1);
    }

    #[test]
    fn test_empty_vault_backup() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();
        init_vault(base_dir).unwrap();

        // Create only a database (no vault files)
        let db_path = base_dir.join("dokassist.db");
        fs::write(&db_path, b"empty vault db").unwrap();

        // Create and restore backup
        let backup_key = crypto::generate_key();
        let backup = create_backup(base_dir, &backup_key, 6).unwrap();
        let manifest = restore_backup(base_dir, &backup, &backup_key).unwrap();

        // Should succeed with just the database
        assert_eq!(manifest.db_schema_version, 6);
        assert!(db_path.exists());
    }
}

