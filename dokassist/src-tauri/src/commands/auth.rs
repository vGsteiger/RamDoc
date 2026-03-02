use crate::constants::{DB_KEY_ACCOUNT, FS_KEY_ACCOUNT, KEYCHAIN_SERVICE, RECOVERY_FILENAME};
use crate::error::AppError;
use crate::state::{AppState, AuthState};
use crate::{crypto, keychain, recovery};
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
    let mut auth = state.auth.lock().unwrap();

    // Verify we're in FirstRun state
    if !matches!(*auth, AuthState::FirstRun) {
        return Err(AppError::Validation(
            "App is already initialized".to_string(),
        ));
    }

    // Generate master keys
    let db_key = crypto::generate_key();
    let fs_key = crypto::generate_key();

    // Store keys in Keychain
    keychain::store_key(KEYCHAIN_SERVICE, DB_KEY_ACCOUNT, &db_key)?;
    keychain::store_key(KEYCHAIN_SERVICE, FS_KEY_ACCOUNT, &fs_key)?;

    // Create recovery vault
    let vault_path = state.data_dir.join(RECOVERY_FILENAME);
    let words = recovery::create_recovery(&db_key, &fs_key, &vault_path)?;

    // Transition to Unlocked state
    *auth = AuthState::Unlocked {
        db_key: zeroize::Zeroizing::new(db_key),
        fs_key: zeroize::Zeroizing::new(fs_key),
    };

    // Initialize database with db_key
    drop(auth); // Release lock before calling init_db
    state.init_db(&db_key)?;

    Ok(words)
}

/// Unlock: triggers Touch ID, retrieves keys from Keychain.
#[tauri::command]
pub async fn unlock_app(state: State<'_, AppState>) -> Result<bool, AppError> {
    let mut auth = state.auth.lock().unwrap();

    // Verify we're in Locked state
    if !matches!(*auth, AuthState::Locked) {
        return Err(AppError::Validation("App is not locked".to_string()));
    }

    // Retrieve keys from Keychain (triggers Touch ID)
    let mut db_key_vec = keychain::retrieve_key(KEYCHAIN_SERVICE, DB_KEY_ACCOUNT)?;
    let mut fs_key_vec = keychain::retrieve_key(KEYCHAIN_SERVICE, FS_KEY_ACCOUNT)?;

    // Convert to fixed-size arrays
    if db_key_vec.len() != 32 || fs_key_vec.len() != 32 {
        // Zeroize before returning error
        zeroize::Zeroize::zeroize(&mut db_key_vec);
        zeroize::Zeroize::zeroize(&mut fs_key_vec);
        return Err(AppError::Keychain("Invalid key size".to_string()));
    }

    let mut db_key = [0u8; 32];
    let mut fs_key = [0u8; 32];
    db_key.copy_from_slice(&db_key_vec);
    fs_key.copy_from_slice(&fs_key_vec);

    // Zeroize original key vectors now that we've copied their contents
    zeroize::Zeroize::zeroize(&mut db_key_vec);
    zeroize::Zeroize::zeroize(&mut fs_key_vec);

    // Transition to Unlocked state
    *auth = AuthState::Unlocked {
        db_key: zeroize::Zeroizing::new(db_key),
        fs_key: zeroize::Zeroizing::new(fs_key),
    };

    // Initialize database with db_key
    drop(auth); // Release lock before calling init_db
    state.init_db(&db_key)?;

    Ok(true)
}

/// Recover keys from 24-word mnemonic.
#[tauri::command]
pub async fn recover_app(state: State<'_, AppState>, words: Vec<String>) -> Result<bool, AppError> {
    let mut auth = state.auth.lock().unwrap();

    // Verify we're in RecoveryRequired state
    if !matches!(*auth, AuthState::RecoveryRequired) {
        return Err(AppError::Validation("Recovery is not required".to_string()));
    }

    // Recover keys from mnemonic
    let vault_path = state.data_dir.join(RECOVERY_FILENAME);
    let (db_key, fs_key) = recovery::recover_from_mnemonic(&words, &vault_path)?;

    // Store recovered keys in Keychain
    keychain::store_key(KEYCHAIN_SERVICE, DB_KEY_ACCOUNT, &db_key)?;
    keychain::store_key(KEYCHAIN_SERVICE, FS_KEY_ACCOUNT, &fs_key)?;

    // Transition to Unlocked state
    *auth = AuthState::Unlocked {
        db_key: zeroize::Zeroizing::new(db_key),
        fs_key: zeroize::Zeroizing::new(fs_key),
    };

    // Initialize database with db_key
    drop(auth); // Release lock before calling init_db
    state.init_db(&db_key)?;

    Ok(true)
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
