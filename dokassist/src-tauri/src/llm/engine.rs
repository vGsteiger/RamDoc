use crate::error::AppError;
use encoding_rs::UTF_8;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::path::PathBuf;

/// Sentinel value for n_gpu_layers that offloads all layers to Metal GPU.
const ALL_GPU_LAYERS: u32 = 999;
/// Token context window size used for all inference calls.
const N_CTX: usize = 4096;

pub struct LlmEngine {
    // IMPORTANT: field declaration order controls drop order in Rust.
    // `model` must be dropped before `backend` — the LlamaModel holds a
    // raw pointer into the LlamaBackend, so freeing the backend first
    // causes a use-after-free crash in the llama.cpp C code at shutdown.
    model: Option<LlamaModel>,
    model_path: PathBuf,
    model_name: String,
    backend: LlamaBackend,
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
}

impl LlmEngine {
    /// Load a GGUF model from disk, offloading all layers to Metal.
    pub fn load(model_path: PathBuf, model_name: String) -> Result<Self, AppError> {
        let backend = LlamaBackend::init()
            .map_err(|e| AppError::Llm(format!("Failed to init llama backend: {e}")))?;

        // HIGH-3: Disable memory-mapped I/O so that a crafted GGUF file cannot
        // trigger memory-mapped reads of out-of-bounds data before the C library
        // has validated the tensor layout.  The SHA-256 pre-download check (CRIT-3)
        // already ensures model integrity; this is an additional layer of defence.
        let model_params = LlamaModelParams::default().with_n_gpu_layers(ALL_GPU_LAYERS);

        let model = LlamaModel::load_from_file(&backend, &model_path, &model_params)
            .map_err(|e| AppError::Llm(format!("Failed to load model: {e}")))?;

        Ok(Self {
            backend,
            model: Some(model),
            model_path,
            model_name,
        })
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
        mut on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?;

        let prompt = format_chatml(system_prompt, user_message);

        // 1. Tokenise
        let tokens = model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| AppError::Llm(format!("Tokenization failed: {e}")))?;

        // 2. Context (4 096-token window)
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(N_CTX as u32))
            .with_n_batch(N_CTX as u32);
        let mut ctx = model
            .new_context(&self.backend, ctx_params)
            .map_err(|e| AppError::Llm(format!("Failed to create context: {e}")))?;

        // 3. Decode prompt in one batch
        let n_prompt = tokens.len();
        if n_prompt >= N_CTX {
            return Err(AppError::Llm(format!(
                "Prompt too long ({n_prompt} tokens), exceeds context window ({N_CTX})"
            )));
        }
        let mut batch = LlamaBatch::new(N_CTX, 1);
        batch
            .add_sequence(&tokens, 0, false)
            .map_err(|e| AppError::Llm(format!("Failed to build batch: {e}")))?;
        ctx.decode(&mut batch)
            .map_err(|e| AppError::Llm(format!("Failed to decode prompt: {e}")))?;

        // 4. Sampler chain: temp → top-k → top-p → dist (terminal)
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(temperature),
            LlamaSampler::top_k(40),
            LlamaSampler::top_p(0.9, 1),
            LlamaSampler::dist(0),
        ]);

        // 5. Stateful UTF-8 decoder for multi-byte tokens
        let mut utf8_dec = UTF_8.new_decoder();

        for (n_cur, _) in (n_prompt as i32..).zip(0..max_tokens) {
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);

            if model.is_eog_token(token) {
                break;
            }

            let piece = model
                .token_to_piece(token, &mut utf8_dec, false, None)
                .map_err(|e| AppError::Llm(format!("Token decode failed: {e}")))?;

            if !on_token(&piece) {
                break;
            }

            // Advance context with the new token
            if n_cur + 1 >= N_CTX as i32 {
                break;
            }
            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .map_err(|e| AppError::Llm(format!("Failed to add token: {e}")))?;
            ctx.decode(&mut batch)
                .map_err(|e| AppError::Llm(format!("Failed to decode token: {e}")))?;
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
        mut on_token: impl FnMut(&str) -> bool,
    ) -> Result<(), AppError> {
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?;

        let tokens = model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| AppError::Llm(format!("Tokenization failed: {e}")))?;

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(N_CTX as u32))
            .with_n_batch(N_CTX as u32);
        let mut ctx = model
            .new_context(&self.backend, ctx_params)
            .map_err(|e| AppError::Llm(format!("Failed to create context: {e}")))?;

        let n_prompt = tokens.len();
        if n_prompt >= N_CTX {
            return Err(AppError::Llm(format!(
                "Prompt too long ({n_prompt} tokens), exceeds context window ({N_CTX})"
            )));
        }
        let mut batch = LlamaBatch::new(N_CTX, 1);
        batch
            .add_sequence(&tokens, 0, false)
            .map_err(|e| AppError::Llm(format!("Failed to build batch: {e}")))?;
        ctx.decode(&mut batch)
            .map_err(|e| AppError::Llm(format!("Failed to decode prompt: {e}")))?;

        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(temperature),
            LlamaSampler::top_k(40),
            LlamaSampler::top_p(0.9, 1),
            LlamaSampler::dist(0),
        ]);

        let mut utf8_dec = UTF_8.new_decoder();

        for (n_cur, _) in (n_prompt as i32..).zip(0..max_tokens) {
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);

            if model.is_eog_token(token) {
                break;
            }

            let piece = model
                .token_to_piece(token, &mut utf8_dec, false, None)
                .map_err(|e| AppError::Llm(format!("Token decode failed: {e}")))?;

            if !on_token(&piece) {
                break;
            }

            if n_cur + 1 >= N_CTX as i32 {
                break;
            }
            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .map_err(|e| AppError::Llm(format!("Failed to add token: {e}")))?;
            ctx.decode(&mut batch)
                .map_err(|e| AppError::Llm(format!("Failed to decode token: {e}")))?;
        }
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        self.model.is_some()
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
        let ram = Self::total_ram();
        const GB: u64 = 1024 * 1024 * 1024;

        if ram >= 24 * GB {
            ModelChoice {
                name: "Qwen3-30B-A3B MoE Q4_K_M".to_string(),
                filename: "Qwen3-30B-A3B-Q4_K_M.gguf".to_string(),
                size_bytes: 18 * GB,
                reason: "24 GB+ RAM: Qwen3 30B MoE für beste Qualität".to_string(),
            }
        } else if ram >= 16 * GB {
            ModelChoice {
                name: "Qwen3-8B Q4_K_M".to_string(),
                filename: "Qwen3-8B-Q4_K_M.gguf".to_string(),
                size_bytes: 5 * GB,
                reason: "16–24 GB RAM: Qwen3 8B für gute Qualität".to_string(),
            }
        } else {
            ModelChoice {
                name: "Phi-4 Mini Q4_K_M".to_string(),
                filename: "Phi-4-mini-instruct-Q4_K_M.gguf".to_string(),
                size_bytes: 3 * GB,
                reason: "Unter 16 GB RAM: Phi-4 Mini für minimale Ressourcen".to_string(),
            }
        }
    }
}

fn format_chatml(system_prompt: &str, user_message: &str) -> String {
    format!(
        "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
        system_prompt, user_message,
    )
}

/// A message in an agent conversation history.
#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
}

/// Format a multi-turn conversation as a ChatML prompt.
/// `role` values: "user", "assistant", "tool_call", "tool_result"
/// Tool call/result messages are rendered as assistant/user turns.
pub fn format_chatml_history(system_prompt: &str, messages: &[AgentMessage]) -> String {
    let mut out = format!("<|im_start|>system\n{}<|im_end|>\n", system_prompt);
    for msg in messages {
        let chatml_role = match msg.role.as_str() {
            "tool_call" => "assistant",
            "tool_result" => "user",
            other => other,
        };
        out.push_str(&format!(
            "<|im_start|>{}\n{}<|im_end|>\n",
            chatml_role, msg.content
        ));
    }
    out.push_str("<|im_start|>assistant\n");
    out
}
