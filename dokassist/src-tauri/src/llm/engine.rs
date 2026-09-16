use super::context_cache::{reusable_prefix, ContextCacheTelemetry, ContextKey, InferenceSession};
use super::download;
use super::inference::{
    validate_context_budget, FlashAttentionMode, InferenceDiagnostics, InferenceProfile,
    KvCacheQuantization,
};
use super::memory_governor::{MemoryGovernor, MemoryGovernorDiagnostics};
use crate::error::AppError;
use encoding_rs::UTF_8;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::context::LlamaContext;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
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
}

struct CachedContext {
    context: LlamaContext<'static>,
    tokens: Vec<LlamaToken>,
    key: ContextKey,
    last_used: u64,
}

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
    context_size: usize,
    inference: Mutex<InferenceRuntime>,
    model_hash: String,
    chat_template_hash: String,
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
            let metal_devices: Vec<usize> = llama_cpp_2::list_llama_ggml_backend_devices()
                .into_iter()
                .filter(|device| device.backend.eq_ignore_ascii_case("metal"))
                .map(|device| device.index)
                .collect();
            if metal_devices.is_empty() {
                return Err(AppError::Llm(
                    "Metal inference was requested, but llama.cpp reported no Metal device; refusing a silent CPU downgrade"
                        .to_string(),
                ));
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

        Ok(Self {
            contexts: Mutex::new(ContextPool::new(max_persistent_contexts_for_ram(ram))),
            backend,
            model: Some(model),
            model_path,
            model_name,
            chat_template,
            context_size,
            inference: Mutex::new(InferenceRuntime {
                profile,
                fallback,
                fallback_code,
                memory_governor: Some(governor_diagnostics),
            }),
            model_hash,
            chat_template_hash,
            last_stats: Mutex::new(None),
        })
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
        let mut result = String::new();
        self.generate_streaming(
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
        let prompt = self.format_chat_history(
            system_prompt,
            &[AgentMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
        )?;
        self.generate_streaming_raw(&prompt, max_tokens, temperature, on_token)
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
        let tokens = model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| AppError::Llm(format!("Tokenization failed: {e}")))?;

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

        // 4. Sampler chain: temp → top-k → top-p → dist (terminal)
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(temperature),
            LlamaSampler::top_k(40),
            LlamaSampler::top_p(0.9, 1),
            LlamaSampler::dist(0),
        ]);

        // 5. Stateful UTF-8 decoder for multi-byte tokens
        let mut utf8_dec = UTF_8.new_decoder();

        let mut first_token = true;
        let mut ttft_ms = 0.0f64;
        let mut completion_tokens = 0usize;

        for (n_cur, _) in (n_prompt as i32..).zip(0..max_tokens) {
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);

            if model.is_eog_token(token) {
                break;
            }

            let piece = model
                .token_to_piece(token, &mut utf8_dec, false, None)
                .map_err(|e| AppError::Llm(format!("Token decode failed: {e}")))?;

            if first_token {
                ttft_ms = wall_start.elapsed().as_secs_f64() * 1000.0;
                first_token = false;
            }

            if !on_token(&piece) {
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
        let digest = sha256_bytes(prompt.as_bytes());
        self.generate_streaming_cached(
            &InferenceSession::isolated(format!("raw:{digest}")),
            &digest,
            prompt,
            max_tokens,
            temperature,
            on_token,
        )
    }

    /// Generate using a leased persistent context for a logical conversation.
    pub fn generate_streaming_session(
        &self,
        session: &InferenceSession,
        system_prompt: &str,
        prompt: &str,
        max_tokens: usize,
        temperature: f32,
        on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        self.generate_streaming_cached(
            session,
            &sha256_bytes(system_prompt.as_bytes()),
            prompt,
            max_tokens,
            temperature,
            on_token,
        )
    }

    fn context_key(
        &self,
        session: &InferenceSession,
        system_prompt_hash: &str,
        runtime: &InferenceRuntime,
    ) -> ContextKey {
        ContextKey {
            model_hash: self.model_hash.clone(),
            chat_template_hash: self.chat_template_hash.clone(),
            system_prompt_hash: system_prompt_hash.to_string(),
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
        }
    }

    fn generate_streaming_cached(
        &self,
        session: &InferenceSession,
        system_prompt_hash: &str,
        prompt: &str,
        max_tokens: usize,
        temperature: f32,
        mut on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?;
        let tokens = model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| AppError::Llm(format!("Tokenization failed: {e}")))?;
        let runtime = self.inference_runtime()?;
        validate_context_budget(
            self.context_size,
            tokens.len(),
            max_tokens,
            runtime.profile.completion_headroom,
        )?;
        let mut key = self.context_key(session, system_prompt_hash, &runtime);

        // Holding this guard is the context lease. It serializes inference and
        // context creation, bounding transient model/KV memory during swaps.
        let mut pool = self
            .contexts
            .lock()
            .map_err(|_| AppError::Llm("Inference context pool mutex poisoned".to_string()))?;
        pool.clock = pool.clock.wrapping_add(1);
        let use_clock = pool.clock;

        let stale = pool
            .entries
            .iter()
            .filter(|entry| entry.key.same_logical_context(&key) && entry.key != key)
            .count();
        if stale > 0 {
            pool.entries
                .retain(|entry| !entry.key.same_logical_context(&key) || entry.key == key);
            pool.telemetry.invalidations += stale as u64;
        }

        let entry_found = pool.entries.iter().any(|entry| entry.key == key);
        let index = if let Some(index) = pool.entries.iter().position(|entry| entry.key == key) {
            index
        } else {
            while pool.entries.len() >= pool.telemetry.max_contexts.max(1) {
                if let Some((oldest, _)) = pool
                    .entries
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, entry)| entry.last_used)
                {
                    pool.entries.remove(oldest);
                    pool.telemetry.evictions += 1;
                }
            }

            let context = self.create_context(model)?;
            // SAFETY: the model is boxed, so its address remains stable. The
            // context pool is declared before `model`, guaranteeing contexts
            // are dropped first, and access is serialized by the pool mutex.
            let context =
                unsafe { std::mem::transmute::<LlamaContext<'_>, LlamaContext<'static>>(context) };
            // Context creation may have selected a safe fallback profile.
            key = self.context_key(session, system_prompt_hash, &self.inference_runtime()?);
            pool.entries.push(CachedContext {
                context,
                tokens: Vec::new(),
                key,
                last_used: use_clock,
            });
            pool.entries.len() - 1
        };

        pool.entries[index].last_used = use_clock;
        let mut reused = reusable_prefix(&pool.entries[index].tokens, &tokens);
        if reused > 0 {
            let rollback_ok = pool.entries[index]
                .context
                .clear_kv_cache_seq(Some(0), Some(reused as u32), None)
                .map_err(|e| AppError::Llm(format!("KV-cache rollback failed: {e}")))?;
            if !rollback_ok {
                pool.entries[index].context.clear_kv_cache();
                pool.telemetry.invalidations += 1;
                reused = 0;
            }
        } else if !pool.entries[index].tokens.is_empty() {
            pool.entries[index].context.clear_kv_cache();
        }
        pool.entries[index].tokens.truncate(reused);
        // A resident entry is only a real cache hit when at least one prompt
        // token survives validation and rollback. This keeps UI and telemetry
        // aligned with actual prefill work avoided.
        let cache_hit = entry_found && reused > 0;
        if cache_hit {
            pool.telemetry.hits += 1;
        } else {
            pool.telemetry.misses += 1;
        }

        let wall_start = Instant::now();
        let prompt_batch_size = pool.entries[index].context.n_batch() as usize;
        let mut batch = LlamaBatch::new(prompt_batch_size, 1);
        for chunk in tokens[reused..].chunks(prompt_batch_size) {
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
        let prefill_ms = wall_start.elapsed().as_secs_f64() * 1000.0;
        let evaluated = tokens.len().saturating_sub(reused);
        let historical_ms_per_token = if pool.telemetry.evaluated_tokens == 0 {
            0.0
        } else {
            pool.evaluated_prefill_ms / pool.telemetry.evaluated_tokens as f64
        };
        let estimated_saved_ms = reused as f64 * historical_ms_per_token;
        let gen_phase_start = Instant::now();

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(temperature),
            LlamaSampler::top_k(40),
            LlamaSampler::top_p(0.9, 1),
            LlamaSampler::dist(0),
        ]);
        let mut utf8_dec = UTF_8.new_decoder();
        let mut ttft_ms = 0.0;
        let mut completion_tokens = 0usize;

        for _ in 0..max_tokens {
            let token = sampler.sample(&pool.entries[index].context, -1);
            sampler.accept(token);
            if model.is_eog_token(token) {
                break;
            }
            let piece = model
                .token_to_piece(token, &mut utf8_dec, false, None)
                .map_err(|e| AppError::Llm(format!("Token decode failed: {e}")))?;
            if completion_tokens == 0 {
                ttft_ms = wall_start.elapsed().as_secs_f64() * 1000.0;
            }
            if !on_token(&piece) {
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

        let tokens = model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| AppError::Llm(format!("Tokenization failed: {e}")))?;

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

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(temperature),
            LlamaSampler::top_k(40),
            LlamaSampler::top_p(0.9, 1),
            LlamaSampler::dist(0),
        ]);

        let mut utf8_dec = UTF_8.new_decoder();

        let mut first_token = true;
        let mut ttft_ms = 0.0f64;
        let mut completion_tokens = 0usize;

        for (n_cur, _) in (n_prompt as i32..).zip(0..max_tokens) {
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);

            if model.is_eog_token(token) {
                break;
            }

            let piece = model
                .token_to_piece(token, &mut utf8_dec, false, None)
                .map_err(|e| AppError::Llm(format!("Token decode failed: {e}")))?;

            if first_token {
                ttft_ms = wall_start.elapsed().as_secs_f64() * 1000.0;
                first_token = false;
            }

            if !on_token(&piece) {
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

    /// Format conversation history with the template embedded in the loaded GGUF.
    /// This is required for non-ChatML families such as gpt-oss (Harmony) and Gemma.
    pub fn format_chat_history(
        &self,
        system_prompt: &str,
        messages: &[AgentMessage],
    ) -> Result<String, AppError> {
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
        model
            .str_to_token(text, AddBos::Always)
            .map(|t| t.len())
            .unwrap_or(text.len() / 4)
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
        "Qwen3.6-27B-Q4_K_M.gguf"
    } else if ram >= 18 * GB {
        "gpt-oss-20b-MXFP4.gguf"
    } else if ram >= 16 * GB {
        "Qwen3-8B-Q4_K_M.gguf"
    } else if ram >= 12 * GB {
        "gemma-4-E4B-it-Q4_0.gguf"
    } else {
        "gemma-4-E2B-it-Q4_0.gguf"
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

fn sha256_bytes(bytes: &[u8]) -> String {
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

fn new_chat_message(role: &str, content: &str) -> Result<LlamaChatMessage, AppError> {
    LlamaChatMessage::new(role.to_string(), content.to_string())
        .map_err(|e| AppError::Llm(format!("Invalid chat message: {e}")))
}

/// A message in an agent conversation history.
#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommendations_cover_memory_tiers() {
        const GB: u64 = 1024 * 1024 * 1024;
        let cases = [
            (8, "gemma-4-E2B-it-Q4_0.gguf"),
            (12, "gemma-4-E4B-it-Q4_0.gguf"),
            (16, "Qwen3-8B-Q4_K_M.gguf"),
            (18, "gpt-oss-20b-MXFP4.gguf"),
            (24, "Qwen3.6-27B-Q4_K_M.gguf"),
            (32, "Qwen3.6-35B-A3B-UD-Q4_K_M.gguf"),
        ];

        for (ram_gb, expected) in cases {
            let choice = recommended_model_for_ram(ram_gb * GB);
            assert_eq!(choice.filename, expected);
            assert!(download::find_model(&choice.filename).is_some());
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
            .generate_streaming_session(
                &session,
                "You are a concise clinical assistant.",
                &prompt,
                64,
                0.0,
                |piece| {
                    first_answer.push_str(piece);
                    true
                },
            )
            .unwrap();
        let mut warm_answer = String::new();
        engine
            .generate_streaming_session(
                &session,
                "You are a concise clinical assistant.",
                &prompt,
                64,
                0.0,
                |piece| {
                    warm_answer.push_str(piece);
                    true
                },
            )
            .unwrap();
        let warm = engine.last_generation_stats().unwrap();

        assert_eq!(cold_answer, first_answer);
        assert_eq!(first_answer, warm_answer);
        assert!(warm.cache_hit);
        assert!(warm.reused_prompt_tokens > 0);
        println!("{}", serde_json::json!({"cold": cold, "warm": warm}));
    }
}
