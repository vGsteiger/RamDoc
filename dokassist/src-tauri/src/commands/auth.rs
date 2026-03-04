use crate::constants::{DB_KEY_ACCOUNT, FS_KEY_ACCOUNT, KEYCHAIN_SERVICE, RECOVERY_FILENAME};
use crate::error::AppError;
use crate::state::{AppState, AuthState};
use crate::{keychain, recovery};
use tauri::State;

/// Returns "first_run" | "locked" | "unlocked" | "recovery_required"
#[tauri::command]
pub async fn check_auth(state: State<'_, AppState>) -> Result<String, AppError> {
    let auth = state.auth.lock().unwrap();

    match *auth {
        AuthState::FirstRun => Ok("first_run".to_string()),
        AuthState::Locked => Ok("locked".to_string()),
        AuthState::Unlocked { .. } => Ok("unlocked".to_string()),
        AuthState::RecoveryRequired => Ok("recovery_required".to_string()),
    }
}

/// First run: generate keys, store in Keychain. Returns 24 mnemonic words.
#[tauri::command]
pub async fn initialize_app(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    {
        let auth = state.auth.lock().unwrap();
        if !matches!(*auth, AuthState::FirstRun) {
            return Err(AppError::Validation(
                "App is already initialized".to_string(),
            ));
        }
    }

    // Derive master keys from a freshly generated mnemonic (Argon2id KDF).
    // The vault marker is written to vault_path; keys are wrapped in Zeroizing
    // immediately so they are wiped on any early-return path.
    let vault_path = state.data_dir.join(RECOVERY_FILENAME);
    let (words, db_key_raw, fs_key_raw) = recovery::create_recovery(&vault_path)?;
    let db_key = zeroize::Zeroizing::new(db_key_raw);
    let fs_key = zeroize::Zeroizing::new(fs_key_raw);

    // Store keys in Keychain
    keychain::store_key(KEYCHAIN_SERVICE, DB_KEY_ACCOUNT, &*db_key)?;
    keychain::store_key(KEYCHAIN_SERVICE, FS_KEY_ACCOUNT, &*fs_key)?;

    // Initialize database *before* committing auth state (HIGH-5: TOCTOU fix)
    state.init_db(&db_key)?;

    // Only transition to Unlocked after DB init succeeds
    let mut auth = state.auth.lock().unwrap();
    *auth = AuthState::Unlocked { db_key, fs_key };

    Ok(words)
}

/// Unlock: triggers Touch ID, retrieves keys from Keychain.
#[tauri::command]
pub async fn unlock_app(state: State<'_, AppState>) -> Result<bool, AppError> {
    // --- Retrieve keys while holding the auth lock ---
    let (db_key, fs_key) = {
        let auth = state.auth.lock().unwrap();

        if !matches!(*auth, AuthState::Locked) {
            return Err(AppError::Validation("App is not locked".to_string()));
        }

        // Retrieve keys from Keychain (triggers Touch ID)
        let mut db_key_vec = keychain::retrieve_key(KEYCHAIN_SERVICE, DB_KEY_ACCOUNT)?;
        let mut fs_key_vec = keychain::retrieve_key(KEYCHAIN_SERVICE, FS_KEY_ACCOUNT)?;

        if db_key_vec.len() != 32 || fs_key_vec.len() != 32 {
            zeroize::Zeroize::zeroize(&mut db_key_vec);
            zeroize::Zeroize::zeroize(&mut fs_key_vec);
            return Err(AppError::Keychain("Invalid key size".to_string()));
        }

        let mut db_key = [0u8; 32];
        let mut fs_key = [0u8; 32];
        db_key.copy_from_slice(&db_key_vec);
        fs_key.copy_from_slice(&fs_key_vec);

        zeroize::Zeroize::zeroize(&mut db_key_vec);
        zeroize::Zeroize::zeroize(&mut fs_key_vec);

        (db_key, fs_key)
        // auth lock is released here
    };

    // Initialize database *before* committing auth state (HIGH-5: TOCTOU fix).
    // If init_db fails, auth state remains Locked — no inconsistent state.
    state.init_db(&db_key)?;

    // Silently upgrade existing keychain items to biometric protection.
    // This is idempotent: items already protected are replaced with fresh ones.
    // Errors are ignored — the app is already unlocked; migration is best-effort.
    let _ = keychain::store_key(KEYCHAIN_SERVICE, DB_KEY_ACCOUNT, &db_key);
    let _ = keychain::store_key(KEYCHAIN_SERVICE, FS_KEY_ACCOUNT, &fs_key);

    // Only transition to Unlocked after DB init succeeds
    let mut auth = state.auth.lock().unwrap();
    *auth = AuthState::Unlocked {
        db_key: zeroize::Zeroizing::new(db_key),
        fs_key: zeroize::Zeroizing::new(fs_key),
    };

    Ok(true)
}

/// Recover keys from 24-word mnemonic.
#[tauri::command]
pub async fn recover_app(state: State<'_, AppState>, words: Vec<String>) -> Result<bool, AppError> {
    // Verify we're in RecoveryRequired state (without holding lock during I/O)
    {
        let auth = state.auth.lock().unwrap();
        if !matches!(*auth, AuthState::RecoveryRequired) {
            return Err(AppError::Validation("Recovery is not required".to_string()));
        }
    }

    // CRIT-1: Check rate limit before attempting recovery
    recovery::check_recovery_rate_limit(&state.data_dir)?;

    // Recover keys from mnemonic
    let vault_path = state.data_dir.join(RECOVERY_FILENAME);
    let result = recovery::recover_from_mnemonic(&words, &vault_path);

    let (db_key, fs_key) = match result {
        Ok(keys) => {
            // Clear attempt counter on success
            recovery::clear_recovery_attempts(&state.data_dir);
            keys
        }
        Err(e) => {
            // Record failed attempt (increments counter and updates lockout)
            recovery::record_failed_attempt(&state.data_dir);
            return Err(e);
        }
    };

    // Store recovered keys in Keychain
    keychain::store_key(KEYCHAIN_SERVICE, DB_KEY_ACCOUNT, &db_key)?;
    keychain::store_key(KEYCHAIN_SERVICE, FS_KEY_ACCOUNT, &fs_key)?;

    // Initialize database *before* committing auth state (HIGH-5: TOCTOU fix)
    state.init_db(&db_key)?;

    // Only transition to Unlocked after DB init succeeds
    let mut auth = state.auth.lock().unwrap();
    *auth = AuthState::Unlocked {
        db_key: zeroize::Zeroizing::new(db_key),
        fs_key: zeroize::Zeroizing::new(fs_key),
    };

    Ok(true)
}

/// Factory reset: wipe all keychain keys, the entire data directory, and
/// return the app to `FirstRun` state.
///
/// ⚠ Irreversible — all patient data, the encrypted vault, and model files
/// stored in the data directory are permanently deleted.
#[tauri::command]
pub async fn reset_app(state: State<'_, AppState>) -> Result<(), AppError> {
    log::warn!("Factory reset requested — wiping all app data");

    // 1. Transition to FirstRun and release any in-memory keys / DB handles.
    {
        let mut auth = state.auth.lock().unwrap();
        *auth = AuthState::FirstRun;
    }
    state.clear_db()?;
    {
        let mut llm_lock = state.llm.lock().unwrap();
        *llm_lock = None;
    }

    // 2. Delete keychain entries (ignore "not found" errors).
    let _ = keychain::delete_key(KEYCHAIN_SERVICE, DB_KEY_ACCOUNT);
    let _ = keychain::delete_key(KEYCHAIN_SERVICE, FS_KEY_ACCOUNT);

    // 3. Wipe the entire data directory (database, vault, model files, …).
    if state.data_dir.exists() {
        std::fs::remove_dir_all(&state.data_dir)?;
    }

    // 4. Re-create an empty data directory ready for `initialize_app`.
    std::fs::create_dir_all(&state.data_dir)?;

    log::warn!("Factory reset complete — app is in FirstRun state");
    Ok(())
}

/// Lock: zero keys from memory.
#[tauri::command]
pub async fn lock_app(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut auth = state.auth.lock().unwrap();

    // Only lock if currently unlocked
    if matches!(*auth, AuthState::Unlocked { .. }) {
        *auth = AuthState::Locked;
        drop(auth);

        // Clear database pool
        state.clear_db()?;
    }

    Ok(())
}
