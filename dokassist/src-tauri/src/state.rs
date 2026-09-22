#[cfg(target_os = "macos")]
use crate::constants::KEYCHAIN_SERVICE;
use crate::constants::{DATABASE_FILENAME, RECOVERY_FILENAME};
use crate::database::DbPool;
use crate::llm::router::RouterBenchmarkDiagnostics;
use crate::llm::{embed::EmbedEngine, EngineLifecyclePhase, EngineLifecycleStatus, LlmEngine};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub(crate) fn llm_lock_poisoned() -> crate::error::AppError {
    crate::error::AppError::Llm("LLM state mutex poisoned".to_string())
}

/// Application state shared across all Tauri commands.
pub struct AppState {
    pub auth: Mutex<AuthState>,
    pub data_dir: std::path::PathBuf,
    pub db: Mutex<Option<DbPool>>,
    pub llm: Mutex<Option<Arc<LlmEngine>>>,
    /// Serializes model/context swaps so the old allocation is released before
    /// a replacement model begins loading.
    pub llm_swap: tokio::sync::Mutex<()>,
    /// Runtime-only lifecycle for the single resident writing engine. The
    /// durable desired model remains the registry default and is resolved by
    /// commands when needed.
    pub llm_lifecycle: Mutex<EngineLifecycleStatus>,
    pub llm_lifecycle_changed: tokio::sync::Notify,
    /// The optional, dedicated engine used only for agent tool decisions. It
    /// intentionally has separate ownership and lifecycle from the writing
    /// engine: final patient-facing prose must never silently move to it.
    pub router_llm: Mutex<Option<Arc<LlmEngine>>>,
    pub router_llm_swap: tokio::sync::Mutex<()>,
    pub router_lifecycle: Mutex<EngineLifecycleStatus>,
    pub router_lifecycle_changed: tokio::sync::Notify,
    pub router_benchmark: Mutex<RouterBenchmarkDiagnostics>,
    /// Embedding engine for semantic search.  Populated lazily by `process_file`.
    pub embed: Mutex<Option<Arc<Mutex<EmbedEngine>>>>,
    /// Unencrypted medication reference SQLite (public AIPS data).
    /// `None` until the user downloads the reference DB via settings.
    pub medication_ref: Mutex<Option<Connection>>,
}

pub enum AuthState {
    FirstRun,
    /// `initialize_app` has claimed the first run and is generating master keys.
    /// Nothing else may write the vault or the Keychain items until it finishes.
    Initializing,
    Locked,
    Unlocked {
        db_key: zeroize::Zeroizing<[u8; 32]>,
        fs_key: zeroize::Zeroizing<[u8; 32]>,
    },
    RecoveryRequired,
}

/// Whether `db_path` holds a real database rather than the zero-length file that
/// `Connection::open` leaves behind when initialization fails before the first
/// page is written. An empty file carries no data and must not block a retry.
pub(crate) fn database_is_initialized(db_path: &std::path::Path) -> bool {
    std::fs::metadata(db_path)
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false)
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
            llm_swap: tokio::sync::Mutex::new(()),
            llm_lifecycle: Mutex::new(EngineLifecycleStatus::default()),
            llm_lifecycle_changed: tokio::sync::Notify::new(),
            router_llm: Mutex::new(None),
            router_llm_swap: tokio::sync::Mutex::new(()),
            router_lifecycle: Mutex::new(EngineLifecycleStatus::default()),
            router_lifecycle_changed: tokio::sync::Notify::new(),
            router_benchmark: Mutex::new(RouterBenchmarkDiagnostics::default()),
            embed: Mutex::new(None),
            medication_ref: Mutex::new(medication_ref),
        }
    }

    /// Initialize the database and strictly verify its external audit checkpoint.
    pub fn init_db(
        &self,
        db_key: &[u8; 32],
        fs_key: &[u8; 32],
    ) -> Result<(), crate::error::AppError> {
        self.init_db_with_mode(db_key, fs_key, crate::database::CheckpointMode::Strict)
    }

    /// Initialize after an explicitly authorized recovery/restore and establish a
    /// new checkpoint after the complete HMAC chain has verified.
    pub fn init_db_reanchor(
        &self,
        db_key: &[u8; 32],
        fs_key: &[u8; 32],
    ) -> Result<(), crate::error::AppError> {
        self.init_db_with_mode(db_key, fs_key, crate::database::CheckpointMode::Reanchor)
    }

    fn init_db_with_mode(
        &self,
        db_key: &[u8; 32],
        fs_key: &[u8; 32],
        mode: crate::database::CheckpointMode,
    ) -> Result<(), crate::error::AppError> {
        let db_path = self.data_dir.join(DATABASE_FILENAME);
        #[cfg(target_os = "macos")]
        let pool = crate::database::init_db_with_audit_checkpoint(&db_path, db_key, fs_key, mode)?;
        #[cfg(not(target_os = "macos"))]
        let pool = {
            let _ = (fs_key, mode);
            crate::database::init_db(&db_path, db_key)?
        };

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
        // This is the same application-owned ML teardown boundary used by
        // lock/reset/close. Keep a dedicated router from retaining a prompt
        // cache after the writing engine has been released.
        if let Ok(mut g) = self.router_llm.lock() {
            *g = None;
        }
        if let Ok(mut lifecycle) = self.llm_lifecycle.lock() {
            *lifecycle = EngineLifecycleStatus::default();
        }
        if let Ok(mut lifecycle) = self.router_lifecycle.lock() {
            *lifecycle = EngineLifecycleStatus::default();
        }
        self.llm_lifecycle_changed.notify_waiters();
        self.router_lifecycle_changed.notify_waiters();
    }

    pub fn llm_lifecycle(&self) -> EngineLifecycleStatus {
        self.llm_lifecycle
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| EngineLifecycleStatus {
                phase: EngineLifecyclePhase::Error,
                requested_filename: None,
                active_filename: None,
                error: Some("LLM lifecycle mutex poisoned".to_string()),
            })
    }

    /// Start a load. A duplicate request for the same file joins the active
    /// operation; a different file must be retried after the current swap.
    pub fn begin_llm_load(
        &self,
        filename: &str,
    ) -> Result<LlmLoadDisposition, crate::error::AppError> {
        let mut lifecycle = self
            .llm_lifecycle
            .lock()
            .map_err(|_| crate::error::AppError::Llm("LLM lifecycle mutex poisoned".to_string()))?;
        match lifecycle.phase {
            EngineLifecyclePhase::Loading
                if lifecycle.requested_filename.as_deref() == Some(filename) =>
            {
                Ok(LlmLoadDisposition::Join)
            }
            EngineLifecyclePhase::Loading | EngineLifecyclePhase::Unloading => {
                Err(crate::error::AppError::Llm(
                    "A model lifecycle operation is already in progress".to_string(),
                ))
            }
            EngineLifecyclePhase::Ready
                if lifecycle.active_filename.as_deref() == Some(filename) =>
            {
                // Lock/reset/window-close can clear the engine without taking
                // the asynchronous lifecycle coordinator. Confirm residency
                // before treating a stale Ready snapshot as a no-op.
                let is_resident = self
                    .llm
                    .lock()
                    .map(|engine| engine.is_some())
                    .unwrap_or(false);
                if is_resident {
                    Ok(LlmLoadDisposition::AlreadyReady)
                } else {
                    lifecycle.phase = EngineLifecyclePhase::Loading;
                    lifecycle.requested_filename = Some(filename.to_string());
                    lifecycle.active_filename = None;
                    lifecycle.error = None;
                    Ok(LlmLoadDisposition::Start)
                }
            }
            _ => {
                lifecycle.phase = EngineLifecyclePhase::Loading;
                lifecycle.requested_filename = Some(filename.to_string());
                lifecycle.active_filename = None;
                lifecycle.error = None;
                Ok(LlmLoadDisposition::Start)
            }
        }
    }

    pub async fn wait_for_llm_load(&self, filename: &str) -> Result<(), crate::error::AppError> {
        loop {
            let changed = self.llm_lifecycle_changed.notified();
            let lifecycle = self.llm_lifecycle();
            match lifecycle.phase {
                EngineLifecyclePhase::Loading
                    if lifecycle.requested_filename.as_deref() == Some(filename) =>
                {
                    changed.await;
                }
                EngineLifecyclePhase::Ready
                    if lifecycle.active_filename.as_deref() == Some(filename) =>
                {
                    return Ok(())
                }
                EngineLifecyclePhase::Error
                    if lifecycle.requested_filename.as_deref() == Some(filename) =>
                {
                    return Err(crate::error::AppError::Llm(
                        lifecycle
                            .error
                            .unwrap_or_else(|| "Model load failed".to_string()),
                    ));
                }
                _ => {
                    return Err(crate::error::AppError::Llm(
                        "Requested model did not become ready".to_string(),
                    ))
                }
            }
        }
    }

    pub fn mark_llm_ready(&self, filename: String) {
        if let Ok(mut lifecycle) = self.llm_lifecycle.lock() {
            *lifecycle = EngineLifecycleStatus {
                phase: EngineLifecyclePhase::Ready,
                requested_filename: None,
                active_filename: Some(filename),
                error: None,
            };
        }
        self.llm_lifecycle_changed.notify_waiters();
    }

    pub fn mark_llm_load_failed(&self, filename: String, error: String) {
        if let Ok(mut lifecycle) = self.llm_lifecycle.lock() {
            *lifecycle = EngineLifecycleStatus {
                phase: EngineLifecyclePhase::Error,
                requested_filename: Some(filename),
                active_filename: None,
                error: Some(error),
            };
        }
        self.llm_lifecycle_changed.notify_waiters();
    }

    pub fn begin_llm_unload(&self) -> Result<(), crate::error::AppError> {
        let mut lifecycle = self
            .llm_lifecycle
            .lock()
            .map_err(|_| crate::error::AppError::Llm("LLM lifecycle mutex poisoned".to_string()))?;
        if matches!(
            lifecycle.phase,
            EngineLifecyclePhase::Loading | EngineLifecyclePhase::Unloading
        ) {
            return Err(crate::error::AppError::Llm(
                "Cannot unload while another model lifecycle operation is in progress".to_string(),
            ));
        }
        lifecycle.phase = EngineLifecyclePhase::Unloading;
        lifecycle.requested_filename = None;
        lifecycle.error = None;
        Ok(())
    }

    pub fn mark_llm_unloaded(&self) {
        if let Ok(mut lifecycle) = self.llm_lifecycle.lock() {
            *lifecycle = EngineLifecycleStatus::default();
        }
        self.llm_lifecycle_changed.notify_waiters();
    }

    pub fn router_lifecycle(&self) -> EngineLifecycleStatus {
        self.router_lifecycle
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| EngineLifecycleStatus {
                phase: EngineLifecyclePhase::Error,
                requested_filename: None,
                active_filename: None,
                error: Some("Router lifecycle mutex poisoned".to_string()),
            })
    }

    /// The router has the same join semantics as the writing lifecycle, but
    /// is deliberately coordinated through its own slot so loading it never
    /// replaces the writing engine.
    pub fn begin_router_load(
        &self,
        filename: &str,
    ) -> Result<LlmLoadDisposition, crate::error::AppError> {
        let mut lifecycle = self.router_lifecycle.lock().map_err(|_| {
            crate::error::AppError::Llm("Router lifecycle mutex poisoned".to_string())
        })?;
        match lifecycle.phase {
            EngineLifecyclePhase::Loading
                if lifecycle.requested_filename.as_deref() == Some(filename) =>
            {
                Ok(LlmLoadDisposition::Join)
            }
            EngineLifecyclePhase::Loading | EngineLifecyclePhase::Unloading => {
                Err(crate::error::AppError::Llm(
                    "A router lifecycle operation is already in progress".to_string(),
                ))
            }
            EngineLifecyclePhase::Ready
                if lifecycle.active_filename.as_deref() == Some(filename) =>
            {
                let is_resident = self
                    .router_llm
                    .lock()
                    .map(|engine| engine.is_some())
                    .unwrap_or(false);
                if is_resident {
                    Ok(LlmLoadDisposition::AlreadyReady)
                } else {
                    lifecycle.phase = EngineLifecyclePhase::Loading;
                    lifecycle.requested_filename = Some(filename.to_string());
                    lifecycle.active_filename = None;
                    lifecycle.error = None;
                    Ok(LlmLoadDisposition::Start)
                }
            }
            _ => {
                lifecycle.phase = EngineLifecyclePhase::Loading;
                lifecycle.requested_filename = Some(filename.to_string());
                lifecycle.active_filename = None;
                lifecycle.error = None;
                Ok(LlmLoadDisposition::Start)
            }
        }
    }

    pub async fn wait_for_router_load(&self, filename: &str) -> Result<(), crate::error::AppError> {
        loop {
            let changed = self.router_lifecycle_changed.notified();
            let lifecycle = self.router_lifecycle();
            match lifecycle.phase {
                EngineLifecyclePhase::Loading
                    if lifecycle.requested_filename.as_deref() == Some(filename) =>
                {
                    changed.await;
                }
                EngineLifecyclePhase::Ready
                    if lifecycle.active_filename.as_deref() == Some(filename) =>
                {
                    return Ok(())
                }
                EngineLifecyclePhase::Error
                    if lifecycle.requested_filename.as_deref() == Some(filename) =>
                {
                    return Err(crate::error::AppError::Llm(
                        lifecycle
                            .error
                            .unwrap_or_else(|| "Router load failed".to_string()),
                    ));
                }
                _ => {
                    return Err(crate::error::AppError::Llm(
                        "Requested router did not become ready".to_string(),
                    ))
                }
            }
        }
    }

    pub fn mark_router_ready(&self, filename: String) {
        if let Ok(mut lifecycle) = self.router_lifecycle.lock() {
            *lifecycle = EngineLifecycleStatus {
                phase: EngineLifecyclePhase::Ready,
                requested_filename: None,
                active_filename: Some(filename),
                error: None,
            };
        }
        self.router_lifecycle_changed.notify_waiters();
    }

    pub fn mark_router_load_failed(&self, filename: String, error: String) {
        if let Ok(mut lifecycle) = self.router_lifecycle.lock() {
            *lifecycle = EngineLifecycleStatus {
                phase: EngineLifecyclePhase::Error,
                requested_filename: Some(filename),
                active_filename: None,
                error: Some(error),
            };
        }
        self.router_lifecycle_changed.notify_waiters();
    }

    pub fn begin_router_unload(&self) -> Result<(), crate::error::AppError> {
        let mut lifecycle = self.router_lifecycle.lock().map_err(|_| {
            crate::error::AppError::Llm("Router lifecycle mutex poisoned".to_string())
        })?;
        if matches!(
            lifecycle.phase,
            EngineLifecyclePhase::Loading | EngineLifecyclePhase::Unloading
        ) {
            return Err(crate::error::AppError::Llm(
                "Cannot unload while another router lifecycle operation is in progress".to_string(),
            ));
        }
        lifecycle.phase = EngineLifecyclePhase::Unloading;
        lifecycle.requested_filename = None;
        lifecycle.error = None;
        Ok(())
    }

    pub fn mark_router_unloaded(&self) {
        if let Ok(mut lifecycle) = self.router_lifecycle.lock() {
            *lifecycle = EngineLifecycleStatus::default();
        }
        self.router_lifecycle_changed.notify_waiters();
    }

    pub fn router_benchmark(&self) -> RouterBenchmarkDiagnostics {
        self.router_benchmark
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default()
    }

    pub fn record_router_probe(&self, dedicated: bool, duration: std::time::Duration) {
        if let Ok(mut benchmark) = self.router_benchmark.lock() {
            benchmark.probe_count = benchmark.probe_count.saturating_add(1);
            if dedicated {
                benchmark.dedicated_probe_count = benchmark.dedicated_probe_count.saturating_add(1);
            } else {
                benchmark.writing_fallback_count =
                    benchmark.writing_fallback_count.saturating_add(1);
            }
            benchmark.last_probe_duration_ms =
                Some(duration.as_millis().min(u128::from(u64::MAX)) as u64);
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

pub enum LlmLoadDisposition {
    Start,
    Join,
    AlreadyReady,
}

fn determine_initial_auth_state(data_dir: &std::path::Path) -> AuthState {
    let vault_path = data_dir.join(RECOVERY_FILENAME);
    let vault_exists = vault_path.exists();
    #[cfg(target_os = "macos")]
    let database_exists = database_is_initialized(&data_dir.join(DATABASE_FILENAME));

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
            Some(false) => auth_state_without_master_keys(vault_exists, database_exists),
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

/// Decide whether a vault without master keys represents interrupted setup or
/// existing data that must be recovered. A database file is the commit point:
/// the recovery phrase is only shown after both keys and the database exist.
fn auth_state_without_master_keys(vault_exists: bool, database_exists: bool) -> AuthState {
    match (vault_exists, database_exists) {
        // Existing encrypted data without its master keys must never be
        // overwritten automatically. The recovery phrase is needed to
        // recreate the Keychain items.
        (true, true) => AuthState::RecoveryRequired,
        // `initialize_app` creates the vault before writing Keychain items or
        // the database. If that first setup was interrupted (including the
        // -34018 regression), the phrase was never shown and there is no
        // patient data to preserve. Start setup over instead.
        _ => AuthState::FirstRun,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn missing_keys_require_recovery_only_when_patient_data_exists() {
        assert!(matches!(
            auth_state_without_master_keys(true, true),
            AuthState::RecoveryRequired
        ));
        assert!(matches!(
            auth_state_without_master_keys(true, false),
            AuthState::FirstRun
        ));
        assert!(matches!(
            auth_state_without_master_keys(false, false),
            AuthState::FirstRun
        ));
    }

    /// `Connection::open` creates the file before it can fail on the key, so a
    /// zero-length `dokassist.db` means setup never got as far as writing a page.
    #[test]
    fn empty_database_file_does_not_count_as_initialized() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join(DATABASE_FILENAME);

        assert!(!database_is_initialized(&db_path));

        std::fs::write(&db_path, []).unwrap();
        assert!(!database_is_initialized(&db_path));

        std::fs::write(&db_path, b"SQLite format 3\0").unwrap();
        assert!(database_is_initialized(&db_path));
    }

    #[test]
    fn poisoned_llm_lock_maps_to_llm_error() {
        let llm = Mutex::new(None::<Arc<LlmEngine>>);

        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = llm.lock().unwrap();
            panic!("poison LLM mutex");
        }));

        let error = match llm.lock().map_err(|_| llm_lock_poisoned()) {
            Ok(_) => panic!("poisoned LLM mutex unexpectedly locked"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            crate::error::AppError::Llm(message) if message == "LLM state mutex poisoned"
        ));
    }
}
