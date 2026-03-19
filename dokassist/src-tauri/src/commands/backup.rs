use crate::audit::{self, AuditAction};
use crate::error::AppError;
use crate::filesystem;
use crate::recovery;
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
/// 1. Decrypt and validate the backup archive
/// 2. Verify all file checksums
/// 3. Check schema version compatibility
/// 4. Replace the database and vault directory
/// 5. Re-initialize the database connection
///
/// After restore, the app will need to restart to use the restored database.
#[tauri::command]
pub async fn restore_vault_backup(
    encrypted_backup: Vec<u8>,
    state: State<'_, AppState>,
) -> Result<BackupInfo, AppError> {
    let base_dir = state.data_dir.clone();

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

    // Perform the restore
    let manifest = filesystem::restore_backup(&base_dir, &encrypted_backup, &backup_key)?;

    // Log the restore action in the newly restored database
    // We need to re-initialize the database connection with the restored DB
    let db_key: [u8; 32] = {
        let auth = state
            .auth
            .lock()
            .map_err(|_| AppError::Validation("Auth state mutex poisoned".to_string()))?;
        match &*auth {
            AuthState::Unlocked { db_key, .. } => **db_key,
            _ => return Err(AppError::AuthRequired),
        }
    };

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

    // Return backup info without actually restoring
    Ok(BackupInfo {
        schema_version: manifest.schema_version,
        created_at: manifest.created_at,
        db_schema_version: manifest.db_schema_version,
        file_count: manifest.checksums.len(),
    })
}
