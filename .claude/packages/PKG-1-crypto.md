## PKG-1 — Crypto Core + Keychain + Touch ID

**Goal**: All cryptographic operations and key lifecycle management. This is the security foundation — everything else depends on it.

**Files**:

```
src-tauri/src/
├── crypto.rs       # AES-256-GCM encrypt/decrypt, key generation
├── keychain.rs     # macOS Keychain CRUD with Touch ID gating
├── recovery.rs     # BIP-39 mnemonic generation, recovery.vault encrypt/decrypt
└── auth.rs         # App-level auth state machine
```

**Public interface**:

```rust
// === crypto.rs ===

/// Generate a cryptographically random 256-bit key
pub fn generate_key() -> [u8; 32];

/// AES-256-GCM encrypt. Returns: [12-byte nonce || ciphertext || 16-byte tag]
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, AppError>;

/// AES-256-GCM decrypt. Input format: [12-byte nonce || ciphertext || 16-byte tag]
pub fn decrypt(key: &[u8; 32], ciphertext: &[u8]) -> Result<Vec<u8>, AppError>;


// === keychain.rs ===

/// Store a key in macOS Keychain with Touch ID + device passcode protection.
/// Uses kSecAttrAccessibleWhenUnlockedThisDeviceOnly + BiometryCurrentSet.
pub fn store_key(service: &str, account: &str, key: &[u8]) -> Result<(), AppError>;

/// Retrieve a key from Keychain. Triggers Touch ID prompt.
pub fn retrieve_key(service: &str, account: &str) -> Result<Vec<u8>, AppError>;

/// Delete a key from Keychain.
pub fn delete_key(service: &str, account: &str) -> Result<(), AppError>;

/// Check if keys exist in Keychain (does NOT trigger biometric).
pub fn keys_exist(service: &str) -> Result<bool, AppError>;


// === recovery.rs ===

/// Generate a BIP-39 24-word mnemonic from the master keys.
/// Returns the mnemonic words AND writes recovery.vault to the given path.
pub fn create_recovery(
    db_key: &[u8; 32],
    fs_key: &[u8; 32],
    vault_path: &Path,
) -> Result<Vec<String>, AppError>;

/// Recover master keys from a mnemonic + recovery.vault file.
/// Returns (db_key, fs_key).
pub fn recover_from_mnemonic(
    words: &[String],
    vault_path: &Path,
) -> Result<([u8; 32], [u8; 32]), AppError>;


// === auth.rs ===

/// Application authentication state.
pub enum AuthState {
    /// First launch — no keys exist yet
    FirstRun,
    /// Keys exist, waiting for Touch ID
    Locked,
    /// Touch ID passed, keys in memory
    Unlocked { db_key: [u8; 32], fs_key: [u8; 32] },
    /// Existing vault found but no Keychain keys — recovery needed
    RecoveryRequired,
}

/// Determine current auth state (called on app launch).
pub fn check_auth_state(data_dir: &Path) -> Result<AuthState, AppError>;

/// First run: generate keys, store in Keychain, create recovery vault.
/// Returns the 24 mnemonic words to display to the user.
pub fn initialize(data_dir: &Path) -> Result<Vec<String>, AppError>;

/// Unlock: retrieve keys from Keychain (triggers Touch ID).
pub fn unlock() -> Result<AuthState, AppError>;

/// Recover: restore keys from mnemonic, store in new Keychain.
pub fn recover(words: &[String], data_dir: &Path) -> Result<AuthState, AppError>;

/// Lock: zero keys from memory.
pub fn lock(state: &mut AuthState);
```

**Tauri commands** (registered in `commands/auth.rs`):

```rust
#[tauri::command]
async fn check_auth() -> Result<String, AppError>;  // returns "first_run"|"locked"|"recovery"

#[tauri::command]
async fn initialize_app() -> Result<Vec<String>, AppError>;  // returns 24 words

#[tauri::command]
async fn unlock_app() -> Result<bool, AppError>;  // triggers Touch ID

#[tauri::command]
async fn recover_app(words: Vec<String>) -> Result<bool, AppError>;

#[tauri::command]
async fn lock_app() -> Result<(), AppError>;
```

**State management**: `AuthState` stored in `tauri::State<Mutex<AuthState>>`, injected into all commands that need keys.

**Key constants**:

```rust
const KEYCHAIN_SERVICE: &str = "ch.dokassist.app";
const DB_KEY_ACCOUNT: &str = "db.master-key";
const FS_KEY_ACCOUNT: &str = "fs.master-key";
const RECOVERY_FILENAME: &str = "recovery.vault";
```

**Acceptance criteria**:

- [ ] `generate_key()` produces 32 random bytes, never the same twice
- [ ] `encrypt()` → `decrypt()` round-trips correctly for payloads from 0 bytes to 100 MB
- [ ] Keychain store/retrieve triggers Touch ID dialog on macOS
- [ ] `BIOMETRY_CURRENT_SET` flag verified: enrolling new fingerprint invalidates stored keys
- [ ] Recovery mnemonic: generate → write vault → recover from mnemonic produces identical keys
- [ ] `recovery.vault` is not decryptable with wrong mnemonic
- [ ] All key material uses `zeroize` on drop
- [ ] Auth state machine transitions: FirstRun → Unlocked, Locked → Unlocked, RecoveryRequired → Unlocked

**Effort**: ~14h

-----