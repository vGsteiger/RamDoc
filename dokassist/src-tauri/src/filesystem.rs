use crate::crypto;
use crate::error::AppError;
use crate::spotlight;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};
use uuid::Uuid;

const VAULT_DIR: &str = "vault";
const TEMP_DIR: &str = "temp";

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
    // Validate patient_id to prevent path traversal
    validate_path_component(patient_id)?;

    // Create patient subdirectory in vault
    let patient_vault = base_dir.join(VAULT_DIR).join(patient_id);
    if !patient_vault.exists() {
        fs::create_dir_all(&patient_vault)?;
    }

    // Generate unique file ID for storage
    let file_uuid = Uuid::now_v7();
    let encrypted_filename = format!("{}.enc", file_uuid);
    let vault_relative_path = format!("{}/{}", patient_id, encrypted_filename);
    let full_path = patient_vault.join(&encrypted_filename);

    // Encrypt the file content
    let encrypted_data = crypto::encrypt(fs_key, plaintext)?;

    // Write encrypted data to disk
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

    // Generate unique temp filename to avoid collisions
    let temp_uuid = Uuid::now_v7();
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
        let entry = entry?;
        let metadata = entry.metadata()?;

        let is_hidden = entry
            .file_name()
            .to_string_lossy()
            .starts_with('.');
        if metadata.is_file() && !is_hidden {
            if let Ok(modified) = metadata.modified() {
                if let Ok(age) = now.duration_since(modified) {
                    if age >= max_age {
                        fs::remove_file(entry.path())?;
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
}
