//! Tool-routing policy and diagnostics.
//!
//! The application deliberately keeps one LLM resident. The writing model is
//! therefore also the managed tool-router model today. This module makes that
//! policy explicit so an advanced, separately resident router can be added
//! later without silently changing which model sees patient-scoped prompts.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RouterMode {
    /// Route tool decisions with the active writing model.
    Managed,
    /// Reserved for a future explicit, separately resident router model.
    AdvancedOverride,
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
    pub managed_model_filename: Option<String>,
    pub active_model_filename: Option<String>,
    pub resident_engine_count: u8,
    pub advanced_override: AdvancedRouterOverrideMetadata,
}

impl RouterDiagnostics {
    pub fn managed(
        managed_model_filename: Option<String>,
        active_model_filename: Option<String>,
    ) -> Self {
        Self {
            role: "tool_router".to_string(),
            mode: RouterMode::Managed,
            managed_model_filename,
            active_model_filename,
            resident_engine_count: 1,
            advanced_override: AdvancedRouterOverrideMetadata {
                available: false,
                configured_filename: None,
                requires_separate_engine: true,
                limitation: "Advanced router overrides are metadata-only until a separately resident router engine is explicitly implemented.".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_router_never_claims_a_second_resident_engine() {
        let diagnostics = RouterDiagnostics::managed(
            Some("writing.gguf".to_string()),
            Some("writing.gguf".to_string()),
        );
        assert_eq!(diagnostics.mode, RouterMode::Managed);
        assert_eq!(diagnostics.resident_engine_count, 1);
        assert!(!diagnostics.advanced_override.available);
        assert!(diagnostics.advanced_override.requires_separate_engine);
    }
}
