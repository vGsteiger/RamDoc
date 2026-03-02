use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use llama_cpp::{LlamaModel, LlamaParams, SessionParams};
use llama_cpp::standard_sampler::{StandardSampler, SamplerStage};
use crate::error::AppError;

/// Sentinel value for n_gpu_layers that offloads all layers to Metal GPU.
const ALL_GPU_LAYERS: i32 = 999;

pub struct LlmEngine {
    model: Option<LlamaModel>,
    model_path: PathBuf,
    model_name: String,
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
}

impl LlmEngine {
    /// Load a GGUF model from disk, offloading all layers to Metal.
    pub fn load(model_path: PathBuf, model_name: String) -> Result<Self, AppError> {
        let params = LlamaParams {
            n_gpu_layers: ALL_GPU_LAYERS,
            use_mmap: true,
            ..Default::default()
        };
        let model = LlamaModel::load_from_file(&model_path, params)
            .map_err(|e| AppError::Llm(format!("Failed to load model: {e}")))?;
        Ok(Self {
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
        self.generate_streaming(system_prompt, user_message, max_tokens, temperature, |token| {
            result.push_str(token);
            true
        })?;
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
        let model = self.model.as_ref()
            .ok_or_else(|| AppError::Llm("Model not loaded".to_string()))?;

        let prompt = format_chatml(system_prompt, user_message);

        let session_params = SessionParams {
            n_ctx: 4096,
            ..Default::default()
        };
        let mut session = model.create_session(session_params)
            .map_err(|e| AppError::Llm(format!("Failed to create session: {e}")))?;

        session.advance_context(&prompt)
            .map_err(|e| AppError::Llm(format!("Failed to advance context: {e}")))?;

        let sampler = StandardSampler::new_softmax(
            vec![
                SamplerStage::Temperature(temperature),
                SamplerStage::TopP(0.9),
                SamplerStage::TopK(40),
            ],
            1,
        );

        let completions = session
            .start_completing_with(sampler, max_tokens)
            .map_err(|e| AppError::Llm(format!("Failed to start completion: {e}")))?;

        for token in completions.into_strings() {
            if !on_token(&token) {
                break;
            }
        }

        Ok(())
    }

    pub fn status(&self) -> EngineStatus {
        EngineStatus {
            is_loaded: self.model.is_some(),
            model_name: self.model.as_ref().map(|_| self.model_name.clone()),
            model_path: self.model.as_ref().map(|_| self.model_path.to_string_lossy().into_owned()),
            total_ram_bytes: Self::total_ram(),
        }
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
