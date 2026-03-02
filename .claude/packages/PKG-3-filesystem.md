## PKG-3 — Encrypted Filesystem (File Vault)

**Goal**: Encrypt, store, retrieve, and delete patient files on the local filesystem.

**Depends on**: PKG-1 (needs `fs_key` from auth state)

**Files**:

```
src-tauri/src/
├── filesystem.rs    # Core vault operations
└── spotlight.rs     # macOS Spotlight exclusion helper
```

**Public interface**:

```rust
// === filesystem.rs ===

/// Initialize the vault directory structure. Creates ~/DokAssist/vault/ if needed.
/// Sets .metadata_never_index and adds to Spotlight privacy list.
pub fn init_vault(base_dir: &Path) -> Result<(), AppError>;

/// Encrypt a file and store it in the patient's vault subdirectory.
/// Returns the vault-relative path (e.g., "<patient-uuid>/<file-uuid>.enc").
pub fn store_file(
    base_dir: &Path,
    fs_key: &[u8; 32],
    patient_id: &str,
    original_filename: &str,
    plaintext: &[u8],
) -> Result<String, AppError>;

/// Decrypt a file from the vault. Returns the plaintext bytes.
pub fn read_file(
    base_dir: &Path,
    fs_key: &[u8; 32],
    vault_path: &str,
) -> Result<Vec<u8>, AppError>;

/// Delete an encrypted file from the vault.
pub fn delete_file(base_dir: &Path, vault_path: &str) -> Result<(), AppError>;

/// Export a file to a temporary decrypted location.
/// Returns the temp path. Caller must schedule cleanup.
pub fn export_temp(
    base_dir: &Path,
    fs_key: &[u8; 32],
    vault_path: &str,
    original_filename: &str,
) -> Result<PathBuf, AppError>;

/// Clean up expired temp exports (older than `max_age`).
pub fn cleanup_exports(base_dir: &Path, max_age: Duration) -> Result<u32, AppError>;

/// Get total vault size in bytes for a patient.
pub fn patient_vault_size(base_dir: &Path, patient_id: &str) -> Result<u64, AppError>;
```

**Tauri commands** (`commands/files.rs`):

```rust
/// Upload a file: receives bytes from frontend, encrypts, stores, creates DB record,
/// triggers async LLM metadata extraction.
#[tauri::command]
async fn upload_file(
    state: State<'_, AppState>,
    patient_id: String,
    filename: String,
    data: Vec<u8>,        // Tauri IPC supports binary transfer
    mime_type: String,
) -> Result<FileRecord, AppError>;

/// Download/view a file: decrypt and return bytes to frontend.
#[tauri::command]
async fn download_file(
    state: State<'_, AppState>,
    vault_path: String,
) -> Result<Vec<u8>, AppError>;

/// Delete a file: remove from vault + DB record.
#[tauri::command]
async fn delete_file(
    state: State<'_, AppState>,
    file_id: String,
) -> Result<(), AppError>;

/// Export a file to a user-chosen location (triggers save dialog).
#[tauri::command]
async fn export_file(
    state: State<'_, AppState>,
    file_id: String,
) -> Result<String, AppError>;
```

**Spotlight exclusion** (`spotlight.rs`):

```rust
/// Add directory to Spotlight privacy exclusions via `mdutil` and `.metadata_never_index`
pub fn exclude_from_spotlight(dir: &Path) -> Result<(), AppError>;
```

**Acceptance criteria**:

- [ ] Store → read round-trips: decrypted output == original input for files 1 KB to 500 MB
- [ ] Wrong key returns `AppError::Crypto`, not corrupted data
- [ ] Patient directory created on first file upload
- [ ] `.enc` files have no readable headers (no magic bytes, no filename leakage)
- [ ] Spotlight exclusion verified: `mdls ~/DokAssist/vault/` returns "no metadata"
- [ ] Temp export auto-cleanup works after configured duration
- [ ] Deleting a file removes it from disk (not just unlinked)
- [ ] `store_file` with duplicate filename generates unique vault path (UUID-based)

**Effort**: ~10h

-----