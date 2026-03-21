use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::filesystem;
use crate::state::{AppState, AuthState};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackupInfo {
    pub schema_version: u32,
    pub created_at: String,
    pub db_schema_version: i32,
    pub file_count: usize,
}

/// Create an encrypted full-vault backup
///
/// This backs up:
/// - The entire SQLCipher database
/// - All encrypted files in the vault directory
/// - A manifest with checksums and metadata
///
/// The backup is encrypted with AES-256-GCM using a key derived from the
/// user's master password (via the BIP-39 mnemonic).
///
/// Returns the encrypted backup as a byte array.
#[tauri::command]
pub async fn create_vault_backup(state: State<'_, AppState>) -> Result<Vec<u8>, AppError> {
    // Get the database connection and file system key
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    let base_dir = state.data_dir.clone();

    // Get current database schema version
    let db_schema_version: i32 = conn.query_row("PRAGMA user_version;", [], |row| row.get(0))?;

    // Derive backup key from the fs_key (which is derived from the master password)
    // We use the fs_key as the backup encryption key for simplicity
    let backup_key: [u8; 32] = {
        let auth = state
            .auth
            .lock()
            .map_err(|_| AppError::Validation("Auth state mutex poisoned".to_string()))?;
        match &*auth {
            AuthState::Unlocked { fs_key, .. } => **fs_key,
            _ => return Err(AppError::AuthRequired),
        }
    };

    // Create the backup
    let encrypted_backup = filesystem::create_backup(&base_dir, &backup_key, db_schema_version)?;

    // Log the backup action
    audit::log(
        &conn,
        AuditAction::Export,
        "full_vault_backup",
        None,
        Some("Created encrypted full vault backup"),
    )?;

    Ok(encrypted_backup)
}

/// Restore a full-vault backup
///
/// WARNING: This is a DESTRUCTIVE operation that replaces ALL current data
/// with the backup contents. The caller MUST confirm with the user first.
///
/// Steps:
/// 1. Close existing database connection to avoid file locks
/// 2. Decrypt and validate the backup archive
/// 3. Verify all file checksums
/// 4. Check schema version compatibility
/// 5. Replace the database and vault directory
/// 6. Re-initialize the database connection
///
/// After restore, the app will need to restart to use the restored database.
#[tauri::command]
pub async fn restore_vault_backup(
    encrypted_backup: Vec<u8>,
    state: State<'_, AppState>,
) -> Result<BackupInfo, AppError> {
    let base_dir = state.data_dir.clone();

    // Get keys from current auth state before closing DB
    let (backup_key, db_key): ([u8; 32], [u8; 32]) = {
        let auth = state
            .auth
            .lock()
            .map_err(|_| AppError::Validation("Auth state mutex poisoned".to_string()))?;
        match &*auth {
            AuthState::Unlocked { fs_key, db_key, .. } => (**fs_key, **db_key),
            _ => return Err(AppError::AuthRequired),
        }
    };

    // Close the database connection before restore to avoid file lock issues
    {
        let mut db_lock = state.db.lock().map_err(|_| {
            AppError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some("Database state mutex poisoned".to_string()),
            ))
        })?;
        *db_lock = None;
    }

    // Perform the restore
    let manifest = filesystem::restore_backup(&base_dir, &encrypted_backup, &backup_key)?;

    // Re-initialize the database connection with the restored database
    state.init_db(&db_key)?;

    // Log the restore action
    let pool = state.get_db()?;
    let conn = pool.conn()?;
    audit::log(
        &conn,
        AuditAction::Export,
        "full_vault_restore",
        None,
        Some(&format!(
            "Restored full vault backup from {}",
            manifest.created_at
        )),
    )?;

    // Return backup info
    Ok(BackupInfo {
        schema_version: manifest.schema_version,
        created_at: manifest.created_at,
        db_schema_version: manifest.db_schema_version,
        file_count: manifest.checksums.len(),
    })
}

/// Validate a backup archive without restoring it
///
/// This decrypts the backup and verifies:
/// - The manifest can be parsed
/// - The schema version is supported
/// - All file checksums match
///
/// Returns backup metadata if validation succeeds.
#[tauri::command]
pub async fn validate_backup_archive(
    encrypted_backup: Vec<u8>,
    state: State<'_, AppState>,
) -> Result<BackupInfo, AppError> {
    // Get backup key from current auth state
    let backup_key: [u8; 32] = {
        let auth = state
            .auth
            .lock()
            .map_err(|_| AppError::Validation("Auth state mutex poisoned".to_string()))?;
        match &*auth {
            AuthState::Unlocked { fs_key, .. } => **fs_key,
            _ => return Err(AppError::AuthRequired),
        }
    };

    // Decrypt the archive
    let decrypted_backup = crate::crypto::decrypt(&backup_key, &encrypted_backup)?;

    // Open outer ZIP (contains manifest.json and vault.zip)
    use std::io::Cursor;
    use zip::ZipArchive;
    let cursor = Cursor::new(decrypted_backup);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| AppError::Validation(format!("Invalid backup archive: {}", e)))?;

    // Extract and parse manifest
    let manifest: filesystem::BackupManifest = {
        let mut manifest_file = archive
            .by_name("manifest.json")
            .map_err(|e| AppError::Validation(format!("Manifest not found: {}", e)))?;

        let mut manifest_data = Vec::new();
        std::io::copy(&mut manifest_file, &mut manifest_data)?;

        serde_json::from_slice(&manifest_data)
            .map_err(|e| AppError::Validation(format!("Invalid manifest: {}", e)))?
    };

    // Check schema version compatibility
    if manifest.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "Unsupported backup schema version: {}",
            manifest.schema_version
        )));
    }

    // Check database schema version compatibility
    if manifest.db_schema_version > 6 {
        return Err(AppError::Validation(format!(
            "Backup database schema version {} is newer than supported version 6",
            manifest.db_schema_version
        )));
    }

    // Verify all checksums in vault.zip
    let mut vault_zip_file = archive
        .by_name("vault.zip")
        .map_err(|e| AppError::Validation(format!("vault.zip not found: {}", e)))?;

    let mut vault_zip_data = Vec::new();
    std::io::copy(&mut vault_zip_file, &mut vault_zip_data)?;

    // Open vault.zip and verify checksums
    let vault_cursor = Cursor::new(vault_zip_data);
    let mut vault_archive = ZipArchive::new(vault_cursor)
        .map_err(|e| AppError::Validation(format!("Invalid vault.zip: {}", e)))?;

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

        // Compute and verify checksum
        use ring::digest::{digest, SHA256};
        let hash = digest(&SHA256, &file_data);
        let computed_checksum = hex::encode(hash.as_ref());

        let expected_checksum = manifest.checksums.get(&file_path).ok_or_else(|| {
            AppError::Validation(format!("Missing checksum for {} in manifest", file_path))
        })?;

        if &computed_checksum != expected_checksum {
            return Err(AppError::Validation(format!(
                "Checksum mismatch for {}: expected {}, got {}",
                file_path, expected_checksum, computed_checksum
            )));
        }
    }

    // Return backup info after successful validation
    Ok(BackupInfo {
        schema_version: manifest.schema_version,
        created_at: manifest.created_at,
        db_schema_version: manifest.db_schema_version,
        file_count: manifest.checksums.len(),
    })
}
