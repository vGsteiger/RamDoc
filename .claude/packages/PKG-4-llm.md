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