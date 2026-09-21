//! Dedicated tool-router policy, residency admission, and observable state.
//!
//! The router is a separate runtime role. It is lazy-loaded beside the writing
//! engine only when a conservative file-weight budget admits both. Otherwise
//! callers explicitly use the writing engine for the probe.

use super::EngineLifecycleStatus;
use serde::{Deserialize, Serialize};

/// Model weights may use at most half of RAM, leaving room for KV caches and
/// the rest of the application. GGUF file size is only a lower bound on the
/// actual llama.cpp allocation, hence this deliberately conservative limit.
pub const DUAL_RESIDENCY_WEIGHT_FRACTION_NUMERATOR: u64 = 1;
pub const DUAL_RESIDENCY_WEIGHT_FRACTION_DENOMINATOR: u64 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouterMode {
    Managed,
    AdvancedOverride,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouterSelectionSource {
    SmallestCompatibleInstalled,
    ExpertOverride,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouterResidency {
    Unloaded,
    DualResident,
    WritingFallbackMemory,
    Unavailable,
    WritingFallbackError,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouterBenchmarkDiagnostics {
    pub probe_count: u64,
    pub dedicated_probe_count: u64,
    pub writing_fallback_count: u64,
    pub last_probe_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedRouterOverrideMetadata {
    pub available: bool,
    pub configured_filename: Option<String>,
    pub requires_separate_engine: bool,
    pub limitation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterDiagnostics {
    pub role: String,
    pub mode: RouterMode,
    pub selection_source: RouterSelectionSource,
    pub managed_model_filename: Option<String>,
    pub selected_model_filename: Option<String>,
    pub active_model_filename: Option<String>,
    pub resident_engine_count: u8,
    pub lifecycle: EngineLifecycleStatus,
    pub residency: RouterResidency,
    pub dual_residency_safe: bool,
    pub total_ram_bytes: u64,
    pub dual_residency_weight_budget_bytes: u64,
    pub estimated_resident_weight_bytes: u64,
    pub fallback_reason: Option<String>,
    pub benchmark: RouterBenchmarkDiagnostics,
    pub advanced_override: AdvancedRouterOverrideMetadata,
}

pub fn dual_residency_weight_budget(total_ram_bytes: u64) -> u64 {
    total_ram_bytes.saturating_mul(DUAL_RESIDENCY_WEIGHT_FRACTION_NUMERATOR)
        / DUAL_RESIDENCY_WEIGHT_FRACTION_DENOMINATOR
}

pub fn dual_residency_is_safe(
    total_ram_bytes: u64,
    writing_weight_bytes: u64,
    router_weight_bytes: u64,
) -> bool {
    writing_weight_bytes.saturating_add(router_weight_bytes)
        <= dual_residency_weight_budget(total_ram_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_residency_reserves_half_of_ram_for_runtime_overhead() {
        let gib = 1024 * 1024 * 1024;
        assert!(dual_residency_is_safe(32 * gib, 12 * gib, 4 * gib));
        assert!(!dual_residency_is_safe(32 * gib, 12 * gib, 5 * gib));
    }

    #[test]
    fn diagnostics_types_serialize_as_contract_values() {
        assert_eq!(
            serde_json::to_string(&RouterResidency::WritingFallbackMemory).unwrap(),
            "\"writing_fallback_memory\""
        );
    }
}
