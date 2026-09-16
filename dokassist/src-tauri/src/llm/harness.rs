//! Clinical generation harness: task × effort → sampler, think budget, grounding.
//!
//! Local GGUFs have no native Codex effort API. What RamDoc can control is the
//! sampler, how long a `<think>` block may run, and whether the system prompt
//! forbids inventing clinical facts. This module is that control plane.

use super::thinking::ThinkingEffort;
use llama_cpp_2::sampling::LlamaSampler;

/// What the clinician asked the model to do. Sampling and grounding depend on
/// this more than on the user's effort knob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationTask {
    Chat,
    Report,
    Letter,
    Summary,
    Evidence,
    Improve,
    Extract,
    ToolProbe,
}

/// llama.cpp sampler chain for one generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplerConfig {
    pub temperature: f32,
    pub top_k: i32,
    pub top_p: f32,
    pub min_p: f32,
}

impl SamplerConfig {
    /// Conservative local-model chain. Temperature 0 stays greedy so
    /// deterministic benchmarks do not drift.
    pub fn from_temperature(temperature: f32) -> Self {
        if temperature <= 0.0 {
            Self {
                temperature: 0.0,
                top_k: 1,
                top_p: 1.0,
                min_p: 0.0,
            }
        } else {
            Self {
                temperature,
                top_k: 40,
                top_p: 0.9,
                min_p: 0.05,
            }
        }
    }

    pub fn build(self) -> LlamaSampler {
        LlamaSampler::chain_simple([
            LlamaSampler::temp(self.temperature),
            LlamaSampler::min_p(self.min_p.max(0.0), 1),
            LlamaSampler::top_k(self.top_k),
            LlamaSampler::top_p(self.top_p, 1),
            LlamaSampler::dist(0),
        ])
    }
}

/// Fully resolved generation settings for one call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenerationProfile {
    pub sampler: SamplerConfig,
    pub effort: ThinkingEffort,
    pub max_tokens: usize,
}

const CLINICAL_GROUNDING: &str = "\n\nErfinde keine Medikamente, Dosierungen, Daten, Diagnosen, \
Zitate oder Untersuchungsergebnisse, die nicht in den bereitgestellten Daten stehen. \
Kennzeichne fehlende Angaben ausdrücklich als fehlend — nicht interpolieren.";

const TOOL_PROBE_HINT: &str = "\n\nWenn du ein Tool brauchst, gib AUSSCHLIESSLICH den \
<tool_call>-Block aus. Kein <think>-Block, kein Fliesstext.";

impl GenerationTask {
    pub fn profile(self, user_effort: ThinkingEffort) -> GenerationProfile {
        let effort = match self {
            Self::ToolProbe | Self::Extract => ThinkingEffort::Low,
            _ => user_effort,
        };
        let max_tokens = match self {
            Self::ToolProbe => 512,
            Self::Extract => 512,
            _ => effort.max_tokens(),
        };
        GenerationProfile {
            sampler: self.sampler(),
            effort,
            max_tokens,
        }
    }

    fn sampler(self) -> SamplerConfig {
        match self {
            Self::ToolProbe => SamplerConfig {
                temperature: 0.1,
                top_k: 20,
                top_p: 0.9,
                min_p: 0.1,
            },
            Self::Extract => SamplerConfig::from_temperature(0.0),
            Self::Evidence => SamplerConfig::from_temperature(0.2),
            Self::Report | Self::Letter | Self::Summary => SamplerConfig::from_temperature(0.35),
            Self::Improve => SamplerConfig::from_temperature(0.4),
            Self::Chat => SamplerConfig::from_temperature(0.55),
        }
    }

    pub fn system_suffix(self) -> &'static str {
        match self {
            Self::ToolProbe => TOOL_PROBE_HINT,
            Self::Report | Self::Letter | Self::Summary | Self::Evidence | Self::Chat => {
                CLINICAL_GROUNDING
            }
            Self::Improve | Self::Extract => "",
        }
    }

    pub fn apply_to_system_prompt(self, system_prompt: &str) -> String {
        let suffix = self.system_suffix();
        if suffix.is_empty() {
            system_prompt.to_string()
        } else {
            format!("{system_prompt}{suffix}")
        }
    }
}

/// Remove complete `<think>` blocks so a reasoning preamble cannot hide a tool call.
pub fn strip_think_blocks(output: &str) -> String {
    let mut rest = output;
    let mut kept = String::with_capacity(output.len());
    while let Some(start) = rest.find("<think>") {
        kept.push_str(&rest[..start]);
        rest = &rest[start + "<think>".len()..];
        if let Some(end) = rest.find("</think>") {
            rest = rest[end + "</think>".len()..].trim_start();
        } else {
            return kept;
        }
    }
    kept.push_str(rest);
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_probe_ignores_user_effort_and_stays_short() {
        let profile = GenerationTask::ToolProbe.profile(ThinkingEffort::ExtraHigh);
        assert_eq!(profile.effort, ThinkingEffort::Low);
        assert_eq!(profile.max_tokens, 512);
        assert!(profile.sampler.temperature <= 0.1);
    }

    #[test]
    fn reports_sample_cooler_than_chat() {
        let report = GenerationTask::Report.profile(ThinkingEffort::Medium);
        let chat = GenerationTask::Chat.profile(ThinkingEffort::Medium);
        assert!(report.sampler.temperature < chat.sampler.temperature);
        assert!(report.sampler.temperature < 0.5);
    }

    #[test]
    fn zero_temperature_stays_greedy() {
        let greedy = SamplerConfig::from_temperature(0.0);
        assert_eq!(greedy.top_k, 1);
        assert_eq!(greedy.min_p, 0.0);
    }

    #[test]
    fn strip_think_blocks_exposes_a_following_tool_call() {
        let raw = "<think>I should look this up.</think>\n<tool_call>{\"name\":\"get_patient\",\"args\":{}}</tool_call>";
        let stripped = strip_think_blocks(raw);
        assert!(stripped.starts_with("<tool_call>"));
        assert!(!stripped.contains("<think>"));
    }

    #[test]
    fn unclosed_think_is_dropped() {
        let stripped = strip_think_blocks("<think>still reasoning");
        assert!(stripped.is_empty());
    }

    #[test]
    fn clinical_tasks_forbid_inventing_facts() {
        let suffix = GenerationTask::Report.apply_to_system_prompt("base");
        assert!(suffix.contains("Erfinde keine Medikamente"));
    }
}
