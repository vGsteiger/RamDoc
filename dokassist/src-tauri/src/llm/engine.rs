use super::chat_format::ChatFamily;
use super::context_cache::{
    checkpoint_for_prefix, checkpoint_position, plan_slot, reusable_prefix, ContextCacheTelemetry,
    ContextKey, InferenceSession, SessionKind, SlotAction,
};
use super::download;
use super::harness::{SamplerConfig, SamplerFamily};
use super::inference::{
    validate_context_budget, FlashAttentionMode, InferenceDiagnostics, InferenceProfile,
    KvCacheQuantization,
};
use super::memory_governor::{MemoryEstimate, MemoryGovernor, MemoryGovernorDiagnostics};
use super::thinking::ThinkingEffort;
use crate::error::AppError;
use encoding_rs::UTF_8;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::context::session::{LlamaStateSeqFlags, SeqState};
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::token::LlamaToken;
use ring::digest::{Context as DigestContext, SHA256};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

/// Sentinel value for n_gpu_layers that offloads all layers to Metal GPU.
const ALL_GPU_LAYERS: u32 = 999;
const MIN_CONTEXT_SIZE: usize = 16_384;
const STANDARD_CONTEXT_SIZE: usize = 32_768;
const LARGE_CONTEXT_SIZE: usize = 65_536;

/// Performance metrics recorded after each generation call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationStats {
    /// Wall-clock milliseconds from inference start to first token emitted.
    pub ttft_ms: f64,
    /// Tokens generated per second (generation phase, excluding prompt evaluation).
    pub tps: f64,
    /// Number of tokens in the generated completion.
    pub completion_tokens: usize,
    /// Number of tokens in the prompt.
    pub prompt_tokens: usize,
    pub evaluated_prompt_tokens: usize,
    pub reused_prompt_tokens: usize,
    pub cache_hit: bool,
    pub prefill_ms: f64,
    pub estimated_prefill_saved_ms: f64,
    pub total_latency_ms: f64,
    pub peak_rss_bytes: u64,
    /// Workload class of the generation, so cache hit rates can be split
    /// between agent conversations and one-off generations.
    #[serde(default)]
    pub session_kind: SessionKind,
}

struct CachedContext {
    context: LlamaContext<'static>,
    tokens: Vec<LlamaToken>,
    key: ContextKey,
    last_used: u64,
    /// Recurrent-state snapshots (hybrid/recurrent models only), oldest first.
    checkpoints: Vec<StateCheckpoint>,
}

/// The recurrent part of sequence 0's state after its first `n_tokens` prompt
/// tokens. Restoring it lets a hybrid model rewind to that position, which
/// KV-cache rollback alone cannot do for recurrent layers.
struct StateCheckpoint {
    n_tokens: usize,
    state: SeqState,
}

/// Prompt tokens decoded after the snapshot, so the next turn (this prompt
/// plus an answer) resumes all but this many tokens.
const CHECKPOINT_TAIL_TOKENS: usize = 64;
/// Snapshots kept per context; each holds one recurrent state.
const MAX_CHECKPOINTS_PER_CONTEXT: usize = 2;

// llama.cpp permits a context to move between threads when calls are
// serialized. ContextPool never exposes one outside its mutex lease.
unsafe impl Send for CachedContext {}

struct ContextPool {
    entries: Vec<CachedContext>,
    clock: u64,
    telemetry: ContextCacheTelemetry,
    evaluated_prefill_ms: f64,
}

impl ContextPool {
    fn new(max_contexts: usize) -> Self {
        Self {
            entries: Vec::new(),
            clock: 0,
            telemetry: ContextCacheTelemetry {
                max_contexts,
                ..ContextCacheTelemetry::default()
            },
            evaluated_prefill_ms: 0.0,
        }
    }
}

pub struct LlmEngine {
    // IMPORTANT: field declaration order controls drop order in Rust.
    // The pool is dropped first, before the boxed model its contexts borrow.
    contexts: Mutex<ContextPool>,
    // `model` must be dropped before `backend` — the LlamaModel holds a
    // raw pointer into the LlamaBackend, so freeing the backend first
    // causes a use-after-free crash in the llama.cpp C code at shutdown.
    model: Option<Box<LlamaModel>>,
    model_path: PathBuf,
    model_name: String,
    chat_template: LlamaChatTemplate,
    chat_family: ChatFamily,
    /// Vendor sampler defaults for the loaded model (issue #558).
    sampler_family: SamplerFamily,
    /// Hybrid or recurrent memory: KV rollback cannot rewind the recurrent
    /// layers, so prefix reuse relies on [`StateCheckpoint`]s.
    uses_state_checkpoints: bool,
    /// Whether the embedded template uses plain-text `<think>` blocks.
    template_uses_think_tags: bool,
    context_size: usize,
    inference: Mutex<InferenceRuntime>,
    model_hash: String,
    chat_template_hash: String,
    /// Sizes the context pool from the per-context memory estimate.
    governor: MemoryGovernor,
    total_ram: u64,
    backend: LlamaBackend,
    last_stats: Mutex<Option<GenerationStats>>,
}

#[derive(Debug, Clone)]
struct InferenceRuntime {
    profile: InferenceProfile,
    fallback: Option<String>,
    fallback_code: Option<String>,
    memory_governor: Option<MemoryGovernorDiagnostics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelChoice {
    pub name: String,
    pub filename: String,
    pub size_bytes: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStatus {
    pub is_loaded: bool,
    pub model_name: Option<String>,
    pub model_path: Option<String>,
    pub total_ram_bytes: u64,
    /// Whether the model file exists on disk (may not yet be loaded into memory).
    pub is_downloaded: bool,
    /// Filename of the downloaded model, if present on disk.
    pub downloaded_filename: Option<String>,
    /// Performance stats from the most recent generation, if any.
    pub last_generation_stats: Option<GenerationStats>,
    /// Effective llama.cpp context parameters and any explicit fallback.
    pub inference_config: Option<InferenceDiagnostics>,
    pub context_cache: ContextCacheTelemetry,
    /// Durable writing-model intent resolved from the model registry. This is
    /// deliberately separate from the engine currently resident in memory.
    pub desired_model: Option<DesiredModelStatus>,
    /// Authoritative runtime state for callers that need to gate work while a
    /// model is being loaded or released.
    pub lifecycle: EngineLifecycleStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesiredModelStatus {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub exists_on_disk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EngineLifecyclePhase {
    Idle,
    Loading,
    Ready,
    Unloading,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineLifecycleStatus {
    pub phase: EngineLifecyclePhase,
    pub requested_filename: Option<String>,
    pub active_filename: Option<String>,
    pub error: Option<String>,
}

impl Default for EngineLifecycleStatus {
    fn default() -> Self {
        Self {
            phase: EngineLifecyclePhase::Idle,
            requested_filename: None,
            active_filename: None,
            error: None,
        }
    }
}

impl LlmEngine {
    /// Load a GGUF model from disk, offloading all layers to Metal.
    pub fn load(model_path: PathBuf, model_name: String) -> Result<Self, AppError> {
        Self::load_with_profile(model_path, model_name, "conservative")
    }

    /// Load a GGUF model with a named, validated inference profile.
    pub fn load_with_profile(
        model_path: PathBuf,
        model_name: String,
        profile_name: &str,
    ) -> Result<Self, AppError> {
        let ram = Self::total_ram();
        let requested_profile = match profile_name {
            "governed" | "auto" => None,
            name => Some(InferenceProfile::named(name, runtime_context_for_ram(ram))?),
        };
        Self::load_with_requested_profile(model_path, model_name, requested_profile, ram, None)
    }

    /// Load an arbitrary validated profile for the opt-in benchmark binary.
    /// The ordinary app can only select named profiles.
    #[cfg(any(test, feature = "benchmark-harness"))]
    pub(crate) fn load_benchmark_profile(
        model_path: PathBuf,
        model_name: String,
        profile: InferenceProfile,
        verified_model_hash: Option<String>,
    ) -> Result<Self, AppError> {
        profile.validate()?;
        let ram = Self::total_ram();
        Self::load_with_requested_profile(
            model_path,
            model_name,
            Some(profile),
            ram,
            verified_model_hash,
        )
    }

    fn load_with_requested_profile(
        model_path: PathBuf,
        model_name: String,
        requested_profile: Option<InferenceProfile>,
        ram: u64,
        verified_model_hash: Option<String>,
    ) -> Result<Self, AppError> {
        let model_hash = match verified_model_hash {
            Some(hash) if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) => {
                hash.to_ascii_lowercase()
            }
            Some(_) => {
                return Err(AppError::Validation(
                    "Benchmark model SHA-256 must contain exactly 64 hexadecimal characters"
                        .to_string(),
                ))
            }
            None => sha256_file(&model_path)?,
        };
        // Parse the GGUF header before loading tensors so unsafe models are
        // rejected before unified-memory pressure can destabilise macOS.
        let governor = MemoryGovernor::inspect(&model_path, ram)?;
        let (profile, governor_diagnostics) = governor.plan(requested_profile.as_ref());
        if !governor_diagnostics.safe {
            return Err(AppError::Llm(format!(
                "Refusing to load model: {}. Select a smaller model or use a research override after freeing memory.",
                governor_diagnostics.reason
            )));
        }
        let fallback = requested_profile.as_ref().and_then(|requested| {
            (profile.n_ctx != requested.n_ctx).then(|| {
                format!(
                    "Requested {}-token context capped to model native context of {} tokens",
                    requested.n_ctx, profile.n_ctx
                )
            })
        });
        let fallback_code = fallback.as_ref().map(|_| "native_context_cap".to_string());
        let context_size = profile.n_ctx;

        let backend = LlamaBackend::init()
            .map_err(|e| AppError::Llm(format!("Failed to init llama backend: {e}")))?;

        // HIGH-3: Disable memory-mapped I/O so that a crafted GGUF file cannot
        // trigger memory-mapped reads of out-of-bounds data before the C library
        // has validated the tensor layout.  The SHA-256 pre-download check (CRIT-3)
        // already ensures model integrity; this is an additional layer of defence.
        let model_params = LlamaModelParams::default().with_n_gpu_layers(ALL_GPU_LAYERS);
        #[cfg(feature = "metal")]
        let model_params = {
            let available_devices = llama_cpp_2::list_llama_ggml_backend_devices();
            let metal_devices: Vec<usize> = available_devices
                .iter()
                .filter(|device| is_selectable_metal_device(&device.backend, &device.name))
                .map(|device| device.index)
                .collect();
            if metal_devices.is_empty() {
                let observed = available_devices
                    .iter()
                    .map(|device| format!("{}:{}", device.backend, device.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(AppError::Llm(format!(
                    "Metal inference was requested, but llama.cpp reported no Metal device; refusing a silent CPU downgrade (observed: {observed})"
                )));
            }
            model_params.with_devices(&metal_devices).map_err(|error| {
                AppError::Llm(format!(
                    "Failed to select the Metal inference device: {error}"
                ))
            })?
        };

        let model = Box::new(
            LlamaModel::load_from_file(&backend, &model_path, &model_params)
                .map_err(|e| AppError::Llm(format!("Failed to load model: {e}")))?,
        );
        let chat_template = model.chat_template(None).map_err(|e| {
            AppError::Llm(format!(
                "Model '{}' has no usable embedded chat template: {e}",
                model_name
            ))
        })?;
        let chat_template_hash = sha256_bytes(chat_template.as_c_str().to_bytes());
        let chat_family = ChatFamily::detect(&String::from_utf8_lossy(
            chat_template.as_c_str().to_bytes(),
        ));
        let uses_state_checkpoints = model.is_hybrid() || model.is_recurrent();
        let template_uses_think_tags =
            String::from_utf8_lossy(chat_template.as_c_str().to_bytes()).contains("<think>");
        let sampler_family = SamplerFamily::resolve(
            &model_name,
            &governor_diagnostics.architecture.architecture,
            chat_family,
        );
        log::info!(
            "Chat family for '{model_name}': {chat_family:?}; sampler family: {sampler_family:?}"
        );

        let max_contexts = context_pool_capacity(
            ram,
            &governor_diagnostics.estimate,
            governor_diagnostics.inference_budget_bytes,
        );
        log::info!("LLM context pool capacity: {max_contexts}");

        Ok(Self {
            contexts: Mutex::new(ContextPool::new(max_contexts)),
            backend,
            model: Some(model),
            model_path,
            model_name,
            chat_template,
            chat_family,
            sampler_family,
            uses_state_checkpoints,
            template_uses_think_tags,
            context_size,
            inference: Mutex::new(InferenceRuntime {
                profile,
                fallback,
                fallback_code,
                memory_governor: Some(governor_diagnostics),
            }),
            model_hash,
            chat_template_hash,
            governor,
            total_ram: ram,
            last_stats: Mutex::new(None),
        })
    }

    /// Contexts the pool may keep resident for `profile`.
    fn pool_capacity(&self, profile: &InferenceProfile) -> usize {
        context_pool_capacity(
            self.total_ram,
            &self.governor.estimate(profile),
            self.governor.inference_budget_bytes(),
        )
    }

    fn context_params(profile: &InferenceProfile) -> LlamaContextParams {
        let flash_policy = match profile.flash_attention {
            FlashAttentionMode::Enabled => llama_cpp_sys_2::LLAMA_FLASH_ATTN_TYPE_ENABLED,
            FlashAttentionMode::Auto => llama_cpp_sys_2::LLAMA_FLASH_ATTN_TYPE_AUTO,
        };
        LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(profile.n_ctx as u32))
            .with_n_batch(profile.n_batch)
            .with_n_ubatch(profile.n_ubatch)
            .with_type_k(profile.kv_cache.llama_type())
            .with_type_v(profile.kv_cache.llama_type())
            .with_flash_attention_policy(flash_policy)
    }

    fn create_context<'a>(&self, model: &'a LlamaModel) -> Result<LlamaContext<'a>, AppError> {
        let runtime = self
            .inference
            .lock()
            .map_err(|_| AppError::Llm("Inference profile mutex poisoned".to_string()))?
            .clone();

        let attempt = |profile: &InferenceProfile| {
            model.new_context(&self.backend, Self::context_params(profile))
        };
        let requested_error = match attempt(&runtime.profile) {
            Ok(context) => return Ok(context),
            Err(error) => error,
        };

        let mut auto_flash = runtime.profile.clone();
        auto_flash.flash_attention = FlashAttentionMode::Auto;
        if runtime.profile.flash_attention == FlashAttentionMode::Enabled {
            if let Ok(context) = attempt(&auto_flash) {
                self.record_fallback(
                    auto_flash,
                    "flash_auto",
                    "Flash Attention was rejected by this backend/model; using llama.cpp auto policy"
                        .to_string(),
                )?;
                return Ok(context);
            }
        }

        if runtime.profile.kv_cache != KvCacheQuantization::F16 {
            let mut f16_flash = runtime.profile.clone();
            f16_flash.kv_cache = KvCacheQuantization::F16;
            if let Ok(context) = attempt(&f16_flash) {
                self.record_fallback(
                    f16_flash,
                    "kv_f16",
                    format!(
                        "{} KV cache was rejected by this backend/model; using F16 KV cache",
                        runtime.profile.kv_cache.label()
                    ),
                )?;
                return Ok(context);
            }

            let mut f16_auto = f16_flash;
            f16_auto.flash_attention = FlashAttentionMode::Auto;
            if let Ok(context) = attempt(&f16_auto) {
                self.record_fallback(
                    f16_auto,
                    "kv_f16_flash_auto",
                    format!(
                        "{} KV cache and forced Flash Attention were rejected by this backend/model; using F16 KV cache with llama.cpp auto Flash Attention policy",
                        runtime.profile.kv_cache.label()
                    ),
                )?;
                return Ok(context);
            }
        }

        Err(AppError::Llm(format!(
            "Failed to create context for inference profile '{}': {requested_error}; conservative fallbacks were also rejected (GPU layer offload was not changed)",
            runtime.profile.name
        )))
    }

    fn record_fallback(
        &self,
        profile: InferenceProfile,
        fallback_code: &str,
        diagnostic: String,
    ) -> Result<(), AppError> {
        let mut runtime = self
            .inference
            .lock()
            .map_err(|_| AppError::Llm("Inference profile mutex poisoned".to_string()))?;
        runtime.profile = profile;
        runtime.fallback_code = Some(fallback_code.to_string());
        runtime.fallback = Some(match runtime.fallback.take() {
            Some(existing) => format!("{existing}; {diagnostic}"),
            None => diagnostic,
        });
        Ok(())
    }

    fn inference_runtime(&self) -> Result<InferenceRuntime, AppError> {
        self.inference
            .lock()
            .map(|runtime| runtime.clone())
            .map_err(|_| AppError::Llm("Inference profile mutex poisoned".to_string()))
    }

    /// Run blocking inference and return the full completion string.
    pub fn generate(
        &self,
        system_prompt: &str,
        user_message: &str,
        max_tokens: usize,
        temperature: f32,
    ) -> Result<String, AppError> {
        self.generate_with_session(None, system_prompt, user_message, max_tokens, temperature)
    }

    /// Like [`Self::generate`], in the context of `session` (see
    /// [`Self::generate_streaming_with_session`]).
    pub fn generate_with_session(
        &self,
        session: Option<&InferenceSession>,
        system_prompt: &str,
        user_message: &str,
        max_tokens: usize,
        temperature: f32,
    ) -> Result<String, AppError> {
        let mut result = String::new();
        self.generate_streaming_with_session(
            session,
            system_prompt,
            user_message,
            max_tokens,
            temperature,
            |token| {
                result.push_str(token);
                true
            },
        )?;
        Ok(result)
    }

    /// Run blocking inference, calling `on_token` for each piece.
    /// Return `false` from the callback to abort generation early.
    pub fn generate_streaming(
        &self,
        system_prompt: &str,
        user_message: &str,
        max_tokens: usize,
        temperature: f32,
        on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        self.generate_streaming_with_session(
            None,
            system_prompt,
            user_message,
            max_tokens,
            temperature,
            on_token,
        )
    }

    /// Like [`Self::generate_streaming`], with the KV context keyed by
    /// `session` (e.g. [`InferenceSession::one_off`]) so the next generation
    /// of the same task and patient record reuses the common prompt prefix.
    /// `None` keys it by the prompt digest: only an exact repeat reuses it.
    pub fn generate_streaming_with_session(
        &self,
        session: Option<&InferenceSession>,
        system_prompt: &str,
        user_message: &str,
        max_tokens: usize,
        temperature: f32,
        on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        let prompt = self.format_chat_history(
            system_prompt,
            &[AgentMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
        )?;
        self.generate_streaming_raw_keyed(
            session,
            &prompt,
            max_tokens,
            SamplerConfig::from_temperature(temperature),
            on_token,
        )
    }

    /// Like [`Self::generate_streaming_raw`] with an explicit clinical sampler.
    pub fn generate_streaming_raw_with_sampler(
        &self,
        prompt: &str,
        max_tokens: usize,
        sampler: SamplerConfig,
        on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        self.generate_streaming_raw_keyed(None, prompt, max_tokens, sampler, on_token)
    }

    /// Generate from a pre-formatted prompt in `session`'s context, or, with
    /// `None`, in an isolated context keyed by the prompt digest.
    fn generate_streaming_raw_keyed(
        &self,
        session: Option<&InferenceSession>,
        prompt: &str,
        max_tokens: usize,
        sampler: SamplerConfig,
        on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        let isolated;
        let session = match session {
            Some(session) => session,
            None => {
                isolated =
                    InferenceSession::isolated(format!("raw:{}", sha256_bytes(prompt.as_bytes())));
                &isolated
            }
        };
        self.generate_streaming_cached(session, prompt, max_tokens, sampler, on_token)
    }

    /// Reference cold path retained for equivalence tests and benchmarks.
    #[allow(dead_code)]
    fn generate_streaming_cold(
        &self,
        system_prompt: &str,
        user_message: &str,
        max_tokens: usize,
        temperature: f32,
        mut on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?;

        let prompt = self.format_chat_history(
            system_prompt,
            &[AgentMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
        )?;

        // 1. Tokenise
        let tokens = model.vocab().tokenize(prompt.as_bytes(), true, true);

        // 2. Context sized to the machine, with bounded prompt batches so long
        // contexts do not require an equally large temporary compute buffer.
        let mut ctx = self.create_context(model)?;
        let runtime = self.inference_runtime()?;

        // 3. Decode the prompt in bounded batches.
        let n_prompt = tokens.len();
        validate_context_budget(
            self.context_size,
            n_prompt,
            max_tokens,
            runtime.profile.completion_headroom,
        )?;
        let prompt_batch_size = ctx.n_batch() as usize;
        let mut batch = LlamaBatch::new(prompt_batch_size, 1);

        let wall_start = Instant::now();
        for (chunk_index, chunk) in tokens.chunks(prompt_batch_size).enumerate() {
            batch.clear();
            let start = chunk_index * prompt_batch_size;
            for (offset, token) in chunk.iter().enumerate() {
                let position = i32::try_from(start + offset)
                    .map_err(|_| AppError::Llm("Prompt position exceeds i32".to_string()))?;
                let needs_logits = start + offset + 1 == n_prompt;
                batch
                    .add(*token, position, &[0], needs_logits)
                    .map_err(|e| AppError::Llm(format!("Failed to build batch: {e}")))?;
            }
            ctx.decode(&mut batch)
                .map_err(|e| AppError::Llm(format!("Failed to decode prompt: {e}")))?;
        }
        let gen_phase_start = Instant::now();

        // 4. Sampler chain from the clinical harness (greedy when temperature is 0).
        let mut sampler = SamplerConfig::from_temperature(temperature).build(model.n_vocab());

        // 5. Stateful UTF-8 decoder for multi-byte tokens
        let mut utf8_dec = UTF_8.new_decoder();
        let decode_special = self.chat_family.decodes_special_tokens();
        let mut normalizer = self.chat_family.normalizer();

        let mut first_token = true;
        let mut ttft_ms = 0.0f64;
        let mut completion_tokens = 0usize;

        for (n_cur, _) in (n_prompt as i32..).zip(0..max_tokens) {
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);

            if model.vocab().is_eog(token) {
                break;
            }

            let raw_piece = decode_piece(model, token, &mut utf8_dec, decode_special);
            let piece = normalizer.push(&raw_piece);

            if first_token {
                ttft_ms = wall_start.elapsed().as_secs_f64() * 1000.0;
                first_token = false;
            }

            if !piece.is_empty() && !on_token(&piece) {
                break;
            }

            completion_tokens += 1;

            // Advance context with the new token
            if n_cur + 1 >= self.context_size as i32 {
                break;
            }
            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .map_err(|e| AppError::Llm(format!("Failed to add token: {e}")))?;
            ctx.decode(&mut batch)
                .map_err(|e| AppError::Llm(format!("Failed to decode token: {e}")))?;
        }

        let tail = normalizer.finish();
        if !tail.is_empty() {
            on_token(&tail);
        }

        if completion_tokens > 0 {
            let gen_elapsed = gen_phase_start.elapsed().as_secs_f64();
            let tps = if gen_elapsed > 0.0 {
                completion_tokens as f64 / gen_elapsed
            } else {
                0.0
            };
            if let Ok(mut stats) = self.last_stats.lock() {
                *stats = Some(GenerationStats {
                    ttft_ms,
                    tps,
                    completion_tokens,
                    prompt_tokens: n_prompt,
                    evaluated_prompt_tokens: n_prompt,
                    reused_prompt_tokens: 0,
                    cache_hit: false,
                    prefill_ms: gen_phase_start.duration_since(wall_start).as_secs_f64() * 1000.0,
                    estimated_prefill_saved_ms: 0.0,
                    total_latency_ms: wall_start.elapsed().as_secs_f64() * 1000.0,
                    peak_rss_bytes: peak_rss_bytes(),
                    session_kind: SessionKind::OneOff,
                });
            }
        }

        Ok(())
    }

    pub fn status(&self) -> EngineStatus {
        EngineStatus {
            is_loaded: self.model.is_some(),
            model_name: self.model.as_ref().map(|_| self.model_name.clone()),
            model_path: self
                .model
                .as_ref()
                .map(|_| self.model_path.to_string_lossy().into_owned()),
            total_ram_bytes: Self::total_ram(),
            // A loaded model is always on disk.
            is_downloaded: self.model.is_some(),
            downloaded_filename: self.model.as_ref().map(|_| self.model_name.clone()),
            last_generation_stats: self.last_stats.lock().ok().and_then(|g| g.clone()),
            inference_config: self.inference.lock().ok().map(|runtime| {
                InferenceDiagnostics::from_profile(
                    &runtime.profile,
                    runtime.fallback.clone(),
                    runtime.fallback_code.clone(),
                    runtime.memory_governor.clone(),
                )
            }),
            context_cache: self
                .contexts
                .lock()
                .map(|pool| pool.telemetry.clone())
                .unwrap_or_default(),
            desired_model: None,
            lifecycle: EngineLifecycleStatus {
                phase: EngineLifecyclePhase::Ready,
                requested_filename: None,
                active_filename: self.model.as_ref().map(|_| self.model_name.clone()),
                error: None,
            },
        }
    }

    /// Run blocking inference from a pre-formatted prompt string (full ChatML).
    /// Like `generate_streaming` but bypasses `format_chatml` so callers can
    /// pass multi-turn history they have built themselves.
    pub fn generate_streaming_raw(
        &self,
        prompt: &str,
        max_tokens: usize,
        temperature: f32,
        on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        self.generate_streaming_raw_keyed(
            None,
            prompt,
            max_tokens,
            SamplerConfig::from_temperature(temperature),
            on_token,
        )
    }

    /// Generate using a leased persistent context for a logical conversation.
    pub fn generate_streaming_session(
        &self,
        session: &InferenceSession,
        prompt: &str,
        max_tokens: usize,
        temperature: f32,
        on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        self.generate_streaming_session_with_sampler(
            session,
            prompt,
            max_tokens,
            SamplerConfig::from_temperature(temperature),
            on_token,
        )
    }

    /// The KV context is keyed by session, not by system prompt: generations
    /// with different system prompts in one conversation (agent probe and
    /// final answer) share one context and reuse their common token prefix.
    /// `reusable_prefix` already limits reuse to identical tokens.
    pub fn generate_streaming_session_with_sampler(
        &self,
        session: &InferenceSession,
        prompt: &str,
        max_tokens: usize,
        sampler: SamplerConfig,
        on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        self.generate_streaming_cached(session, prompt, max_tokens, sampler, on_token)
    }

    fn context_key(&self, session: &InferenceSession, runtime: &InferenceRuntime) -> ContextKey {
        ContextKey {
            model_hash: self.model_hash.clone(),
            chat_template_hash: self.chat_template_hash.clone(),
            prompt_version: session.prompt_version.clone(),
            adapter_hash: session.adapter_hash.clone(),
            context_size: runtime.profile.n_ctx,
            batch_size: runtime.profile.n_batch,
            kv_config_hash: format!(
                "{}:{}:{}",
                runtime.profile.kv_cache.label(),
                runtime.profile.n_ubatch,
                runtime.profile.flash_attention.label()
            ),
            conversation_id: session.conversation_id.clone(),
            patient_id: session.patient_id.clone(),
            patient_revision: session.patient_revision.clone(),
            kind: session.kind,
        }
    }

    /// Continue an already-admitted generation (e.g. after the think budget
    /// force-closed reasoning) in the same leased context.
    ///
    /// Phase 1 reserved the completion headroom; the continuation prompt is
    /// phase 1's prompt plus what it generated, so it must not reserve the full
    /// headroom again. `max_tokens` is clamped to the space actually left, and
    /// the call fails only if no room remains at all.
    pub fn continue_streaming_session_with_sampler(
        &self,
        session: &InferenceSession,
        prompt: &str,
        max_tokens: usize,
        sampler: SamplerConfig,
        on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        let prompt_tokens = self.count_tokens(prompt);
        let max_tokens = continuation_budget(self.context_size, prompt_tokens, max_tokens)?;
        self.generate_streaming_cached_with_headroom(
            session,
            prompt,
            max_tokens,
            sampler,
            Some(max_tokens),
            on_token,
        )
    }

    fn generate_streaming_cached(
        &self,
        session: &InferenceSession,
        prompt: &str,
        max_tokens: usize,
        sampler_config: SamplerConfig,
        on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        self.generate_streaming_cached_with_headroom(
            session,
            prompt,
            max_tokens,
            sampler_config,
            None,
            on_token,
        )
    }

    /// `completion_headroom` overrides the profile's reservation; `None` keeps it.
    #[allow(clippy::too_many_arguments)]
    fn generate_streaming_cached_with_headroom(
        &self,
        session: &InferenceSession,
        prompt: &str,
        max_tokens: usize,
        sampler_config: SamplerConfig,
        completion_headroom: Option<usize>,
        mut on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?;
        let tokens = model.vocab().tokenize(prompt.as_bytes(), true, true);
        let runtime = self.inference_runtime()?;
        validate_context_budget(
            self.context_size,
            tokens.len(),
            max_tokens,
            completion_headroom.unwrap_or(runtime.profile.completion_headroom),
        )?;
        let mut key = self.context_key(session, &runtime);

        // Holding this guard is the context lease. It serializes inference and
        // context creation, bounding transient model/KV memory during swaps.
        let mut lease = self
            .contexts
            .lock()
            .map_err(|_| AppError::Llm("Inference context pool mutex poisoned".to_string()))?;
        let pool = &mut *lease;
        pool.clock = pool.clock.wrapping_add(1);
        let use_clock = pool.clock;

        let plan = {
            let resident: Vec<(&ContextKey, u64)> = pool
                .entries
                .iter()
                .map(|entry| (&entry.key, entry.last_used))
                .collect();
            plan_slot(&resident, &key, pool.telemetry.max_contexts)
        };

        for &evicted in &plan.evict {
            count_departure(&mut pool.telemetry, &pool.entries[evicted].key, &key);
        }
        for &evicted in plan.evict.iter().rev() {
            pool.entries.remove(evicted);
        }
        let shifted = |index: usize| index - plan.evict.iter().filter(|&&e| e < index).count();

        let index = match plan.action {
            SlotAction::Hit(index) => shifted(index),
            SlotAction::Reuse { index, keep_prefix } => {
                // Same runtime profile: re-key the allocation instead of
                // freeing it and allocating a new KV cache and compute buffers.
                let index = shifted(index);
                let entry = &mut pool.entries[index];
                count_departure(&mut pool.telemetry, &entry.key, &key);
                if !keep_prefix {
                    entry.context.clear_kv_cache();
                    entry.tokens.clear();
                    entry.checkpoints.clear();
                }
                entry.key = key;
                pool.telemetry.context_allocations_reused += 1;
                index
            }
            SlotAction::Create => {
                let context = self.create_context(model)?;
                // SAFETY: the model is boxed, so its address remains stable. The
                // context pool is declared before `model`, guaranteeing contexts
                // are dropped first, and access is serialized by the pool mutex.
                let context = unsafe {
                    std::mem::transmute::<LlamaContext<'_>, LlamaContext<'static>>(context)
                };
                pool.telemetry.contexts_created += 1;
                // Context creation may have selected a safe fallback profile.
                let runtime = self.inference_runtime()?;
                key = self.context_key(session, &runtime);
                // After a fallback no future key can match an older entry's
                // KV layout, and the pool size is re-derived for the new one.
                let resident_before = pool.entries.len();
                pool.entries
                    .retain(|entry| entry.key.same_runtime_profile(&key));
                pool.telemetry.evictions += (resident_before - pool.entries.len()) as u64;
                pool.telemetry.max_contexts = self.pool_capacity(&runtime.profile);
                while pool.entries.len() >= pool.telemetry.max_contexts.max(1) {
                    let Some((oldest, _)) = pool
                        .entries
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, entry)| entry.last_used)
                    else {
                        break;
                    };
                    pool.entries.remove(oldest);
                    pool.telemetry.evictions += 1;
                }
                pool.entries.push(CachedContext {
                    context,
                    tokens: Vec::new(),
                    key,
                    last_used: use_clock,
                    checkpoints: Vec::new(),
                });
                pool.entries.len() - 1
            }
        };

        pool.entries[index].last_used = use_clock;
        let mut reused = reusable_prefix(&pool.entries[index].tokens, &tokens);
        if reused > 0 {
            let rollback_ok = pool.entries[index]
                .context
                .clear_kv_cache_seq(Some(0), Some(reused as u32), None)
                .map_err(|e| AppError::Llm(format!("KV-cache rollback failed: {e}")))?;
            if !rollback_ok {
                match restore_checkpoint(&mut pool.entries[index], reused) {
                    Some(restored) => {
                        pool.telemetry.checkpoint_restores += 1;
                        reused = restored;
                    }
                    None => {
                        pool.entries[index].context.clear_kv_cache();
                        pool.telemetry.invalidations += 1;
                        reused = 0;
                    }
                }
            }
        } else if !pool.entries[index].tokens.is_empty() {
            pool.entries[index].context.clear_kv_cache();
        }
        pool.entries[index].tokens.truncate(reused);
        // Snapshots past the surviving prefix describe a different history.
        pool.entries[index]
            .checkpoints
            .retain(|checkpoint| checkpoint.n_tokens <= reused);
        // A context is only a real cache hit when at least one prompt token
        // survives validation and rollback (a re-keyed context of the same
        // patient record may keep one too). This keeps UI and telemetry
        // aligned with actual prefill work avoided.
        let cache_hit = reused > 0;
        if cache_hit {
            pool.telemetry.hits += 1;
        } else {
            pool.telemetry.misses += 1;
        }

        let wall_start = Instant::now();
        let prompt_batch_size = pool.entries[index].context.n_batch() as usize;
        let mut batch = LlamaBatch::new(prompt_batch_size, 1);
        let checkpoint_at = if self.uses_state_checkpoints {
            checkpoint_position(tokens.len(), reused, CHECKPOINT_TAIL_TOKENS)
        } else {
            None
        };
        let segments: Vec<&[LlamaToken]> = match checkpoint_at {
            Some(at) => vec![&tokens[reused..at], &tokens[at..]],
            None => vec![&tokens[reused..]],
        };
        for (segment_index, segment) in segments.iter().enumerate() {
            if segment_index == 1 {
                take_checkpoint(&mut pool.entries[index]);
                pool.telemetry.checkpoints_created += 1;
            }
            for chunk in segment.chunks(prompt_batch_size) {
                batch.clear();
                let start = pool.entries[index].tokens.len();
                for (offset, token) in chunk.iter().enumerate() {
                    let position = i32::try_from(start + offset)
                        .map_err(|_| AppError::Llm("Prompt position exceeds i32".to_string()))?;
                    let needs_logits = start + offset + 1 == tokens.len();
                    batch
                        .add(*token, position, &[0], needs_logits)
                        .map_err(|e| AppError::Llm(format!("Failed to build batch: {e}")))?;
                }
                pool.entries[index]
                    .context
                    .decode(&mut batch)
                    .map_err(|e| AppError::Llm(format!("Failed to decode prompt suffix: {e}")))?;
                pool.entries[index].tokens.extend_from_slice(chunk);
            }
        }
        let prefill_ms = wall_start.elapsed().as_secs_f64() * 1000.0;
        let evaluated = tokens.len().saturating_sub(reused);
        let historical_ms_per_token = if pool.telemetry.evaluated_tokens == 0 {
            0.0
        } else {
            pool.evaluated_prefill_ms / pool.telemetry.evaluated_tokens as f64
        };
        let estimated_saved_ms = reused as f64 * historical_ms_per_token;
        let gen_phase_start = Instant::now();

        let mut sampler = sampler_config.build(model.n_vocab());
        let mut utf8_dec = UTF_8.new_decoder();
        let decode_special = self.chat_family.decodes_special_tokens();
        let mut normalizer = self.chat_family.normalizer();
        let mut ttft_ms = 0.0;
        let mut completion_tokens = 0usize;

        for _ in 0..max_tokens {
            let token = sampler.sample(&pool.entries[index].context, -1);
            sampler.accept(token);
            if model.vocab().is_eog(token) {
                break;
            }
            let raw_piece = decode_piece(model, token, &mut utf8_dec, decode_special);
            let piece = normalizer.push(&raw_piece);
            if completion_tokens == 0 {
                ttft_ms = wall_start.elapsed().as_secs_f64() * 1000.0;
            }
            if !piece.is_empty() && !on_token(&piece) {
                break;
            }
            let position = pool.entries[index].tokens.len();
            if position + 1 >= self.context_size {
                break;
            }
            batch.clear();
            batch
                .add(token, position as i32, &[0], true)
                .map_err(|e| AppError::Llm(format!("Failed to add token: {e}")))?;
            pool.entries[index]
                .context
                .decode(&mut batch)
                .map_err(|e| AppError::Llm(format!("Failed to decode token: {e}")))?;
            pool.entries[index].tokens.push(token);
            completion_tokens += 1;
        }

        let tail = normalizer.finish();
        if !tail.is_empty() {
            on_token(&tail);
        }

        let gen_elapsed = gen_phase_start.elapsed().as_secs_f64();
        let tps = if gen_elapsed > 0.0 {
            completion_tokens as f64 / gen_elapsed
        } else {
            0.0
        };
        pool.telemetry.reused_tokens += reused as u64;
        pool.telemetry.evaluated_tokens += evaluated as u64;
        pool.telemetry.estimated_prefill_saved_ms += estimated_saved_ms;
        pool.evaluated_prefill_ms += prefill_ms;
        pool.telemetry.resident_contexts = pool.entries.len();
        log::info!(
            "LLM context cache: hit={cache_hit}, reused={reused}, evaluated={evaluated}, prompt={}",
            tokens.len()
        );

        if let Ok(mut stats) = self.last_stats.lock() {
            *stats = Some(GenerationStats {
                ttft_ms,
                tps,
                completion_tokens,
                prompt_tokens: tokens.len(),
                evaluated_prompt_tokens: evaluated,
                reused_prompt_tokens: reused,
                cache_hit,
                prefill_ms,
                estimated_prefill_saved_ms: estimated_saved_ms,
                total_latency_ms: wall_start.elapsed().as_secs_f64() * 1000.0,
                peak_rss_bytes: peak_rss_bytes(),
                session_kind: session.kind,
            });
        }
        Ok(())
    }

    /// Reference cold path retained for equivalence tests and benchmarks.
    #[allow(dead_code)]
    fn generate_streaming_raw_cold(
        &self,
        prompt: &str,
        max_tokens: usize,
        temperature: f32,
        mut on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?;

        let tokens = model.vocab().tokenize(prompt.as_bytes(), true, true);

        let mut ctx = self.create_context(model)?;
        let runtime = self.inference_runtime()?;

        let n_prompt = tokens.len();
        validate_context_budget(
            self.context_size,
            n_prompt,
            max_tokens,
            runtime.profile.completion_headroom,
        )?;
        let prompt_batch_size = ctx.n_batch() as usize;
        let mut batch = LlamaBatch::new(prompt_batch_size, 1);

        let wall_start = Instant::now();
        for (chunk_index, chunk) in tokens.chunks(prompt_batch_size).enumerate() {
            batch.clear();
            let start = chunk_index * prompt_batch_size;
            for (offset, token) in chunk.iter().enumerate() {
                let position = i32::try_from(start + offset)
                    .map_err(|_| AppError::Llm("Prompt position exceeds i32".to_string()))?;
                let needs_logits = start + offset + 1 == n_prompt;
                batch
                    .add(*token, position, &[0], needs_logits)
                    .map_err(|e| AppError::Llm(format!("Failed to build batch: {e}")))?;
            }
            ctx.decode(&mut batch)
                .map_err(|e| AppError::Llm(format!("Failed to decode prompt: {e}")))?;
        }
        let gen_phase_start = Instant::now();

        let mut sampler = SamplerConfig::from_temperature(temperature).build(model.n_vocab());

        let mut utf8_dec = UTF_8.new_decoder();
        let decode_special = self.chat_family.decodes_special_tokens();
        let mut normalizer = self.chat_family.normalizer();

        let mut first_token = true;
        let mut ttft_ms = 0.0f64;
        let mut completion_tokens = 0usize;

        for (n_cur, _) in (n_prompt as i32..).zip(0..max_tokens) {
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);

            if model.vocab().is_eog(token) {
                break;
            }

            let raw_piece = decode_piece(model, token, &mut utf8_dec, decode_special);
            let piece = normalizer.push(&raw_piece);

            if first_token {
                ttft_ms = wall_start.elapsed().as_secs_f64() * 1000.0;
                first_token = false;
            }

            if !piece.is_empty() && !on_token(&piece) {
                break;
            }

            completion_tokens += 1;

            if n_cur + 1 >= self.context_size as i32 {
                break;
            }
            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .map_err(|e| AppError::Llm(format!("Failed to add token: {e}")))?;
            ctx.decode(&mut batch)
                .map_err(|e| AppError::Llm(format!("Failed to decode token: {e}")))?;
        }

        let tail = normalizer.finish();
        if !tail.is_empty() {
            on_token(&tail);
        }

        if completion_tokens > 0 {
            let gen_elapsed = gen_phase_start.elapsed().as_secs_f64();
            let tps = if gen_elapsed > 0.0 {
                completion_tokens as f64 / gen_elapsed
            } else {
                0.0
            };
            if let Ok(mut stats) = self.last_stats.lock() {
                *stats = Some(GenerationStats {
                    ttft_ms,
                    tps,
                    completion_tokens,
                    prompt_tokens: n_prompt,
                    evaluated_prompt_tokens: n_prompt,
                    reused_prompt_tokens: 0,
                    cache_hit: false,
                    prefill_ms: gen_phase_start.duration_since(wall_start).as_secs_f64() * 1000.0,
                    estimated_prefill_saved_ms: 0.0,
                    total_latency_ms: wall_start.elapsed().as_secs_f64() * 1000.0,
                    peak_rss_bytes: peak_rss_bytes(),
                    session_kind: SessionKind::OneOff,
                });
            }
        }

        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        self.model.is_some()
    }

    /// Returns the context window size used for all inference calls.
    pub fn context_size(&self) -> usize {
        self.context_size
    }

    /// Chat dialect detected from the loaded model's embedded template.
    pub fn chat_family(&self) -> ChatFamily {
        self.chat_family
    }

    /// Sampler family of the loaded model, for [`GenerationTask::profile_for`].
    ///
    /// [`GenerationTask::profile_for`]: super::harness::GenerationTask::profile_for
    pub fn sampler_family(&self) -> SamplerFamily {
        self.sampler_family
    }

    /// Format conversation history with the template embedded in the loaded GGUF.
    /// This is required for non-ChatML families such as gpt-oss (Harmony) and Gemma.
    pub fn format_chat_history(
        &self,
        system_prompt: &str,
        messages: &[AgentMessage],
    ) -> Result<String, AppError> {
        self.format_chat_history_with_effort(system_prompt, messages, ThinkingEffort::default())
    }

    /// Like [`Self::format_chat_history`], forwarding `effort` to families with
    /// a native reasoning switch (Gemma 4 `<|think|>`, Harmony `Reasoning:`).
    pub fn format_chat_history_with_effort(
        &self,
        system_prompt: &str,
        messages: &[AgentMessage],
        effort: ThinkingEffort,
    ) -> Result<String, AppError> {
        if let Some(prompt) = self.chat_family.format(system_prompt, messages, effort) {
            return Ok(prompt);
        }
        let mut chat = Vec::with_capacity(messages.len() + 1);
        chat.push(new_chat_message("system", system_prompt)?);
        for message in messages {
            let role = match message.role.as_str() {
                "tool_call" => "assistant",
                "tool_result" => "user",
                role => role,
            };
            chat.push(new_chat_message(role, &message.content)?);
        }
        self.model
            .as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?
            .apply_chat_template(&self.chat_template, &chat, true)
            .map_err(|e| AppError::Llm(format!("Failed to apply model chat template: {e}")))
    }

    /// Format a conversation with the model's reasoning switched off (or at
    /// its lowest level where it cannot be switched off), for workloads that
    /// need a direct answer within a small token limit, such as the
    /// deterministic runtime benchmark.
    pub fn format_chat_history_reasoning_off(
        &self,
        system_prompt: &str,
        messages: &[AgentMessage],
    ) -> Result<String, AppError> {
        let prompt =
            self.format_chat_history_with_effort(system_prompt, messages, ThinkingEffort::Low)?;
        Ok(self
            .chat_family
            .with_reasoning_off(&prompt, self.template_uses_think_tags))
    }

    /// One-off completion with reasoning switched off, for tasks that need a
    /// direct structured answer (metadata extraction). Reasoning would spend
    /// the small token limit before the answer appears, and greedy decoding
    /// inside a `<think>` block tends to loop. `session` keys the context as
    /// in [`Self::generate_streaming_with_session`].
    pub fn generate_reasoning_off(
        &self,
        system_prompt: &str,
        user_message: &str,
        max_tokens: usize,
        sampler: SamplerConfig,
        session: Option<&InferenceSession>,
    ) -> Result<String, AppError> {
        let prompt = self.format_chat_history_reasoning_off(
            system_prompt,
            &[AgentMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
        )?;
        let mut result = String::new();
        self.generate_streaming_raw_keyed(session, &prompt, max_tokens, sampler, |token| {
            result.push_str(token);
            true
        })?;
        Ok(result)
    }

    /// Returns stats from the most recent generation call, if any.
    pub fn last_generation_stats(&self) -> Option<GenerationStats> {
        self.last_stats.lock().ok().and_then(|g| g.clone())
    }

    /// Count the number of tokens in a string using the loaded model's tokenizer.
    /// Falls back to a character-based estimate (~4 chars/token) if no model is loaded.
    pub fn count_tokens(&self, text: &str) -> usize {
        let Some(model) = self.model.as_ref() else {
            return text.len() / 4;
        };
        model.vocab().tokenize(text.as_bytes(), true, true).len()
    }

    /// Return the total system RAM in bytes (macOS via sysctl).
    pub fn total_ram() -> u64 {
        #[cfg(target_os = "macos")]
        {
            unsafe {
                let mut size: u64 = 0;
                let mut len = std::mem::size_of::<u64>();
                let name = std::ffi::CString::new("hw.memsize").unwrap();
                libc::sysctlbyname(
                    name.as_ptr(),
                    &mut size as *mut u64 as *mut libc::c_void,
                    &mut len as *mut usize,
                    std::ptr::null_mut(),
                    0,
                );
                size
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            16 * 1024 * 1024 * 1024
        }
    }

    /// Choose the best model for the available RAM.
    pub fn recommended_model() -> ModelChoice {
        recommended_model_for_ram(Self::total_ram())
    }
}

fn recommended_model_for_ram(ram: u64) -> ModelChoice {
    const GB: u64 = 1024 * 1024 * 1024;

    let preferred_filename = if ram >= 32 * GB {
        "Qwen3.6-35B-A3B-UD-Q4_K_M.gguf"
    } else if ram >= 24 * GB {
        "Qwen3.8-27B-UD-Q4_K_M.gguf"
    } else if ram >= 18 * GB {
        "gpt-oss-20b-MXFP4.gguf"
    } else if ram >= 16 * GB {
        "Qwen3.5-9B-Q4_K_M.gguf"
    } else if ram >= 12 * GB {
        "gemma-4-E4B_q4_0-it.gguf"
    } else {
        "gemma-4-E2B_q4_0-it.gguf"
    };
    let entry = download::find_model(preferred_filename)
        .expect("recommended model must exist in the download catalog");
    let active_context = runtime_context_for_ram(ram).min(entry.context_window_tokens as usize);

    ModelChoice {
        name: entry.name.to_string(),
        filename: entry.filename.to_string(),
        size_bytes: entry.size_bytes,
        reason: format!(
            "Empfohlen für {} GB RAM: {}, {}K aktiver / {}K nativer Kontext, {} Modellgröße",
            entry.min_ram_gb,
            entry.parameters,
            active_context / 1024,
            entry.context_window_tokens / 1024,
            format_gib(entry.size_bytes)
        ),
    }
}

/// Completion tokens a continuation may use: the request, clamped to the room
/// left after its prompt (keeping one slot for the final decode).
fn continuation_budget(
    context_size: usize,
    prompt_tokens: usize,
    requested: usize,
) -> Result<usize, AppError> {
    let available = context_size.saturating_sub(prompt_tokens + 1);
    match requested.min(available) {
        0 => Err(AppError::Llm(format!(
            "No room left to continue: {prompt_tokens} prompt tokens fill the {context_size}-token context"
        ))),
        budget => Ok(budget),
    }
}

fn format_gib(bytes: u64) -> String {
    format!("{:.1} GiB", bytes as f64 / (1024_f64.powi(3)))
}

pub(crate) fn runtime_context_for_ram(ram: u64) -> usize {
    const GB: u64 = 1024 * 1024 * 1024;
    if ram >= 48 * GB {
        LARGE_CONTEXT_SIZE
    } else if ram >= 24 * GB {
        STANDARD_CONTEXT_SIZE
    } else {
        MIN_CONTEXT_SIZE
    }
}

fn max_persistent_contexts_for_ram(ram: u64) -> usize {
    const GB: u64 = 1024 * 1024 * 1024;
    if ram >= 48 * GB {
        2
    } else {
        1
    }
}

/// Contexts the pool may keep resident: two when the governor's estimate
/// leaves room for a second KV cache and compute buffer in the inference
/// budget (so a one-off generation does not evict the open chat), and never
/// fewer than the RAM tier allows.
fn context_pool_capacity(ram: u64, estimate: &MemoryEstimate, budget_bytes: u64) -> usize {
    let with_second_context = estimate
        .total_bytes
        .saturating_add(estimate.kv_cache_bytes)
        .saturating_add(estimate.graph_bytes);
    let governed = if with_second_context <= budget_bytes {
        2
    } else {
        1
    };
    governed.max(max_persistent_contexts_for_ram(ram))
}

fn is_selectable_metal_device(backend: &str, name: &str) -> bool {
    backend.eq_ignore_ascii_case("metal") || name.to_ascii_lowercase().starts_with("mtl")
}

pub(crate) fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(ring::digest::digest(&SHA256, bytes).as_ref())
}

fn sha256_file(path: &std::path::Path) -> Result<String, AppError> {
    let mut file = File::open(path)
        .map_err(|e| AppError::Llm(format!("Failed to hash model '{}': {e}", path.display())))?;
    let mut digest = DigestContext::new(&SHA256);
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|e| {
            AppError::Llm(format!("Failed to hash model '{}': {e}", path.display()))
        })?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex::encode(digest.finish().as_ref()))
}

fn peak_rss_bytes() -> u64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    let ok = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } == 0;
    if !ok {
        return 0;
    }
    let rss = unsafe { usage.assume_init() }.ru_maxrss.max(0) as u64;
    #[cfg(target_os = "macos")]
    {
        rss
    }
    #[cfg(not(target_os = "macos"))]
    {
        rss.saturating_mul(1024)
    }
}

/// Record a context leaving its key: a stale entry of the same conversation
/// (outdated record revision or configuration) is an invalidation, any other
/// an eviction.
fn count_departure(telemetry: &mut ContextCacheTelemetry, departed: &ContextKey, key: &ContextKey) {
    if departed.same_logical_context(key) {
        telemetry.invalidations += 1;
    } else {
        telemetry.evictions += 1;
    }
}

/// Snapshot the recurrent part of sequence 0 at the context's current length.
fn take_checkpoint(entry: &mut CachedContext) {
    match entry
        .context
        .state_seq_get(0, LlamaStateSeqFlags::PARTIAL_ONLY)
    {
        Ok(state) => {
            if entry.checkpoints.len() >= MAX_CHECKPOINTS_PER_CONTEXT {
                entry.checkpoints.remove(0);
            }
            log::debug!(
                "LLM context checkpoint at {} tokens ({} bytes)",
                entry.tokens.len(),
                state.byte_len()
            );
            entry.checkpoints.push(StateCheckpoint {
                n_tokens: entry.tokens.len(),
                state,
            });
        }
        Err(error) => log::warn!("LLM context checkpoint skipped: {error}"),
    }
}

/// Rewind sequence 0 to the newest checkpoint within `reused` shared tokens:
/// restore its recurrent state, then drop attention KV past it. Returns the
/// position resumed from, or `None` when the caller must reset the context.
fn restore_checkpoint(entry: &mut CachedContext, reused: usize) -> Option<usize> {
    let positions: Vec<usize> = entry.checkpoints.iter().map(|c| c.n_tokens).collect();
    let position = checkpoint_for_prefix(&positions, reused)?;
    let checkpoint = entry.checkpoints.iter().find(|c| c.n_tokens == position)?;
    if let Err(error) = entry.context.state_seq_set(&checkpoint.state, 0) {
        log::warn!("LLM context checkpoint restore failed: {error}");
        return None;
    }
    match entry
        .context
        .clear_kv_cache_seq(Some(0), Some(u32::try_from(position).ok()?), None)
    {
        Ok(true) => Some(position),
        _ => None,
    }
}

/// Decode one token through a stateful UTF-8 decoder so code points split
/// across tokens are emitted once complete.
///
/// `decode_to_string` only writes into spare capacity, so the buffer is sized
/// with the decoder's worst case (pending bytes plus replacement characters,
/// which expand one invalid byte to three). Anything less could silently drop
/// text instead of failing.
fn decode_piece(
    model: &LlamaModel,
    token: LlamaToken,
    decoder: &mut encoding_rs::Decoder,
    special: bool,
) -> String {
    let bytes = model.vocab().token_to_piece(token, special, None);
    decode_bytes(decoder, &bytes)
}

fn decode_bytes(decoder: &mut encoding_rs::Decoder, bytes: &[u8]) -> String {
    let capacity = decoder
        .max_utf8_buffer_length(bytes.len())
        .unwrap_or(bytes.len().saturating_mul(3).saturating_add(16));
    let mut piece = String::with_capacity(capacity);
    let (result, read, _) = decoder.decode_to_string(bytes, &mut piece, false);
    debug_assert_eq!(result, encoding_rs::CoderResult::InputEmpty);
    debug_assert_eq!(read, bytes.len());
    piece
}

fn new_chat_message(role: &str, content: &str) -> Result<LlamaChatMessage, AppError> {
    LlamaChatMessage::new(role.to_string(), content.to_string())
        .map_err(|e| AppError::Llm(format!("Invalid chat message: {e}")))
}

/// A message in an agent conversation history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_decode_keeps_split_and_invalid_bytes() {
        let mut decoder = UTF_8.new_decoder();
        // "ü" split across two tokens, then an invalid byte run.
        let mut out = decode_bytes(&mut decoder, &[0x47, 0xC3]);
        out.push_str(&decode_bytes(&mut decoder, &[0xBC, 0x74]));
        out.push_str(&decode_bytes(&mut decoder, &[0xFF, 0xFE, 0xFD, 0x21]));
        assert_eq!(out, "Güt\u{FFFD}\u{FFFD}\u{FFFD}!");
    }

    #[test]
    fn continuation_budget_clamps_instead_of_reserving_full_headroom() {
        // 12K prompt + 1K reasoning in a 16K context: the 4K headroom check
        // would reject this, but ~3K tokens of answer still fit.
        assert_eq!(continuation_budget(16_384, 13_100, 3_072).unwrap(), 3_072);
        assert_eq!(continuation_budget(16_384, 15_000, 3_072).unwrap(), 1_383);
        assert!(continuation_budget(16_384, 16_384, 3_072).is_err());
    }

    #[test]
    fn recommendations_cover_memory_tiers() {
        const GB: u64 = 1024 * 1024 * 1024;
        let cases = [
            (8, "gemma-4-E2B_q4_0-it.gguf"),
            (12, "gemma-4-E4B_q4_0-it.gguf"),
            (16, "Qwen3.5-9B-Q4_K_M.gguf"),
            (18, "gpt-oss-20b-MXFP4.gguf"),
            (24, "Qwen3.8-27B-UD-Q4_K_M.gguf"),
            (32, "Qwen3.6-35B-A3B-UD-Q4_K_M.gguf"),
        ];

        for (ram_gb, expected) in cases {
            let choice = recommended_model_for_ram(ram_gb * GB);
            assert_eq!(choice.filename, expected);
            let entry = download::find_model(&choice.filename).unwrap();
            assert!(
                entry.superseded_by.is_none(),
                "recommends superseded {expected}"
            );
            assert!(entry.min_ram_gb <= ram_gb);
        }
    }

    #[test]
    fn runtime_context_scales_with_available_memory() {
        const GB: u64 = 1024 * 1024 * 1024;
        assert_eq!(runtime_context_for_ram(16 * GB), MIN_CONTEXT_SIZE);
        assert_eq!(runtime_context_for_ram(24 * GB), STANDARD_CONTEXT_SIZE);
        assert_eq!(runtime_context_for_ram(48 * GB), LARGE_CONTEXT_SIZE);
    }

    #[test]
    fn persistent_context_count_stays_within_memory_tiers() {
        const GB: u64 = 1024 * 1024 * 1024;
        assert_eq!(max_persistent_contexts_for_ram(16 * GB), 1);
        assert_eq!(max_persistent_contexts_for_ram(32 * GB), 1);
        assert_eq!(max_persistent_contexts_for_ram(48 * GB), 2);
    }

    #[test]
    fn pool_holds_a_second_context_only_when_the_budget_covers_it() {
        const GB: u64 = 1024 * 1024 * 1024;
        let estimate = MemoryEstimate {
            weights_bytes: 6 * GB,
            kv_cache_bytes: 2 * GB,
            graph_bytes: GB,
            runtime_bytes: GB,
            total_bytes: 10 * GB,
        };
        // A second context needs another KV cache and compute buffer: 13 GiB.
        assert_eq!(context_pool_capacity(16 * GB, &estimate, 12 * GB), 1);
        assert_eq!(context_pool_capacity(32 * GB, &estimate, 13 * GB), 2);
        // The RAM tier remains a floor.
        assert_eq!(context_pool_capacity(48 * GB, &estimate, 12 * GB), 2);
    }

    #[test]
    fn recognises_llama_cpp_metal_device_labels() {
        assert!(is_selectable_metal_device("Metal", "Apple M5 Pro"));
        assert!(is_selectable_metal_device("", "MTL0"));
        assert!(!is_selectable_metal_device("MTL", ""));
        assert!(!is_selectable_metal_device("CPU", "CPU"));
    }

    /// Real-GGUF check that families with control-token reasoning (Gemma 4,
    /// gpt-oss) format, generate and normalise to canonical `<think>` output.
    /// Run with RAMDOC_BENCH_MODEL pointing at such a model.
    #[test]
    #[ignore = "requires a local GGUF model"]
    fn chat_family_formats_and_normalises_reasoning() {
        let path = PathBuf::from(
            std::env::var("RAMDOC_BENCH_MODEL").expect("RAMDOC_BENCH_MODEL is required"),
        );
        let engine = LlmEngine::load(path, "family-check".into()).unwrap();
        // Run through the product path: the think budget force-closes
        // reasoning (128 tokens at Low) and the continuation writes the
        // answer, so thinking-by-default models (Qwen) are covered too.
        for effort in [ThinkingEffort::Low, ThinkingEffort::High] {
            let mut answer = String::new();
            let collected = std::cell::RefCell::new(String::new());
            crate::llm::thinking::generate_with_think_budget_and_sampler(
                &engine,
                "Sie sind ein knapper klinischer Assistent. Antworten Sie auf Deutsch.",
                "Fasse zusammen: Schlaf verbessert, Angst reduziert.",
                crate::llm::harness::GenerationTask::Summary,
                effort,
                None,
                Some(SamplerConfig::from_temperature(0.0)),
                &|piece| collected.borrow_mut().push_str(piece),
            )
            .map(|returned| answer = returned)
            .unwrap();
            assert_eq!(
                answer,
                *collected.borrow(),
                "streamed and returned output differ"
            );
            println!(
                "family={:?} effort={effort:?}\n{answer}\n---",
                engine.chat_family()
            );
            for leaked in [
                "<|channel",
                "channel|>",
                "<|message|>",
                "<|start|>",
                "<turn|>",
            ] {
                assert!(!answer.contains(leaked), "native marker {leaked} leaked");
            }
            let visible = crate::llm::harness::strip_think_blocks(&answer);
            assert!(!visible.trim().is_empty(), "no visible answer");
            assert!(!visible.trim_start().starts_with("thought"));
            assert!(!visible.trim_start().starts_with("analysis"));
        }
    }

    /// Real-GGUF check of prefix reuse on a long, agent-shaped conversation:
    /// turn 2 repeats turn 1's prompt plus an answer and a new question. On
    /// hybrid models this exercises the recurrent-state checkpoints.
    #[test]
    #[ignore = "requires a local GGUF model"]
    fn long_agent_prefix_reuse() {
        let path = PathBuf::from(
            std::env::var("RAMDOC_BENCH_MODEL").expect("RAMDOC_BENCH_MODEL is required"),
        );
        let engine = LlmEngine::load(path, "prefix-check".into()).unwrap();
        let record = "Sitzung: Schlaf weiterhin gestört, Grübeln am Abend, Sertralin 100 mg 1-0-0 gut vertragen. "
            .repeat(220);
        let system = "Sie sind ein knapper klinischer Assistent.";
        let turn1 = vec![AgentMessage {
            role: "user".into(),
            content: format!("{record}\nFasse den Verlauf in einem Satz zusammen."),
        }];
        let session = InferenceSession::agent("prefix-check", None, None);
        let run = |messages: &[AgentMessage]| {
            let prompt = engine
                .format_chat_history_with_effort(system, messages, ThinkingEffort::Low)
                .unwrap();
            let mut out = String::new();
            let started = Instant::now();
            engine
                .generate_streaming_session_with_sampler(
                    &session,
                    &prompt,
                    48,
                    SamplerConfig::from_temperature(0.0),
                    |piece| {
                        out.push_str(piece);
                        true
                    },
                )
                .unwrap();
            (
                out,
                engine.last_generation_stats().unwrap(),
                started.elapsed(),
            )
        };
        let (answer1, first, first_wall) = run(&turn1);
        let mut turn2 = turn1.clone();
        turn2.push(AgentMessage {
            role: "assistant".into(),
            content: answer1,
        });
        turn2.push(AgentMessage {
            role: "user".into(),
            content: "Welche Dosis ist dokumentiert?".into(),
        });
        let (_, second, second_wall) = run(&turn2);
        let telemetry = engine.contexts.lock().unwrap().telemetry.clone();
        println!(
            "hybrid={} turn1: prompt={} prefill_ms={:.0} wall_ms={} | turn2: prompt={} reused={} evaluated={} prefill_ms={:.0} ttft_ms={:.0} wall_ms={} | restores={} checkpoints={}",
            engine.uses_state_checkpoints,
            first.prompt_tokens,
            first.prefill_ms,
            first_wall.as_millis(),
            second.prompt_tokens,
            second.reused_prompt_tokens,
            second.evaluated_prompt_tokens,
            second.prefill_ms,
            second.ttft_ms,
            second_wall.as_millis(),
            telemetry.checkpoint_restores,
            telemetry.checkpoints_created
        );
        assert!(second.cache_hit);
        assert!(second.reused_prompt_tokens > first.prompt_tokens / 2);
    }

    /// Real-GGUF check for issue #549: the agent probe and the final answer
    /// use different system prompts in one session. They must share one pool
    /// entry, without invalidation or a new context, and the answer must reuse
    /// the shared system-prompt prefix.
    #[test]
    #[ignore = "requires a local GGUF model"]
    fn probe_and_answer_share_one_context() {
        let path = PathBuf::from(
            std::env::var("RAMDOC_BENCH_MODEL").expect("RAMDOC_BENCH_MODEL is required"),
        );
        let engine = LlmEngine::load(path, "shared-context-check".into()).unwrap();
        let base = format!(
            "Sie sind ein klinischer Assistent.\n{}",
            "Patientendaten: Sertralin 100 mg 1-0-0, Schlafstörung, Grübeln. ".repeat(120)
        );
        let probe_system = format!("{base}\n\nGib nur einen <tool_call>-Block aus.");
        let answer_system = format!("{base}\n\nErfinden Sie keine Daten.");
        let messages = [AgentMessage {
            role: "user".into(),
            content: "Welche Dosis ist dokumentiert?".into(),
        }];
        let session = InferenceSession::agent("shared-context-check", None, None);
        let run = |system: &str| {
            let prompt = engine.format_chat_history(system, &messages).unwrap();
            engine
                .generate_streaming_session_with_sampler(
                    &session,
                    &prompt,
                    8,
                    SamplerConfig::from_temperature(0.0),
                    |_| true,
                )
                .unwrap();
            engine.last_generation_stats().unwrap()
        };
        run(&probe_system);
        let before = engine.contexts.lock().unwrap().telemetry.clone();
        let answer = run(&answer_system);
        let after = engine.contexts.lock().unwrap().telemetry.clone();

        assert_eq!(after.resident_contexts, 1);
        // Hybrid/recurrent models cannot roll back to an arbitrary prefix and
        // only resume from a checkpoint near the probe prompt's end, so the
        // shared prefix is not reusable there yet.
        if !engine.uses_state_checkpoints {
            assert_eq!(after.invalidations, before.invalidations);
            assert!(answer.cache_hit);
            assert!(answer.reused_prompt_tokens >= engine.count_tokens(&base) / 2);
        }
    }

    /// Real-GGUF check for issue #548: one-off generations between two chat
    /// turns never allocate a context for an unchanged profile. With a
    /// one-context pool they re-key the chat's allocation; with two they
    /// take the second slot and the chat's next turn is still a cache hit.
    #[test]
    #[ignore = "requires a local GGUF model"]
    fn one_off_between_chat_turns_reuses_the_allocation() {
        let path = PathBuf::from(
            std::env::var("RAMDOC_BENCH_MODEL").expect("RAMDOC_BENCH_MODEL is required"),
        );
        let engine = LlmEngine::load(path, "reuse-check".into()).unwrap();
        let system = format!(
            "Sie sind ein klinischer Assistent.\n{}",
            "Patientendaten: Sertralin 100 mg 1-0-0, Schlafstörung, Grübeln. ".repeat(60)
        );
        let prompt = engine
            .format_chat_history(
                &system,
                &[AgentMessage {
                    role: "user".into(),
                    content: "Welche Dosis ist dokumentiert?".into(),
                }],
            )
            .unwrap();
        let session = InferenceSession::agent(
            "reuse-check",
            Some("patient-a".into()),
            Some("revision-1".into()),
        );
        let chat_turn = || {
            engine
                .generate_streaming_session_with_sampler(
                    &session,
                    &prompt,
                    8,
                    SamplerConfig::from_temperature(0.0),
                    |_| true,
                )
                .unwrap();
            engine.last_generation_stats().unwrap()
        };

        chat_turn();
        for report in ["Bericht A", "Bericht B"] {
            engine
                .generate("Fasse kurz zusammen.", report, 8, 0.0)
                .unwrap();
            assert_eq!(
                engine.last_generation_stats().unwrap().session_kind,
                SessionKind::OneOff
            );
        }
        let second_turn = chat_turn();
        let telemetry = engine.contexts.lock().unwrap().telemetry.clone();
        println!(
            "{}",
            serde_json::json!({ "telemetry": telemetry, "turn": second_turn })
        );

        assert_eq!(second_turn.session_kind, SessionKind::Conversation);
        if telemetry.max_contexts >= 2 {
            // Chat and one-off each hold a slot; the second one-off re-keys
            // the first one's slot instead of the chat's.
            assert_eq!(telemetry.contexts_created, 2);
            assert_eq!(telemetry.context_allocations_reused, 1);
            assert!(second_turn.cache_hit);
        } else {
            assert_eq!(telemetry.contexts_created, 1);
            assert_eq!(telemetry.context_allocations_reused, 3);
        }
    }

    /// Real-GGUF check for issue #550: one-off generations are keyed by task
    /// and patient record instead of the prompt digest. A second metadata
    /// extraction reuses the system prompt and instruction before the
    /// document; a same-patient report with another instruction reuses the
    /// system prompt; another patient's report reuses nothing.
    #[test]
    #[ignore = "requires a local GGUF model"]
    fn task_keyed_one_offs_reuse_their_prefix() {
        use crate::llm::context_cache::{OneOffTask, PatientRevision};

        let path = PathBuf::from(
            std::env::var("RAMDOC_BENCH_MODEL").expect("RAMDOC_BENCH_MODEL is required"),
        );
        let engine = LlmEngine::load(path, "task-key-check".into()).unwrap();

        let mut extractions = Vec::new();
        for document in [
            "Arztbrief vom 3. März: Depressive Episode, Sertralin 100 mg.",
            "Laborbefund vom 9. April: TSH im Normbereich, Lithiumspiegel 0.7 mmol/l.",
        ] {
            // Only the cache statistics matter here, not whether the small
            // output budget yields parseable JSON.
            let _ = super::super::extract::extract_metadata(&engine, document);
            extractions.push(engine.last_generation_stats().unwrap());
        }

        let system = format!(
            "Sie sind ein klinischer Assistent.\n{}",
            "Patientendaten: Sertralin 100 mg 1-0-0, Schlafstörung, Grübeln. ".repeat(60)
        );
        let report = |patient: &str, instruction: &str| {
            let record = PatientRevision {
                patient_id: patient.into(),
                revision: "p1:revision".into(),
            };
            let session = InferenceSession::one_off(OneOffTask::Report, Some(&record));
            engine
                .generate_with_session(Some(&session), &system, instruction, 8, 0.0)
                .unwrap();
            engine.last_generation_stats().unwrap()
        };
        report("patient-a", "Schreibe einen Befundbericht.");
        let regenerated = report("patient-a", "Schreibe einen Verlaufsbericht.");
        let other_patient = report("patient-b", "Schreibe einen Verlaufsbericht.");
        println!(
            "{}",
            serde_json::json!({
                "second_extraction": extractions[1],
                "regenerated": regenerated,
                "other_patient": other_patient,
            })
        );

        assert!(!other_patient.cache_hit);
        assert_eq!(other_patient.reused_prompt_tokens, 0);
        // Hybrid/recurrent models resume only from a checkpoint near the
        // previous prompt's end, which lies past the shared prefix here.
        if !engine.uses_state_checkpoints {
            assert!(extractions[1].cache_hit);
            assert!(extractions[1].reused_prompt_tokens > 0);
            assert!(regenerated.cache_hit);
            assert!(regenerated.reused_prompt_tokens >= engine.count_tokens(&system) / 2);
        }
    }

    /// Hardware benchmark harness for issue #400. It is ignored because CI has
    /// no approved GGUF; run it on a target Mac with RAMDOC_BENCH_MODEL set.
    #[test]
    #[ignore = "requires a local GGUF model"]
    fn benchmark_cold_and_warm_contexts() {
        let path = std::env::var("RAMDOC_BENCH_MODEL").expect("RAMDOC_BENCH_MODEL is required");
        let path = PathBuf::from(path);
        let model_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("benchmark.gguf")
            .to_string();
        let engine = LlmEngine::load(path, model_name).unwrap();
        let prompt = engine
            .format_chat_history(
                "You are a concise clinical assistant.",
                &[AgentMessage {
                    role: "user".into(),
                    content: "Summarize: sleep improved and anxiety decreased.".into(),
                }],
            )
            .unwrap();

        let mut cold_answer = String::new();
        engine
            .generate_streaming_raw_cold(&prompt, 64, 0.0, |piece| {
                cold_answer.push_str(piece);
                true
            })
            .unwrap();
        let cold = engine.last_generation_stats().unwrap();

        let session = InferenceSession::agent(
            "benchmark-session",
            Some("patient-a".into()),
            Some("revision-1".into()),
        );
        let mut first_answer = String::new();
        engine
            .generate_streaming_session(&session, &prompt, 64, 0.0, |piece| {
                first_answer.push_str(piece);
                true
            })
            .unwrap();
        let mut warm_answer = String::new();
        engine
            .generate_streaming_session(&session, &prompt, 64, 0.0, |piece| {
                warm_answer.push_str(piece);
                true
            })
            .unwrap();
        let warm = engine.last_generation_stats().unwrap();

        assert_eq!(cold_answer, first_answer);
        assert_eq!(first_answer, warm_answer);
        assert!(warm.cache_hit);
        assert!(warm.reused_prompt_tokens > 0);
        println!("{}", serde_json::json!({"cold": cold, "warm": warm}));
    }
}
