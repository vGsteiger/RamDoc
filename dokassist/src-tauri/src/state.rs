use std::sync::Mutex;

const KEYCHAIN_SERVICE: &str = "ch.dokassist.app";
const RECOVERY_FILENAME: &str = "recovery.vault";

/// Application state shared across all Tauri commands.
pub struct AppState {
    pub auth: Mutex<AuthState>,
    pub data_dir: std::path::PathBuf,
    // db: Option<DbPool>,       // added in PKG-2
    // llm: Option<LlmEngine>,   // added in PKG-4
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

        Self {
            auth: Mutex::new(initial_state),
            data_dir,
        }
    }
}

fn determine_initial_auth_state(data_dir: &std::path::Path) -> AuthState {
    let vault_path = data_dir.join(RECOVERY_FILENAME);
    let vault_exists = vault_path.exists();

    // Check if keys exist in keychain (macOS only)
    #[cfg(target_os = "macos")]
    {
        let keys_in_keychain = crate::keychain::keys_exist(KEYCHAIN_SERVICE).unwrap_or(false);

        if vault_exists && keys_in_keychain {
            // Normal case: keys in keychain, app is locked
            AuthState::Locked
        } else if vault_exists && !keys_in_keychain {
            // Recovery case: vault exists but no keychain keys
            AuthState::RecoveryRequired
        } else {
            // First run: no vault, no keys
            AuthState::FirstRun
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
