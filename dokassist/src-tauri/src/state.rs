#[cfg(target_os = "macos")]
use crate::constants::KEYCHAIN_SERVICE;
use crate::constants::RECOVERY_FILENAME;
use crate::database::DbPool;
use crate::llm::{embed::EmbedEngine, LlmEngine};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Application state shared across all Tauri commands.
pub struct AppState {
    pub auth: Mutex<AuthState>,
    pub data_dir: std::path::PathBuf,
    pub db: Mutex<Option<DbPool>>,
    pub llm: Mutex<Option<Arc<LlmEngine>>>,
    /// Embedding engine for semantic search.  Populated lazily by `process_file`.
    pub embed: Mutex<Option<Arc<Mutex<EmbedEngine>>>>,
    /// Unencrypted medication reference SQLite (public AIPS data).
    /// `None` until the user downloads the reference DB via settings.
    pub medication_ref: Mutex<Option<Connection>>,
}

pub enum AuthState {
    FirstRun,
    Locked,
    Unlocked {
        db_key: zeroize::Zeroizing<[u8; 32]>,
        fs_key: zeroize::Zeroizing<[u8; 32]>,
    },
    RecoveryRequired,
}

impl AppState {
    pub fn new(data_dir: std::path::PathBuf) -> Self {
        // Determine initial auth state based on keychain and vault file existence
        let initial_state = determine_initial_auth_state(&data_dir);

        // Open the medication reference DB if it has already been downloaded.
        let ref_db_path = data_dir.join("medication_ref.sqlite");
        let medication_ref = if ref_db_path.exists() {
            match crate::medication_reference::open_reference_db(&ref_db_path) {
                Ok(conn) => {
                    log::info!(
                        "Medication reference DB loaded from '{}'",
                        ref_db_path.display()
                    );
                    Some(conn)
                }
                Err(e) => {
                    log::warn!("Failed to open medication reference DB: {e}");
                    None
                }
            }
        } else {
            None
        };

        Self {
            auth: Mutex::new(initial_state),
            data_dir,
            db: Mutex::new(None),
            llm: Mutex::new(None),
            embed: Mutex::new(None),
            medication_ref: Mutex::new(medication_ref),
        }
    }

    /// Initialize database after unlock with encryption key
    pub fn init_db(&self, key: &[u8; 32]) -> Result<(), crate::error::AppError> {
        let db_path = self.data_dir.join("dokassist.db");
        let pool = crate::database::init_db(&db_path, key)?;

        let mut db_lock = self.db.lock().map_err(|_| {
            crate::error::AppError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some("Database state mutex poisoned".to_string()),
            ))
        })?;
        *db_lock = Some(pool);

        Ok(())
    }

    /// Get database connection (requires unlock)
    pub fn get_db(&self) -> Result<DbPool, crate::error::AppError> {
        // Check auth state first
        let auth = self.auth.lock().map_err(|_| {
            crate::error::AppError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some("Auth state mutex poisoned".to_string()),
            ))
        })?;

        if !matches!(*auth, AuthState::Unlocked { .. }) {
            return Err(crate::error::AppError::AuthRequired);
        }
        drop(auth);

        // Then get database pool
        let db_lock = self.db.lock().map_err(|_| {
            crate::error::AppError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some("Database state mutex poisoned".to_string()),
            ))
        })?;
        db_lock
            .as_ref()
            .cloned()
            .ok_or(crate::error::AppError::AuthRequired)
    }

    /// Clear database pool on lock
    pub fn clear_db(&self) -> Result<(), crate::error::AppError> {
        let mut db_lock = self.db.lock().map_err(|_| {
            crate::error::AppError::Database(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some("Database state mutex poisoned".to_string()),
            ))
        })?;
        *db_lock = None;
        Ok(())
    }

    /// Return a cloned Arc to the embed engine if it has been initialised.
    /// Returns `None` if `process_file` has not yet populated the engine.
    pub fn try_get_embed(&self) -> Option<Arc<Mutex<EmbedEngine>>> {
        self.embed
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(Arc::clone))
    }

    /// Store an embed engine in state (no-op if one is already present).
    pub fn set_embed(&self, engine: EmbedEngine) -> Result<(), crate::error::AppError> {
        let mut guard = self
            .embed
            .lock()
            .map_err(|_| crate::error::AppError::Llm("Embed mutex poisoned".to_string()))?;
        if guard.is_none() {
            *guard = Some(Arc::new(Mutex::new(engine)));
        }
        Ok(())
    }

    /// Drop the embed engine on lock / reset.
    pub fn clear_embed(&self) {
        if let Ok(mut g) = self.embed.lock() {
            *g = None;
        }
    }

    /// Drop the LLM engine on app close / reset.
    pub fn clear_llm(&self) {
        if let Ok(mut g) = self.llm.lock() {
            *g = None;
        }
    }

    /// Acquire a lock on the medication reference DB connection, if installed.
    pub fn get_medication_ref(&self) -> Option<std::sync::MutexGuard<'_, Option<Connection>>> {
        self.medication_ref.lock().ok()
    }

    /// Replace the medication reference DB connection after a fresh download.
    pub fn set_medication_ref(&self, conn: Connection) -> Result<(), crate::error::AppError> {
        let mut guard = self.medication_ref.lock().map_err(|_| {
            crate::error::AppError::Validation("Medication ref mutex poisoned".to_string())
        })?;
        *guard = Some(conn);
        Ok(())
    }
}

fn determine_initial_auth_state(data_dir: &std::path::Path) -> AuthState {
    let vault_path = data_dir.join(RECOVERY_FILENAME);
    let vault_exists = vault_path.exists();

    // Check if keys exist in keychain (macOS only)
    #[cfg(target_os = "macos")]
    {
        let keys_in_keychain = match crate::keychain::keys_exist(KEYCHAIN_SERVICE) {
            Ok(present) => Some(present),
            Err(err) => {
                // On keychain access error, avoid forcing RecoveryRequired.
                // Treat as "unknown" so the app can default to a safer state.
                eprintln!(
                    "Failed to check keys in keychain for service {}: {}",
                    KEYCHAIN_SERVICE, err
                );
                None
            }
        };

        match keys_in_keychain {
            Some(true) => {
                if vault_exists {
                    // Normal case: keys in keychain and vault exists, app is locked
                    AuthState::Locked
                } else {
                    // Inconsistent state: keys exist in keychain but vault file is missing.
                    // Treat as a recovery scenario rather than first run to avoid reinitializing keys.
                    AuthState::RecoveryRequired
                }
            }
            Some(false) => {
                if vault_exists {
                    // Recovery case: vault exists but no keychain keys
                    AuthState::RecoveryRequired
                } else {
                    // First run: no vault and no keys
                    AuthState::FirstRun
                }
            }
            None => {
                // Keychain access failed (e.g., locked or permission issue).
                // Safer to treat as locked so UI can prompt for unlock/retry.
                AuthState::Locked
            }
        }
    }

    // Non-macOS: Always start in FirstRun state (keychain not available)
    #[cfg(not(target_os = "macos"))]
    {
        if vault_exists {
            AuthState::RecoveryRequired
        } else {
            AuthState::FirstRun
        }
    }
}
