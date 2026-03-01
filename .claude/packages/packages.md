# DokAssist — Implementation Packages

> Each package is a self-contained unit of work with explicit interface contracts,
> dependencies, acceptance criteria, and test specifications. Packages can be
> assigned to sub-agents in dependency order.

-----

## Dependency Graph

```
PKG-0 (Scaffold)
  ├──▶ PKG-1 (Crypto Core + Keychain)
  │      ├──▶ PKG-2 (Database / SQLCipher)
  │      ├──▶ PKG-3 (Encrypted Filesystem)
  │      └──▶ PKG-9 (Backup & Recovery)
  ├──▶ PKG-4 (LLM Client)
  ├──▶ PKG-5 (Search Engine) ◀── PKG-2
  ├──▶ PKG-6 (Audit Logger) ◀── PKG-2
  └──▶ PKG-7 (Frontend Shell + Auth) ◀── PKG-1
         ├──▶ PKG-8  (Frontend: Patients)      ◀── PKG-2, PKG-5
         ├──▶ PKG-9a (Frontend: Files)          ◀── PKG-3, PKG-4
         ├──▶ PKG-10 (Frontend: Clinical)       ◀── PKG-2
         ├──▶ PKG-11 (Frontend: Reports + PDF)  ◀── PKG-4, PKG-2
         └──▶ PKG-12 (Build & Distribution)     ◀── ALL
```

**Critical path**: PKG-0 → PKG-1 → PKG-2 → PKG-5 → PKG-8 (patient list with search is the first usable feature)

**Parallelizable after PKG-1**: PKG-2, PKG-3, PKG-4 can all be built concurrently.

-----

## PKG-0 — Project Scaffold

**Goal**: Empty Tauri 2 + Svelte 5 app that compiles, opens a window, and runs.

**Outputs**:

```
dokassist/
├── src-tauri/
│   ├── Cargo.toml           # All dependencies declared (even if unused yet)
│   ├── tauri.conf.json       # App name, window config, permissions
│   ├── build.rs
│   ├── src/
│   │   ├── main.rs           # Tauri entry with empty command registrations
│   │   ├── lib.rs            # Module declarations
│   │   ├── commands/mod.rs   # Empty command module
│   │   ├── models/mod.rs     # Empty models module
│   │   └── error.rs          # AppError enum with thiserror
│   ├── capabilities/
│   │   └── default.json      # Tauri capability permissions
│   └── icons/                # App icons (placeholder)
├── src/
│   ├── app.html
│   ├── app.css               # Tailwind v4 base
│   ├── lib/
│   │   ├── api.ts            # Empty typed invoke wrappers
│   │   ├── types.ts          # Shared TypeScript interfaces (empty stubs)
│   │   └── stores/
│   │       └── auth.ts       # Auth state store (stub)
│   └── routes/
│       ├── +layout.svelte    # Root layout with slot
│       └── +page.svelte      # "Hello DokAssist" placeholder
├── package.json
├── pnpm-lock.yaml
├── svelte.config.js
├── vite.config.ts
├── tailwind.config.js
├── tsconfig.json
└── README.md
```

**Cargo.toml dependencies** (declare all upfront, features gated):

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
uuid = { version = "1", features = ["v7"] }
chrono = { version = "0.4", features = ["serde"] }
tokio = { version = "1", features = ["full"] }
log = "0.4"
env_logger = "0.11"

# PKG-1
security-framework = "3"
aes-gcm = "0.10"
ring = "0.17"
argon2 = "0.5"

# PKG-2
rusqlite = { version = "0.32", features = ["bundled-sqlcipher", "vtab"] }

# PKG-4 (model download)
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio-stream = "0.1"
llama_cpp = { version = "0.4", features = ["metal"] }

# PKG-3 / misc
pdf-extract = "0.7"
rand = "0.8"
base64 = "0.22"
hex = "0.4"
bip39 = "2"
zeroize = { version = "1", features = ["derive"] }
```

**`tauri.conf.json` key settings**:

```json
{
  "productName": "DokAssist",
  "identifier": "ch.dokassist.app",
  "app": {
    "windows": [
      {
        "title": "DokAssist",
        "width": 1280,
        "height": 800,
        "minWidth": 900,
        "minHeight": 600
      }
    ]
  }
}
```

**Error type** (`src-tauri/src/error.rs`):

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Keychain error: {0}")]
    Keychain(String),
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Filesystem error: {0}")]
    Filesystem(#[from] std::io::Error),
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Auth required")]
    AuthRequired,
    #[error("Invalid recovery passphrase")]
    InvalidRecoveryPhrase,
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        serializer.serialize_str(&self.to_string())
    }
}
```

**Acceptance criteria**:

- [ ] `pnpm tauri dev` opens a native macOS window with "Hello DokAssist"
- [ ] `pnpm tauri build` produces a `.dmg` that installs and runs
- [ ] All Cargo dependencies compile without errors
- [ ] Svelte hot-reload works during development
- [ ] App icon shows in Dock

**Effort**: ~3h

-----

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

## PKG-2 — Database Module (SQLCipher)

**Goal**: Encrypted SQLite database with schema migrations, connection pool, and typed CRUD operations for all entities.

**Depends on**: PKG-1 (needs `db_key` from auth state)

**Files**:

```
src-tauri/src/
├── database.rs           # Connection management, migrations
├── models/
│   ├── mod.rs
│   ├── patient.rs        # Patient struct + CRUD
│   ├── session.rs        # Session struct + CRUD
│   ├── file_record.rs    # File metadata struct + CRUD (not the file itself)
│   ├── diagnosis.rs      # Diagnosis struct + CRUD
│   ├── medication.rs     # Medication struct + CRUD
│   └── report.rs         # Report struct + CRUD
└── migrations/
    └── 001_initial.sql   # Full schema from architecture doc
```

**Public interface**:

```rust
// === database.rs ===

/// Initialize the database connection with SQLCipher encryption.
/// Runs migrations if needed. Returns a connection pool handle.
pub fn init_db(db_path: &Path, key: &[u8; 32]) -> Result<DbPool, AppError>;

/// Wrapper around r2d2 + rusqlite connection pool.
pub struct DbPool { /* ... */ }

impl DbPool {
    pub fn conn(&self) -> Result<PooledConnection, AppError>;
}


// === models/patient.rs ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patient {
    pub id: String,
    pub ahv_number: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub insurance: Option<String>,
    pub gp_name: Option<String>,
    pub gp_address: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePatient {
    pub ahv_number: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_birth: String,
    pub gender: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub insurance: Option<String>,
    pub gp_name: Option<String>,
    pub gp_address: Option<String>,
    pub notes: Option<String>,
}

pub fn create_patient(conn: &Connection, input: CreatePatient) -> Result<Patient, AppError>;
pub fn get_patient(conn: &Connection, id: &str) -> Result<Patient, AppError>;
pub fn update_patient(conn: &Connection, id: &str, input: UpdatePatient) -> Result<Patient, AppError>;
pub fn delete_patient(conn: &Connection, id: &str) -> Result<(), AppError>;
pub fn list_patients(conn: &Connection, limit: u32, offset: u32) -> Result<Vec<Patient>, AppError>;
```

**Same CRUD pattern for**: `Session`, `FileRecord`, `Diagnosis`, `Medication`, `Report`

**Tauri commands** (`commands/patients.rs`, etc.):

```rust
#[tauri::command]
async fn create_patient(state: State<'_, AppState>, input: CreatePatient) -> Result<Patient, AppError>;

#[tauri::command]
async fn get_patient(state: State<'_, AppState>, id: String) -> Result<Patient, AppError>;

#[tauri::command]
async fn list_patients(state: State<'_, AppState>, limit: u32, offset: u32) -> Result<Vec<Patient>, AppError>;

// ... update, delete for each entity
```

**Migration system**: Simple sequential SQL files. On `init_db`, check a `schema_version` pragma, run any unapplied migrations in order.

**AHV number validation** (built into `create_patient`):

```rust
/// Validates Swiss AHV/AVS number format: 756.XXXX.XXXX.XX
/// Accepts both dotted and plain 13-digit formats.
pub fn validate_ahv(ahv: &str) -> Result<String, AppError>;
```

**Acceptance criteria**:

- [ ] Database opens only with correct key; wrong key returns error, not garbage data
- [ ] All migrations run idempotently
- [ ] Patient CRUD: create → read → update → delete works end-to-end
- [ ] AHV validation rejects invalid formats, normalizes to dotted format
- [ ] UUIDv7 IDs are time-sortable
- [ ] Connection pool handles concurrent reads from Tauri async commands
- [ ] All entity types have working CRUD operations
- [ ] `updated_at` auto-updates on modifications

**Effort**: ~16h

-----

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

## PKG-4 — LLM Engine (Embedded Inference — No External Dependencies)

**Goal**: Embed LLM inference directly into the Tauri binary. Zero external dependencies.
The doctor double-clicks the app and it runs. No separate installs, no background process,
no `brew install`, no terminal.

**Depends on**: PKG-0 only (no crypto dependency — operates on already-decrypted text)

### Architecture Decision: Embedded vs External

|                 |External daemon (rejected)                  |Embedded llama.cpp              |Embedded MLX                   |
|-----------------|--------------------------------------------|--------------------------------|-------------------------------|
|**User setup**   |Must install daemon + pull models separately|Zero — ships with app           |Zero — ships with app          |
|**Process model**|Separate daemon on localhost                |In-process, same binary         |In-process, same binary        |
|**Apple Silicon**|Metal via llama.cpp internally              |Metal backend (feature flag)    |Native Apple GPU, 20-30% faster|
|**MoE support**  |Yes (GGUF)                                  |Yes (GGUF MoE models)           |Yes (MLX format MoE)           |
|**Maturity**     |Stable                                      |Stable (`llama_cpp` crate v0.4+)|Active dev (`mlx-rs` v0.25)    |
|**Failure mode** |"Daemon not running" banner                 |Always available                |Always available               |

**Decision**: Use `llama_cpp` Rust crate (edgenai) for Phase 1. Provides safe high-level
bindings, GGUF format support (including MoE architectures), Metal acceleration on Apple
Silicon, and streaming token generation. Migrate to `mlx-rs` when it stabilizes for the
20-30% speed improvement on Apple Silicon.

### Model Selection for 16 GB / 24 GB Macs

The task is narrow: German psychiatric report generation + document metadata extraction.
We don't need a general-purpose genius — we need reliable structured German output.

**Recommended models (GGUF format)**:

|Model                   |Total Params|Active Params|GGUF Q4 Size|16 GB Mac  |24 GB Mac|Why                                |
|------------------------|------------|-------------|------------|-----------|---------|-----------------------------------|
|**Qwen 3 8B** (dense)   |8B          |8B           |~5 GB       |✅ Best fit |✅        |Best 8B all-rounder, strong German |
|**Qwen 2.5 7B** (dense) |7B          |7B           |~4.5 GB     |✅          |✅        |Excellent structured output / JSON |
|**Qwen 3 30B-A3B** (MoE)|30B         |3B           |~18 GB      |❌ Too large|✅ Perfect|MoE: 30B knowledge, 3B compute cost|
|Phi-4 Mini 3.8B         |3.8B        |3.8B         |~2.5 GB     |✅ Fast     |✅        |Lightweight fallback               |

**Strategy**:

- On 24 GB Mac Mini: Ship with **Qwen 3 30B-A3B MoE Q4** — gets you 30B-class
  quality at 3B inference speed. This is the MoE sweet spot.
- On 16 GB MacBook Air: Ship with **Qwen 3 8B Q4** — best quality that fits.
- App auto-detects available RAM at startup and selects the appropriate model.
- Models stored in `~/DokAssist/models/` (~5–18 GB depending on selection).
- First launch: download model from HuggingFace with progress bar.

### Why MoE Matters Here

For this use case, MoE is not just a performance trick — it's a quality unlock.
The doctor's reports need domain knowledge (psychiatry, German medical terminology,
ICD-10, medication interactions) that benefits from a larger parameter space.

A dense 8B model has 8B parameters, all active. A 30B MoE with 3B active has 30B
parameters of "stored knowledge" but only routes each token through ~3B of compute.
The router learns which experts handle medical terminology, which handle formal
German prose, etc. The result: near-30B quality at 8B speed, with LESS memory
pressure during inference than a dense 8B (because only expert subset is computed).

On the 24GB Mac Mini this is the clear winner.

**Files**:

```
src-tauri/src/
├── llm/
│   ├── mod.rs          # LlmEngine struct — wraps llama_cpp
│   ├── engine.rs       # Model loading, session management, generation
│   ├── extract.rs      # Metadata extraction prompts + JSON parsing
│   ├── report.rs       # Report generation prompts + streaming
│   ├── prompts.rs      # Prompt templates (German psychiatric)
│   └── download.rs     # Model downloader with progress reporting
```

**Public interface**:

```rust
// === llm/engine.rs ===

use llama_cpp::{LlamaModel, LlamaParams, SessionParams};

pub struct LlmEngine {
    model: Option<LlamaModel>,
    model_path: PathBuf,
    model_name: String,
}

impl LlmEngine {
    /// Create engine. Does NOT load model yet (deferred to first use or explicit load).
    pub fn new(models_dir: &Path) -> Self;

    /// Detect available RAM and select the best model.
    pub fn recommended_model() -> ModelChoice;

    /// Load a model into memory. Call once at startup or after model switch.
    pub fn load_model(&mut self, model_path: &Path) -> Result<(), AppError>;

    /// Unload model from memory (frees RAM).
    pub fn unload_model(&mut self);

    /// Check if a model is loaded and ready.
    pub fn is_ready(&self) -> bool;

    /// Generate a completion (blocking). Used for metadata extraction.
    pub fn generate(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        max_tokens: u32,
        temperature: f32,
    ) -> Result<String, AppError>;

    /// Generate with streaming callback. Used for report generation.
    /// Calls `on_token` for each generated token. Return `false` to stop.
    pub fn generate_streaming(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        max_tokens: u32,
        temperature: f32,
        on_token: impl FnMut(&str) -> bool,
    ) -> Result<String, AppError>;
}

#[derive(Debug, Serialize)]
pub struct ModelChoice {
    pub name: String,
    pub filename: String,           // e.g. "qwen3-30b-a3b-q4_k_m.gguf"
    pub download_url: String,       // HuggingFace direct link
    pub size_bytes: u64,
    pub is_moe: bool,
    pub total_params: String,       // "30B"
    pub active_params: String,      // "3B" for MoE, same as total for dense
    pub reason: String,             // "Best fit for 24GB: MoE with 30B knowledge"
}

#[derive(Debug, Serialize)]
pub struct EngineStatus {
    pub model_loaded: bool,
    pub model_name: Option<String>,
    pub model_size_bytes: Option<u64>,
    pub available_ram_bytes: u64,
    pub is_moe: bool,
}


// === llm/download.rs ===

/// Download a model from HuggingFace with progress reporting.
/// Emits Tauri events: "model-download-progress" (0.0–1.0), "model-download-done".
pub async fn download_model(
    app: &AppHandle,
    model: &ModelChoice,
    dest_dir: &Path,
) -> Result<PathBuf, AppError>;

/// Check if a model file exists and matches expected size.
pub fn model_exists(models_dir: &Path, filename: &str, expected_size: u64) -> bool;


// === llm/extract.rs ===

#[derive(Debug, Serialize, Deserialize)]
pub struct FileMetadata {
    pub document_type: String,       // "Laborbericht", "Überweisung", "Rezept", etc.
    pub date: Option<String>,
    pub author: Option<String>,
    pub summary: String,             // 1-2 sentence German summary
    pub keywords: Vec<String>,
    pub patient_name_found: Option<String>,
    pub ahv_found: Option<String>,
}

/// Extract metadata from document text using the embedded LLM.
pub fn extract_metadata(
    engine: &LlmEngine,
    document_text: &str,
) -> Result<FileMetadata, AppError>;


// === llm/report.rs ===

#[derive(Debug, Deserialize)]
pub struct ReportContext {
    pub patient: Patient,
    pub diagnoses: Vec<Diagnosis>,
    pub medications: Vec<Medication>,
    pub sessions: Vec<Session>,
    pub report_type: ReportType,
}

#[derive(Debug, Deserialize)]
pub enum ReportType {
    Befundbericht,
    Verlaufsbericht,
    Ueberweisungsschreiben,
}

/// Build the prompt for report generation from clinical context.
pub fn build_report_prompt(context: &ReportContext) -> String;

/// Generate a report with streaming to frontend via Tauri events.
pub fn generate_report_streaming(
    app: &AppHandle,
    engine: &LlmEngine,
    context: &ReportContext,
) -> Result<String, AppError>;
```

**Tauri commands** (`commands/llm.rs`):

```rust
#[tauri::command]
async fn get_engine_status(state: State<'_, AppState>) -> Result<EngineStatus, AppError>;

#[tauri::command]
async fn get_recommended_model() -> Result<ModelChoice, AppError>;

#[tauri::command]
async fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    model: ModelChoice,
) -> Result<(), AppError>;

#[tauri::command]
async fn load_model(
    state: State<'_, AppState>,
    model_filename: String,
) -> Result<(), AppError>;

#[tauri::command]
async fn extract_file_metadata(
    state: State<'_, AppState>,
    file_id: String,
) -> Result<FileMetadata, AppError>;

/// Streaming report — uses Tauri event system to push chunks to frontend.
#[tauri::command]
async fn generate_report(
    app: AppHandle,
    state: State<'_, AppState>,
    patient_id: String,
    report_type: String,
    session_ids: Vec<String>,
) -> Result<String, AppError>;
```

**Streaming pattern** (Tauri events for real-time UI):

```rust
// Backend: generate_streaming calls on_token, which emits events
engine.generate_streaming(system, prompt, 2048, 0.3, |token| {
    app.emit("report-chunk", token).is_ok()
})?;
app.emit("report-done", &final_text)?;

// Frontend listens
import { listen } from '@tauri-apps/api/event';
const unlisten = await listen('report-chunk', (event) => {
    reportText += event.payload;
});
```

**Cargo dependencies**:

```toml
# In Cargo.toml — embedded LLM inference
llama_cpp = { version = "0.4", features = ["metal"] }  # Metal for Apple Silicon GPU

# Model download from HuggingFace
reqwest = { version = "0.12", features = ["json", "stream"] }
tokio-stream = "0.1"
```

**First-launch flow**:

```
App opens → no model found in ~/DokAssist/models/
    │
    ▼
Detect RAM → recommend model (MoE for 24GB, dense 8B for 16GB)
    │
    ▼
Show: "DokAssist needs to download a language model (~5–18 GB).
       This is a one-time download. The model runs entirely on your Mac."
    │
    ▼
[Download] button → progress bar → model saved to ~/DokAssist/models/
    │
    ▼
Auto-load model → engine ready → green status indicator
```

**Prompt templates** stored in `prompts.rs` as const strings — German psychiatric language,
structured for each report type. Temperature 0.3, context window 8192 tokens.

**Acceptance criteria**:

- [ ] Model loads from GGUF file on disk using Metal backend (Apple Silicon GPU)
- [ ] `generate()` produces coherent German text from psychiatric prompts
- [ ] `generate_streaming()` delivers tokens to callback in real-time
- [ ] `extract_metadata()` returns valid JSON-parsed `FileMetadata` for sample German medical PDFs
- [ ] RAM auto-detection correctly recommends MoE for ≥24GB, dense 8B for 16GB
- [ ] Model download with progress bar works (HuggingFace HTTPS)
- [ ] Model download is resumable (partial file detection)
- [ ] Graceful handling when no model is downloaded yet (setup flow, not crash)
- [ ] Model unload actually frees memory
- [ ] Streaming report generation pushes chunks to frontend (< 200ms per token)
- [ ] Prompt templates produce clinically reasonable German output for test cases
- [ ] No network calls after initial model download (fully offline capable)

**Future migration path (PKG-4b)**: When `mlx-rs` stabilizes with LLM inference support,
create an `MlxEngine` implementing the same `LlmEngine` trait. The rest of the codebase
(prompts, extraction, report generation) stays identical — only the inference backend swaps.
This gives a 20-30% speed boost on Apple Silicon for free.

**Effort**: ~16h (4h more than HTTP client version due to model management + download flow)

-----

## PKG-5 — Search Engine (FTS5)

**Goal**: Unified full-text search across patients, files, sessions, and reports.

**Depends on**: PKG-2 (database must exist with FTS5 virtual table)

**Files**:

```
src-tauri/src/
├── search.rs         # FTS5 indexing + querying
```

**Public interface**:

```rust
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub result_type: String,     // "patient", "file", "session", "report"
    pub entity_id: String,
    pub patient_id: String,
    pub patient_name: String,
    pub title: String,           // display title for result
    pub snippet: String,         // highlighted match context
    pub date: Option<String>,
    pub rank: f64,
}

/// Full-text search across all indexed content.
pub fn search(conn: &Connection, query: &str, limit: u32) -> Result<Vec<SearchResult>, AppError>;

/// Index or re-index a patient's searchable fields.
pub fn index_patient(conn: &Connection, patient: &Patient) -> Result<(), AppError>;

/// Index file content (called after LLM metadata extraction).
pub fn index_file(conn: &Connection, file: &FileRecord, extracted_text: &str) -> Result<(), AppError>;

/// Index session notes.
pub fn index_session(conn: &Connection, session: &Session) -> Result<(), AppError>;

/// Index finalized report content.
pub fn index_report(conn: &Connection, report: &Report) -> Result<(), AppError>;

/// Remove all index entries for an entity.
pub fn remove_from_index(conn: &Connection, entity_type: &str, entity_id: &str) -> Result<(), AppError>;
```

**AHV search normalization**:

```rust
/// Normalize AHV queries: "7561234567897" and "756.1234.5678.97" both match.
fn normalize_ahv_for_search(query: &str) -> String;
```

**Tauri commands** (`commands/search.rs`):

```rust
#[tauri::command]
async fn global_search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SearchResult>, AppError>;
```

**Acceptance criteria**:

- [ ] Search by patient name returns correct patients (partial match, case-insensitive)
- [ ] Search by AHV number works in both dotted and plain formats
- [ ] Search by file content returns files with matching extracted text
- [ ] Results ranked by relevance (FTS5 rank)
- [ ] Snippets contain `<mark>` tags around matched terms
- [ ] Unicode/German characters handled correctly (ä, ö, ü, ß)
- [ ] Diacritics-insensitive: searching "muller" matches "Müller"
- [ ] Re-indexing updates results (not duplicates)
- [ ] Search returns in < 50ms for databases with 1000+ patients

**Effort**: ~8h

-----

## PKG-6 — Audit Logger

**Goal**: Append-only logging of all data access and modifications for nDSG compliance.

**Depends on**: PKG-2 (database)

**Files**:

```
src-tauri/src/
├── audit.rs
```

**Public interface**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    View,
    Create,
    Update,
    Delete,
    Export,
    LlmQuery,
    Login,
    Logout,
    RecoveryUsed,
}

/// Log an auditable action. Call this from every command that touches patient data.
pub fn log(
    conn: &Connection,
    action: AuditAction,
    entity_type: &str,
    entity_id: Option<&str>,
    details: Option<&str>,
) -> Result<(), AppError>;

/// Query audit log with filters.
pub fn query_log(
    conn: &Connection,
    entity_type: Option<&str>,
    entity_id: Option<&str>,
    from: Option<&str>,       // ISO 8601
    to: Option<&str>,         // ISO 8601
    limit: u32,
    offset: u32,
) -> Result<Vec<AuditEntry>, AppError>;

#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub timestamp: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub details: Option<String>,
}
```

**Integration pattern**: Every Tauri command calls `audit::log()` before returning:

```rust
#[tauri::command]
async fn get_patient(state: State<'_, AppState>, id: String) -> Result<Patient, AppError> {
    let conn = state.db.conn()?;
    let patient = models::patient::get_patient(&conn, &id)?;
    audit::log(&conn, AuditAction::View, "patient", Some(&id), None)?;
    Ok(patient)
}
```

**Acceptance criteria**:

- [ ] Every patient data access generates an audit entry
- [ ] Audit table has no UPDATE or DELETE operations exposed
- [ ] Log entries contain no PHI (no patient names, only UUIDs)
- [ ] `details` field used for update diffs (field names changed, not values)
- [ ] Query log with date range filtering works
- [ ] Audit log readable from Settings UI

**Effort**: ~4h

-----

## PKG-7 — Frontend Shell + Auth Flow

**Goal**: App chrome (sidebar, top bar, routing) and the authentication/onboarding UI.

**Depends on**: PKG-1 (auth commands)

**Files**:

```
src/
├── routes/
│   ├── +layout.svelte        # Main layout: sidebar + content area
│   ├── +page.svelte           # Redirect to /patients or /unlock
│   ├── unlock/
│   │   └── +page.svelte       # Touch ID prompt screen
│   ├── setup/
│   │   └── +page.svelte       # First-run: show 24 words, confirm
│   └── recover/
│       └── +page.svelte       # Recovery: enter 24 words
├── lib/
│   ├── stores/
│   │   └── auth.ts            # Auth state store (reactive)
│   ├── components/
│   │   ├── Sidebar.svelte     # Navigation sidebar
│   │   ├── TopBar.svelte      # Search bar + LLM status indicator
│   │   └── MnemonicDisplay.svelte   # 24-word grid display
│   └── api.ts                 # Typed Tauri invoke wrappers for auth
```

**Auth flow (frontend)**:

```
App opens
    │
    ▼
check_auth() ──▶ "first_run"  ──▶ /setup  (generate keys, show 24 words)
    │                                        │
    │                                        ▼
    │                               confirm 3 random words
    │                                        │
    │                                        ▼
    ├──────────────────────────── /patients (main app)
    │                                        ▲
    ▼                                        │
"locked" ──▶ /unlock (Touch ID prompt) ──────┘
    │
    ▼
"recovery" ──▶ /recover (enter 24 words) ────┘
```

**Sidebar navigation items**:

- Patients (main list)
- Calendar
- Settings
- Lock button (bottom)

**Acceptance criteria**:

- [ ] First launch shows setup flow with 24-word mnemonic grid
- [ ] Mnemonic confirmation requires re-entering 3 random words correctly
- [ ] Subsequent launches show Touch ID prompt
- [ ] Successful auth navigates to patient list
- [ ] Lock button zeroes keys and returns to unlock screen
- [ ] Sidebar highlights current route
- [ ] Cmd+K focuses search bar from any page
- [ ] LLM status indicator: green dot (model loaded) / red dot (no model)
- [ ] Responsive layout: sidebar collapses at narrow widths

**Effort**: ~14h

-----

## PKG-8 — Frontend: Patient Management

**Goal**: Patient list, detail view, create/edit forms.

**Depends on**: PKG-2, PKG-5, PKG-7

**Files**:

```
src/routes/
├── patients/
│   ├── +page.svelte              # Patient list with search
│   ├── [id]/
│   │   ├── +page.svelte          # Patient detail (tabs)
│   │   └── +layout.svelte        # Tab navigation for patient subpages
│   └── new/
│       └── +page.svelte          # Create patient form
src/lib/components/
├── PatientCard.svelte             # List item card
├── PatientForm.svelte             # Create/edit form (shared)
├── AhvInput.svelte                # AHV number input with formatting/validation
└── PatientTabs.svelte             # Tab bar: Overview | Sessions | Files | Diagnoses | Meds | Reports
```

**Patient list features**:

- Search bar (calls `global_search` — filters to patients)
- Sort by: last name, last visit, created date
- AHV number displayed in formatted form
- Click to navigate to detail view

**Patient detail tabs**: Overview, Sessions, Files, Diagnoses, Medications, Reports
(each tab's content is built in PKG-9a, PKG-10, PKG-11)

**AHV input component**: Auto-formats as user types (`756.____.____.__ `), validates checksum.

**Acceptance criteria**:

- [ ] Create patient with all fields, AHV validated
- [ ] Patient list loads and displays correctly
- [ ] Search filters patient list in real-time (debounced, < 100ms perceived)
- [ ] Edit patient details, changes persist
- [ ] Delete patient with confirmation dialog
- [ ] Tab navigation on detail page works
- [ ] AHV input auto-formats and validates

**Effort**: ~12h

-----

## PKG-9 — Backup & Recovery

**Goal**: Built-in backup tooling and recovery flow.

**Depends on**: PKG-1, PKG-3

**Files**:

```
src-tauri/src/
├── backup.rs
src/routes/settings/
├── backup/
│   └── +page.svelte
```

**Public interface**:

```rust
/// Create a backup of the entire DokAssist data directory to a target path.
/// Copies: dokassist.db, recovery.vault, entire vault/ directory.
/// All files are already encrypted — this is a plain file copy.
pub fn create_backup(source_dir: &Path, target_dir: &Path) -> Result<BackupReport, AppError>;

/// Verify a backup: checks all expected files exist and are non-zero.
pub fn verify_backup(backup_dir: &Path) -> Result<BackupReport, AppError>;

/// Restore from backup: copy files to data directory, then trigger recovery flow.
pub fn restore_from_backup(backup_dir: &Path, target_dir: &Path) -> Result<(), AppError>;

#[derive(Serialize)]
pub struct BackupReport {
    pub files_copied: u32,
    pub total_size_bytes: u64,
    pub timestamp: String,
    pub vault_file_count: u32,
    pub db_present: bool,
    pub recovery_vault_present: bool,
}
```

**Tauri commands**:

```rust
/// Trigger backup to user-selected directory (opens native folder picker).
#[tauri::command]
async fn create_backup(state: State<'_, AppState>) -> Result<BackupReport, AppError>;

/// Verify an existing backup directory.
#[tauri::command]
async fn verify_backup(path: String) -> Result<BackupReport, AppError>;
```

**Settings UI**: Backup section shows last backup date, button to "Backup Now" (opens folder picker), button to "Verify Backup".

**Acceptance criteria**:

- [ ] Backup copies all files to selected external drive
- [ ] Backup report shows file count, total size, and verifies completeness
- [ ] Verify detects missing or zero-byte files
- [ ] Restore + recovery passphrase entry results in working app on new Mac
- [ ] Backup does not include any decrypted/temp files

**Effort**: ~8h

-----

## PKG-9a — Frontend: File Browser + Upload

**Goal**: File browser tab on patient detail, drag-and-drop upload with LLM indexing.

**Depends on**: PKG-3, PKG-4, PKG-2

**Files**:

```
src/routes/patients/[id]/
├── files/
│   └── +page.svelte
src/lib/components/
├── FileUploader.svelte        # Drag-and-drop zone
├── FileCard.svelte            # File list item with metadata tags
└── FileViewer.svelte          # In-app PDF/image viewer (decrypted in memory)
```

**Upload flow**:

```
User drops file ──▶ Frontend reads bytes
    │
    ▼
invoke('upload_file', { patientId, filename, data, mimeType })
    │
    ▼
Backend: encrypt → store in vault → create DB record → return FileRecord
    │
    ▼
async: extract text (PDF/OCR) → send to embedded LLM → parse metadata → update DB → index in FTS5
    │
    ▼
Frontend: file card updates with tags, summary, document type (reactive via polling or event)
```

**File viewer**: Decrypt in memory, display in-app. PDFs via `<iframe>` or `<embed>` with blob URL. Images via `<img>` with blob URL. Blob URLs revoked after viewing.

**Acceptance criteria**:

- [ ] Drag-and-drop upload works for PDF, PNG, JPG, DOCX
- [ ] File appears in list immediately after upload (before LLM extraction)
- [ ] LLM metadata populates asynchronously (tags, summary, document type)
- [ ] Click file → decrypted view in-app (no temp file written to disk)
- [ ] Download button exports decrypted file via native save dialog
- [ ] Delete file removes from vault + DB + search index
- [ ] Files sorted by upload date, filterable by document type tag
- [ ] Upload progress indicator for large files

**Effort**: ~14h

-----

## PKG-10 — Frontend: Clinical (AMDP, Sessions, Diagnoses, Medications)

**Goal**: The clinical workflow screens — where the doctor does actual psychiatry work.

**Depends on**: PKG-2, PKG-7

**Files**:

```
src/routes/patients/[id]/
├── sessions/
│   ├── +page.svelte           # Session list
│   └── new/
│       └── +page.svelte       # New session form with AMDP
├── diagnoses/
│   └── +page.svelte           # Diagnosis list + ICD-10 search
├── medications/
│   └── +page.svelte           # Medication list + add/edit
src/lib/components/
├── AMDPForm.svelte             # AMDP psychopathological findings (12 categories)
├── AMDPCategory.svelte         # Single AMDP category with 0-3 scoring buttons
├── IcdSearch.svelte            # ICD-10-GM typeahead search
├── MedicationForm.svelte
├── SessionCard.svelte
└── DiagnosisCard.svelte
```

**AMDP form structure** (12 categories, ~140 items):

```typescript
interface AMDPCategory {
    name: string;           // e.g. "Bewusstsein"
    items: AMDPItem[];
}

interface AMDPItem {
    code: string;           // e.g. "Bew-1"
    label: string;          // e.g. "Bewusstseinsverminderung"
    score: 0 | 1 | 2 | 3;  // not present | mild | moderate | severe
}
```

Scores stored as JSON blob in `sessions.amdp_data`.

**ICD-10-GM search**: Load `static/icd10gm.json` at startup. Typeahead component searches by code and description. Data source: free XML from BfArM (formerly DIMDI), pre-converted to JSON at build time.

**Acceptance criteria**:

- [ ] New session form with free-text notes and AMDP scoring
- [ ] AMDP form: all 12 categories navigable, 0-3 tap scoring, scores persist
- [ ] Session list shows date, type, duration, and a summary snippet
- [ ] ICD-10 search returns results as user types (< 50ms, client-side)
- [ ] Add/remove diagnoses with status (active/remission/resolved)
- [ ] Medication list with substance, dose, frequency, date range
- [ ] All clinical data saves to SQLCipher and is searchable

**Effort**: ~18h

-----

## PKG-11 — Frontend: Report Generation + PDF Export

**Goal**: LLM-powered report generation with streaming preview, editing, and PDF export.

**Depends on**: PKG-4, PKG-2

**Files**:

```
src/routes/patients/[id]/
├── reports/
│   ├── +page.svelte           # Report list
│   └── new/
│       └── +page.svelte       # Report generator
src/lib/components/
├── ReportEditor.svelte         # Markdown editor with preview
├── ReportTypeSelector.svelte   # Befundbericht | Verlauf | Überweisung
└── ReportStream.svelte         # Streaming LLM output display
```

**Report generation flow**:

```
Doctor selects report type
    │
    ▼
Selects sessions/date range to include
    │
    ▼
Click "Generate" → invoke('generate_report', { ... })
    │
    ▼
Backend: assemble context → build prompt → stream from embedded LLM
    │
    ▼
Frontend: listen('report-chunk') → append to editor in real-time
    │
    ▼
Doctor edits text → clicks "Finalize"
    │
    ▼
Backend: save to reports table → generate PDF
    │
    ▼
PDF viewable / exportable
```

**PDF generation**: Rust-side using a lightweight HTML-to-PDF approach or `printpdf` crate.
Alternative: use Tauri's `webview.print()` for macOS-native PDF export from rendered HTML.

**Acceptance criteria**:

- [ ] Report type selection with context summary (what data will be included)
- [ ] LLM streaming displays text appearing in real-time
- [ ] Doctor can edit generated text before finalizing
- [ ] Finalized report saved to DB with model name and prompt hash
- [ ] PDF export with proper German formatting and practice letterhead
- [ ] Report list shows all previous reports, filterable by type
- [ ] "Regenerate" button re-runs LLM without losing the original
- [ ] Graceful handling when no model is downloaded yet (shows setup prompt)

**Effort**: ~14h

-----

## PKG-12 — Build, Distribution & Hardening

**Goal**: Production build, `.dmg` packaging, Gatekeeper signing, and final security hardening.

**Depends on**: ALL packages

**Tasks**:

- [ ] Tauri build config: CSP headers, disable devtools in production
- [ ] App Sandbox entitlements (if distributing outside App Store)
- [ ] Code signing with Apple Developer ID (optional but recommended)
- [ ] Notarization for Gatekeeper (optional)
- [ ] `.dmg` background image and install UX
- [ ] Hardcoded CSP: no remote scripts, no eval, no inline styles
- [ ] Memory zeroization audit: ensure all key material uses `zeroize`
- [ ] Disable Tauri remote URL navigation
- [ ] Penetration test: attempt to extract keys from running process
- [ ] Write user manual (2-page PDF for the doctor)

**`tauri.conf.json` security settings**:

```json
{
  "app": {
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'",
      "dangerousDisableAssetCspModification": false
    }
  }
}
```

**Acceptance criteria**:

- [ ] `pnpm tauri build` produces working `.dmg` < 30 MB (excluding bundled model)
- [ ] App launches on clean macOS 14+ without any pre-installed dependencies
- [ ] No remote network calls after initial model download (verified with Little Snitch or `nettop`)
- [ ] DevTools inaccessible in production build
- [ ] User manual covers: install, first run, daily use, backup, recovery

**Effort**: ~8h

-----

## Summary

|Package|Name                            |Effort|Dependencies       |Parallelizable With|
|-------|--------------------------------|------|-------------------|-------------------|
|PKG-0  |Scaffold                        |3h    |—                  |—                  |
|PKG-1  |Crypto + Keychain + Touch ID    |14h   |PKG-0              |—                  |
|PKG-2  |Database (SQLCipher)            |16h   |PKG-1              |PKG-3, PKG-4       |
|PKG-3  |Encrypted Filesystem            |10h   |PKG-1              |PKG-2, PKG-4       |
|PKG-4  |LLM Engine (Embedded, No Ollama)|16h   |PKG-0              |PKG-2, PKG-3       |
|PKG-5  |Search Engine (FTS5)            |8h    |PKG-2              |PKG-6              |
|PKG-6  |Audit Logger                    |4h    |PKG-2              |PKG-5              |
|PKG-7  |Frontend: Shell + Auth          |14h   |PKG-1              |—                  |
|PKG-8  |Frontend: Patients              |12h   |PKG-2, PKG-5, PKG-7|PKG-9a, PKG-10     |
|PKG-9  |Backup & Recovery               |8h    |PKG-1, PKG-3       |PKG-8              |
|PKG-9a |Frontend: Files                 |14h   |PKG-3, PKG-4       |PKG-10, PKG-11     |
|PKG-10 |Frontend: Clinical              |18h   |PKG-2, PKG-7       |PKG-9a, PKG-11     |
|PKG-11 |Frontend: Reports + PDF         |14h   |PKG-4, PKG-2       |PKG-10             |
|PKG-12 |Build & Distribution            |8h    |ALL                |—                  |

**Total: ~159h**

**Critical path**: PKG-0 → PKG-1 → PKG-2 → PKG-7 → PKG-8 (first usable patient list: ~59h)

**Maximum parallelism**: After PKG-1, three agents can work simultaneously on PKG-2, PKG-3, PKG-4. After PKG-7, frontend packages PKG-8, PKG-9a, PKG-10, PKG-11 can be partially parallelized.
