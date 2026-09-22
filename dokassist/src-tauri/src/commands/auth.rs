use crate::constants::{
    AUDIT_CHECKPOINT_A_ACCOUNT, AUDIT_CHECKPOINT_B_ACCOUNT, DATABASE_FILENAME, DB_KEY_ACCOUNT,
    FS_KEY_ACCOUNT, KEYCHAIN_SERVICE, RECOVERY_FILENAME,
};
use crate::error::AppError;
use crate::state::{AppState, AuthState};
use crate::{keychain, recovery};
use std::path::Path;
use tauri::State;

fn lock_poisoned() -> AppError {
    AppError::Keychain("Auth state mutex poisoned".to_string())
}

/// Returns "first_run" | "initializing" | "locked" | "unlocked" | "recovery_required"
#[tauri::command]
pub async fn check_auth(state: State<'_, AppState>) -> Result<String, AppError> {
    let auth = state.auth.lock().map_err(|_| lock_poisoned())?;

    match *auth {
        AuthState::FirstRun => Ok("first_run".to_string()),
        AuthState::Initializing => Ok("initializing".to_string()),
        AuthState::Locked => Ok("locked".to_string()),
        AuthState::Unlocked { .. } => Ok("unlocked".to_string()),
        AuthState::RecoveryRequired => Ok("recovery_required".to_string()),
    }
}

type MasterKeys = (
    Vec<String>,
    zeroize::Zeroizing<[u8; 32]>,
    zeroize::Zeroizing<[u8; 32]>,
);

/// Generate the master keys, the vault marker, the Keychain items and the
/// database. Every step here overwrites or creates on-disk state, so callers
/// must hold the `Initializing` claim for the whole call.
fn create_master_keys(
    state: &AppState,
    vault_path: &Path,
    db_path: &Path,
) -> Result<MasterKeys, AppError> {
    // An existing database is encrypted under keys this function is about to
    // replace. Regenerating them would strand the data behind a key that exists
    // nowhere — neither in the Keychain nor behind the new recovery phrase.
    if crate::state::database_is_initialized(db_path) {
        return Err(AppError::AlreadyInitialized);
    }

    // `create_recovery` refuses to overwrite. A marker with no database behind it
    // is an interrupted setup: the phrase was never confirmed and no data depends
    // on those keys, so clearing it to start over is safe.
    if vault_path.exists() {
        std::fs::remove_file(vault_path)?;
    }

    // Derive master keys from a freshly generated mnemonic (Argon2id KDF).
    // The vault marker is written to vault_path; keys are wrapped in Zeroizing
    // immediately so they are wiped on any early-return path.
    let (words, db_key_raw, fs_key_raw) = recovery::create_recovery(vault_path)?;
    let db_key = zeroize::Zeroizing::new(db_key_raw);
    let fs_key = zeroize::Zeroizing::new(fs_key_raw);

    // Store keys in Keychain
    keychain::store_key(KEYCHAIN_SERVICE, DB_KEY_ACCOUNT, &*db_key)?;
    keychain::store_key(KEYCHAIN_SERVICE, FS_KEY_ACCOUNT, &*fs_key)?;

    // Initialize database *before* committing auth state (HIGH-5: TOCTOU fix)
    state.init_db(&db_key, &fs_key)?;

    Ok((words, db_key, fs_key))
}

/// First run: generate keys, store in Keychain. Returns 24 mnemonic words.
///
/// The `FirstRun` → `Initializing` transition happens under the auth lock so that
/// only one call can ever be past the guard. Two calls racing here (a page reload
/// during the ~1 s of Argon2id and migrations is enough) would each write their
/// own vault and Keychain items while the database kept the first caller's key,
/// leaving a vault that neither Touch ID nor the recovery phrase can open.
#[tauri::command]
pub async fn initialize_app(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    {
        let mut auth = state.auth.lock().map_err(|_| lock_poisoned())?;
        match *auth {
            AuthState::FirstRun => {}
            AuthState::Initializing => return Err(AppError::SetupInProgress),
            _ => return Err(AppError::AlreadyInitialized),
        }
        *auth = AuthState::Initializing;
    }

    let vault_path = state.data_dir.join(RECOVERY_FILENAME);
    let db_path = state.data_dir.join(DATABASE_FILENAME);

    match create_master_keys(&state, &vault_path, &db_path) {
        Ok((words, db_key, fs_key)) => {
            // Only transition to Unlocked after DB init succeeds
            let mut auth = state.auth.lock().map_err(|_| lock_poisoned())?;
            *auth = AuthState::Unlocked { db_key, fs_key };
            Ok(words)
        }
        Err(err) => {
            // Release the claim. A database on disk means the keys were already
            // committed to the Keychain, so the session is Locked rather than
            // fresh — retrying setup from there must not regenerate them.
            let mut auth = state.auth.lock().map_err(|_| lock_poisoned())?;
            *auth = if crate::state::database_is_initialized(&db_path) {
                AuthState::Locked
            } else {
                AuthState::FirstRun
            };
            Err(err)
        }
    }
}

/// Unlock: show Touch ID sheet, retrieve master keys from Keychain.
///
/// Biometric authentication is performed via `LocalAuthentication` (LAContext)
/// before accessing the Keychain.  Returns `AppError::BiometricCancelled`
/// when the user dismisses the Touch ID / login-password sheet.
#[tauri::command]
pub async fn unlock_app(state: State<'_, AppState>) -> Result<bool, AppError> {
    // Check auth state before showing Touch ID (avoids prompting if already unlocked).
    {
        let auth = state.auth.lock().map_err(|_| lock_poisoned())?;
        if !matches!(*auth, AuthState::Locked) {
            return Err(AppError::Validation("App is not locked".to_string()));
        }
    }

    // Show Touch ID / device-password sheet.  This blocks the Tokio worker thread
    // until the user responds; the auth lock is NOT held during this call.
    crate::touch_id::authenticate("Unlock DokAssist to access patient data")?;

    // --- Retrieve keys while holding the auth lock ---
    let (db_key, fs_key) = {
        let mut auth = state.auth.lock().map_err(|_| lock_poisoned())?;

        // Re-check state in case the app was reset while Touch ID was showing.
        if !matches!(*auth, AuthState::Locked) {
            return Err(AppError::Validation("App is not locked".to_string()));
        }

        // Keychain enforces biometric or device-passcode authentication for protected items.
        // A missing item cannot be fixed by retrying Touch ID (for example, the
        // item may have been invalidated after a biometric-set change), so make
        // the recovery flow the session's next state.
        let mut db_key_vec = match keychain::retrieve_key(KEYCHAIN_SERVICE, DB_KEY_ACCOUNT) {
            Ok(key) => key,
            Err(AppError::KeychainItemMissing) => {
                *auth = AuthState::RecoveryRequired;
                return Err(AppError::KeychainItemMissing);
            }
            Err(err) => return Err(err),
        };
        let mut fs_key_vec = match keychain::retrieve_key(KEYCHAIN_SERVICE, FS_KEY_ACCOUNT) {
            Ok(key) => key,
            Err(AppError::KeychainItemMissing) => {
                zeroize::Zeroize::zeroize(&mut db_key_vec);
                *auth = AuthState::RecoveryRequired;
                return Err(AppError::KeychainItemMissing);
            }
            Err(err) => return Err(err),
        };

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
        // auth lock released here
    };

    // Initialize database *before* committing auth state (HIGH-5: TOCTOU fix).
    // If init_db fails, auth state remains Locked — no inconsistent state.
    state.init_db(&db_key, &fs_key)?;

    // Only transition to Unlocked after DB init succeeds
    let mut auth = state.auth.lock().map_err(|_| lock_poisoned())?;
    *auth = AuthState::Unlocked {
        db_key: zeroize::Zeroizing::new(db_key),
        fs_key: zeroize::Zeroizing::new(fs_key),
    };

    Ok(true)
}

/// Recover keys from 24-word mnemonic.
///
/// Accepted from `Locked` as well as `RecoveryRequired`: the phrase is the master
/// credential, and the unlock screen offers it as the way out of a vault that
/// Touch ID cannot open. Requiring `RecoveryRequired` — which is only reached when
/// the Keychain items have gone missing — made the phrase unusable in exactly the
/// case users reach for it.
#[tauri::command]
pub async fn recover_app(state: State<'_, AppState>, words: Vec<String>) -> Result<bool, AppError> {
    // Verify the vault is sealed (without holding the lock during I/O)
    {
        let auth = state.auth.lock().map_err(|_| lock_poisoned())?;
        match *auth {
            AuthState::RecoveryRequired | AuthState::Locked => {}
            AuthState::Initializing => return Err(AppError::SetupInProgress),
            AuthState::FirstRun => {
                return Err(AppError::Validation(
                    "There is no vault to recover yet".to_string(),
                ))
            }
            AuthState::Unlocked { .. } => {
                return Err(AppError::Validation(
                    "The vault is already unlocked".to_string(),
                ))
            }
        }
    }

    // Recover keys from mnemonic
    let vault_path = state.data_dir.join(RECOVERY_FILENAME);
    let (db_key, fs_key) = recovery::recover_from_mnemonic(&words, &vault_path)?;

    // Store recovered keys in Keychain
    keychain::store_key(KEYCHAIN_SERVICE, DB_KEY_ACCOUNT, &db_key)?;
    keychain::store_key(KEYCHAIN_SERVICE, FS_KEY_ACCOUNT, &fs_key)?;

    // Initialize database *before* committing auth state (HIGH-5: TOCTOU fix)
    state.init_db_reanchor(&db_key, &fs_key)?;

    // Only transition to Unlocked after DB init succeeds
    let mut auth = state.auth.lock().map_err(|_| lock_poisoned())?;
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
/// Refuse to wipe while `initialize_app` still holds the `Initializing` claim.
/// That call has already released the mutex and is writing Keychain items and
/// the database; deleting those files underneath it can leave a phrase for a
/// vault that reset just destroyed, or keys written after the wipe.
fn begin_factory_reset(state: &AppState) -> Result<(), AppError> {
    let mut auth = state.auth.lock().map_err(|_| lock_poisoned())?;
    if matches!(*auth, AuthState::Initializing) {
        return Err(AppError::SetupInProgress);
    }
    *auth = AuthState::FirstRun;
    Ok(())
}

#[tauri::command]
pub async fn reset_app(state: State<'_, AppState>) -> Result<(), AppError> {
    log::warn!("Factory reset requested — wiping all app data");

    begin_factory_reset(&state)?;
    state.clear_db()?;
    state.clear_llm();
    state.clear_embed();

    // 2. Delete keychain entries (ignore "not found" errors).
    let _ = keychain::delete_key(KEYCHAIN_SERVICE, DB_KEY_ACCOUNT);
    let _ = keychain::delete_key(KEYCHAIN_SERVICE, FS_KEY_ACCOUNT);
    let _ = keychain::delete_key(KEYCHAIN_SERVICE, AUDIT_CHECKPOINT_A_ACCOUNT);
    let _ = keychain::delete_key(KEYCHAIN_SERVICE, AUDIT_CHECKPOINT_B_ACCOUNT);

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
    let mut auth = state.auth.lock().map_err(|_| lock_poisoned())?;

    // Only lock if currently unlocked
    if matches!(*auth, AuthState::Unlocked { .. }) {
        *auth = AuthState::Locked;
        drop(auth);

        // Clear database pool and ML resources
        state.clear_db()?;
        state.clear_llm();
        state.clear_embed();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn state_with(auth: AuthState) -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState {
            auth: Mutex::new(auth),
            data_dir: dir.path().to_path_buf(),
            db: Mutex::new(None),
            llm: Mutex::new(None),
            llm_swap: tokio::sync::Mutex::new(()),
            llm_lifecycle: Mutex::new(crate::llm::EngineLifecycleStatus::default()),
            llm_lifecycle_changed: tokio::sync::Notify::new(),
            router_llm: Mutex::new(None),
            router_llm_swap: tokio::sync::Mutex::new(()),
            router_lifecycle: Mutex::new(crate::llm::EngineLifecycleStatus::default()),
            router_lifecycle_changed: tokio::sync::Notify::new(),
            router_benchmark: Mutex::new(crate::llm::router::RouterBenchmarkDiagnostics::default()),
            runtime_generation: std::sync::atomic::AtomicU64::new(0),
            embed: Mutex::new(None),
            medication_ref: Mutex::new(None),
        };
        (dir, state)
    }

    #[test]
    fn factory_reset_refuses_to_run_while_setup_is_in_progress() {
        let (_dir, state) = state_with(AuthState::Initializing);
        let error = begin_factory_reset(&state).unwrap_err();
        assert!(matches!(error, AppError::SetupInProgress));
        assert!(matches!(
            *state.auth.lock().unwrap(),
            AuthState::Initializing
        ));
    }

    #[test]
    fn factory_reset_claims_first_run_from_locked() {
        let (_dir, state) = state_with(AuthState::Locked);
        begin_factory_reset(&state).unwrap();
        assert!(matches!(*state.auth.lock().unwrap(), AuthState::FirstRun));
    }
}
