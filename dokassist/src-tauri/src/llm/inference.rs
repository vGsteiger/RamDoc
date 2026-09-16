use super::memory_governor::MemoryGovernorDiagnostics;
use crate::error::AppError;
use llama_cpp_2::context::params::KvCacheType;
use serde::{Deserialize, Serialize};

const EXPERIMENTAL_CONTEXT_SIZE: usize = 32_768;
const LOGICAL_BATCH_SIZE: u32 = 2_048;
const MICRO_BATCH_SIZE: u32 = 512;
const COMPLETION_HEADROOM: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KvCacheQuantization {
    F16,
    Q8,
    Q4,
}

impl KvCacheQuantization {
    pub(crate) fn llama_type(self) -> KvCacheType {
        match self {
            Self::F16 => KvCacheType::F16,
            Self::Q8 => KvCacheType::Q8_0,
            Self::Q4 => KvCacheType::Q4_0,
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::F16 => "F16",
            Self::Q8 => "Q8_0",
            Self::Q4 => "Q4_0",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlashAttentionMode {
    Enabled,
    Auto,
}

impl FlashAttentionMode {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Enabled => "enabled",
            Self::Auto => "auto",
        }
    }
}

/// A validated set of llama.cpp context parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferenceProfile {
    pub name: String,
    pub n_ctx: usize,
    pub kv_cache: KvCacheQuantization,
    pub n_batch: u32,
    pub n_ubatch: u32,
    pub completion_headroom: usize,
    pub(crate) flash_attention: FlashAttentionMode,
}

impl InferenceProfile {
    pub fn named(name: &str, conservative_context: usize) -> Result<Self, AppError> {
        let (n_ctx, kv_cache) = match name {
            "conservative" => (conservative_context, KvCacheQuantization::F16),
            "f16-32k" => (EXPERIMENTAL_CONTEXT_SIZE, KvCacheQuantization::F16),
            "q8-32k" => (EXPERIMENTAL_CONTEXT_SIZE, KvCacheQuantization::Q8),
            "q4-32k" => (EXPERIMENTAL_CONTEXT_SIZE, KvCacheQuantization::Q4),
            _ => {
                return Err(AppError::Validation(format!(
                    "Unknown inference profile '{name}'. Expected one of: conservative, f16-32k, q8-32k, q4-32k"
                )))
            }
        };

        let profile = Self {
            name: name.to_string(),
            n_ctx,
            kv_cache,
            n_batch: LOGICAL_BATCH_SIZE,
            n_ubatch: MICRO_BATCH_SIZE,
            completion_headroom: COMPLETION_HEADROOM,
            flash_attention: FlashAttentionMode::Enabled,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<(), AppError> {
        if self.n_ctx == 0 || self.n_ctx > u32::MAX as usize {
            return Err(AppError::Validation(
                "Inference context must fit in a non-zero u32".to_string(),
            ));
        }
        if self.n_batch == 0 {
            return Err(AppError::Validation(
                "Inference logical batch must be non-zero".to_string(),
            ));
        }
        if self.n_ubatch == 0 || self.n_ubatch > self.n_batch {
            return Err(AppError::Validation(
                "Inference micro-batch must be non-zero and no larger than the logical batch"
                    .to_string(),
            ));
        }
        if self.n_batch as usize > self.n_ctx {
            return Err(AppError::Validation(
                "Inference logical batch cannot exceed the context size".to_string(),
            ));
        }
        if self.completion_headroom == 0 || self.completion_headroom >= self.n_ctx {
            return Err(AppError::Validation(
                "Completion headroom must be non-zero and smaller than the context size"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Construct a validated profile for the explicit local benchmark harness.
    ///
    /// This is deliberately unavailable to production callers: arbitrary
    /// profiles are research inputs and still pass through the memory governor
    /// before any model tensors are loaded.
    #[cfg(any(test, feature = "benchmark-harness"))]
    pub(crate) fn for_benchmark(
        context_size: usize,
        kv_cache: KvCacheQuantization,
        n_batch: u32,
        n_ubatch: u32,
        completion_headroom: usize,
    ) -> Result<Self, AppError> {
        let profile = Self {
            name: format!(
                "benchmark-{}-{}",
                context_size,
                kv_cache.label().to_ascii_lowercase()
            ),
            n_ctx: context_size,
            kv_cache,
            n_batch,
            n_ubatch,
            completion_headroom,
            flash_attention: FlashAttentionMode::Enabled,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub(crate) fn resolved_for_model(&self, native_context: usize) -> Result<Self, AppError> {
        let mut resolved = self.clone();
        if native_context != 0 {
            resolved.n_ctx = resolved.n_ctx.min(native_context);
        }
        resolved.validate()?;
        Ok(resolved)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InferenceDiagnostics {
    pub profile: String,
    pub context_size: usize,
    pub kv_cache_k: String,
    pub kv_cache_v: String,
    pub n_batch: u32,
    pub n_ubatch: u32,
    pub flash_attention: String,
    pub completion_headroom: usize,
    pub fallback: Option<String>,
    /// Stable code used by clients to localize the fallback diagnostic.
    pub fallback_code: Option<String>,
    pub memory_governor: Option<MemoryGovernorDiagnostics>,
}

impl InferenceDiagnostics {
    pub(crate) fn from_profile(
        profile: &InferenceProfile,
        fallback: Option<String>,
        fallback_code: Option<String>,
        memory_governor: Option<MemoryGovernorDiagnostics>,
    ) -> Self {
        Self {
            profile: profile.name.clone(),
            context_size: profile.n_ctx,
            kv_cache_k: profile.kv_cache.label().to_string(),
            kv_cache_v: profile.kv_cache.label().to_string(),
            n_batch: profile.n_batch,
            n_ubatch: profile.n_ubatch,
            flash_attention: profile.flash_attention.label().to_string(),
            completion_headroom: profile.completion_headroom,
            fallback,
            fallback_code,
            memory_governor,
        }
    }
}

pub(crate) fn validate_context_budget(
    context_size: usize,
    prompt_tokens: usize,
    max_tokens: usize,
    completion_headroom: usize,
) -> Result<(), AppError> {
    let reserved = max_tokens.max(completion_headroom);
    let max_prompt_tokens = context_size.saturating_sub(reserved);
    if reserved >= context_size || prompt_tokens > max_prompt_tokens {
        return Err(AppError::Llm(format!(
            "Prompt too long ({prompt_tokens} tokens): context {context_size} reserves {reserved} tokens for completion, leaving {max_prompt_tokens} prompt tokens"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_profiles_are_valid_and_conservative_remains_f16() {
        for name in ["conservative", "f16-32k", "q8-32k", "q4-32k"] {
            let profile = InferenceProfile::named(name, 16_384).unwrap();
            profile.validate().unwrap();
        }
        let conservative = InferenceProfile::named("conservative", 16_384).unwrap();
        assert_eq!(conservative.n_ctx, 16_384);
        assert_eq!(conservative.kv_cache, KvCacheQuantization::F16);
    }

    #[test]
    fn unknown_profile_is_rejected() {
        assert!(InferenceProfile::named("turbo", 16_384).is_err());
    }

    #[test]
    fn invalid_batch_relationships_are_rejected() {
        let mut profile = InferenceProfile::named("conservative", 16_384).unwrap();
        profile.n_ubatch = profile.n_batch + 1;
        assert!(profile.validate().is_err());

        profile.n_ubatch = 512;
        profile.n_batch = profile.n_ctx as u32 + 1;
        assert!(profile.validate().is_err());
    }

    #[test]
    fn native_context_caps_experimental_profile() {
        let profile = InferenceProfile::named("q8-32k", 16_384).unwrap();
        assert_eq!(profile.resolved_for_model(24_576).unwrap().n_ctx, 24_576);
    }

    #[test]
    fn context_budget_reserves_at_least_configured_headroom() {
        assert!(validate_context_budget(16_384, 12_288, 512, 4_096).is_ok());
        assert!(validate_context_budget(16_384, 12_289, 512, 4_096).is_err());
    }

    #[test]
    fn context_budget_reserves_larger_requested_completion() {
        assert!(validate_context_budget(16_384, 8_192, 8_192, 4_096).is_ok());
        assert!(validate_context_budget(16_384, 8_193, 8_192, 4_096).is_err());
    }
}
