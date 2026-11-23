//! Clinical generation harness: task × effort → sampler, think budget, grounding.
//!
//! Local GGUFs have no native Codex effort API. What RamDoc can control is the
//! sampler, how long a `<think>` block may run, and whether the system prompt
//! forbids inventing clinical facts. This module is that control plane.

use super::chat_format::ChatFamily;
use super::thinking::ThinkingEffort;
use llama_cpp_2::sampling::LlamaSampler;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SamplerConfig {
    pub temperature: f32,
    pub top_k: i32,
    pub top_p: f32,
    pub min_p: f32,
    pub repeat_penalty: f32,
    pub presence_penalty: f32,
    #[serde(default)]
    pub seed: u32,
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
                repeat_penalty: 1.0,
                presence_penalty: 0.0,
                seed: 0,
            }
        } else {
            Self {
                temperature,
                top_k: 40,
                top_p: 0.9,
                min_p: 0.05,
                repeat_penalty: 1.0,
                presence_penalty: 0.0,
                seed: 0,
            }
        }
    }

    pub fn validate(self) -> Result<Self, String> {
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err("temperature must be between 0 and 2".into());
        }
        if !(1..=200).contains(&self.top_k) {
            return Err("top_k must be between 1 and 200".into());
        }
        if !(0.0..=1.0).contains(&self.top_p) || !(0.0..=1.0).contains(&self.min_p) {
            return Err("top_p and min_p must be between 0 and 1".into());
        }
        if !(0.8..=2.0).contains(&self.repeat_penalty) {
            return Err("repeat_penalty must be between 0.8 and 2".into());
        }
        if !(0.0..=2.0).contains(&self.presence_penalty) {
            return Err("presence_penalty must be between 0 and 2".into());
        }
        Ok(self)
    }

    pub fn build(self, n_vocab: i32) -> LlamaSampler {
        LlamaSampler::chain_simple([
            LlamaSampler::penalties(n_vocab, 64, self.repeat_penalty, 0.0, self.presence_penalty),
            LlamaSampler::temp(self.temperature),
            LlamaSampler::min_p(self.min_p.max(0.0), 1),
            LlamaSampler::top_k(self.top_k),
            LlamaSampler::top_p(self.top_p, 1),
            LlamaSampler::dist(self.seed),
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

/// Documents (reports, letters, summaries) omit what the record does not
/// contain; placeholders like "nicht angegeben" are as unwanted as invention.
const DOCUMENT_GROUNDING: &str = "\n\nErfinden Sie keine Medikamente, Dosierungen, Daten, \
Diagnosen, Zitate oder Untersuchungsergebnisse, die nicht in den bereitgestellten Daten stehen. \
Fehlende Angaben lassen Sie weg, statt sie zu ergänzen oder als fehlend zu vermerken.";

/// Answers to questions must say when the record is silent, rather than
/// leave the clinician to infer it from an omission.
const ANSWER_GROUNDING: &str = "\n\nErfinden Sie keine Medikamente, Dosierungen, Daten, \
Diagnosen, Zitate oder Untersuchungsergebnisse, die nicht in den bereitgestellten Daten stehen. \
Ist eine erfragte Angabe nicht dokumentiert, sagen Sie das ausdrücklich, statt zu interpolieren.";

const TOOL_PROBE_HINT: &str = "\n\nWenn du ein Tool brauchst, gib AUSSCHLIESSLICH den \
<tool_call>-Block aus. Kein <think>-Block, kein Fliesstext.";

/// Vendor sampling recommendations differ far more between model families than
/// [`ChatFamily`] can express (Qwen, Granite, Nemotron and Ministral all use
/// the native llama.cpp template path). This enum selects the per-family base
/// sampler; [`GenerationTask`] then adjusts it per task.
///
/// Resolved once at model load by [`SamplerFamily::resolve`]. Anything not
/// recognised is [`SamplerFamily::Generic`], which reproduces the samplers used
/// before family awareness so unknown models keep their behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SamplerFamily {
    /// Legacy task samplers. Also MedGemma and Phi, whose vendor examples
    /// decode greedily, which RamDoc does not adopt for free text.
    #[default]
    Generic,
    /// Qwen 3, 3.5, 3.6, 3.8 (thinking-capable templates).
    Qwen,
    /// Gemma 4 (all sizes, incl. QAT).
    Gemma4,
    /// gpt-oss (Harmony). Reasoning cannot be switched off, only set to low.
    GptOss,
    /// IBM Granite 4.x.
    Granite,
    /// NVIDIA Nemotron 3.x.
    Nemotron,
    /// Mistral Ministral 3 Instruct (non-reasoning).
    MinistralInstruct,
}

/// Family base sampler plus the temperature each task class uses. Top-k,
/// top-p, min-p and penalties come from `base`; documents additionally get a
/// min-p floor of [`DOCUMENT_MIN_P`] for grounding.
struct FamilySampling {
    base: SamplerConfig,
    chat: f32,
    improve: f32,
    document: f32,
    evidence: f32,
}

/// Min-p floor for clinical documents: drops tokens far below the most likely
/// one, which curbs invented specifics without making decoding greedy.
const DOCUMENT_MIN_P: f32 = 0.05;

const fn base_sampler(top_k: i32, top_p: f32, min_p: f32) -> SamplerConfig {
    SamplerConfig {
        temperature: 1.0,
        top_k,
        top_p,
        min_p,
        repeat_penalty: 1.0,
        presence_penalty: 0.0,
        seed: 0,
    }
}

/// Tool-call probe: reasoning is switched off (or at its lowest level), the
/// output is a short structured block, and the probe may run on a separate
/// router model, so it stays low-temperature and family-independent.
const TOOL_PROBE_SAMPLER: SamplerConfig = SamplerConfig {
    temperature: 0.1,
    top_k: 20,
    top_p: 0.9,
    min_p: 0.1,
    repeat_penalty: 1.0,
    presence_penalty: 0.0,
    seed: 0,
};

impl SamplerFamily {
    /// Resolve the family from what the engine knows at load time: the chat
    /// dialect detected from the embedded template, the GGUF filename (the
    /// catalogue id in `download.rs`) and GGUF `general.architecture`.
    ///
    /// The filename is checked before the architecture because several
    /// architectures are shared by generations with different vendor
    /// recommendations (`granite`, `nemotron_h`, `gemma3` for MedGemma).
    pub fn resolve(model_filename: &str, gguf_architecture: &str, chat_family: ChatFamily) -> Self {
        match chat_family {
            ChatFamily::Harmony => return Self::GptOss,
            ChatFamily::Gemma4 => return Self::Gemma4,
            ChatFamily::Native => {}
        }
        let name = model_filename.to_ascii_lowercase();
        if let Some(family) = Self::from_model_name(&name) {
            return family;
        }
        Self::from_architecture(&gguf_architecture.to_ascii_lowercase())
    }

    fn from_model_name(name: &str) -> Option<Self> {
        if name.contains("medgemma") {
            // Gemma 3 based; the vendor default is greedy decoding.
            return Some(Self::Generic);
        }
        if name.contains("gemma-4") || name.contains("gemma4") {
            return Some(Self::Gemma4);
        }
        if name.contains("gpt-oss") {
            return Some(Self::GptOss);
        }
        if name.contains("qwen3") {
            return Some(Self::Qwen);
        }
        if name.contains("granite-4") {
            return Some(Self::Granite);
        }
        if name.contains("nemotron-3") {
            return Some(Self::Nemotron);
        }
        if name.contains("ministral-3") && name.contains("instruct") {
            return Some(Self::MinistralInstruct);
        }
        None
    }

    /// Only architectures that map to a single recommendation set.
    fn from_architecture(architecture: &str) -> Self {
        match architecture {
            // qwen3, qwen3moe, qwen3next, qwen35, qwen35moe, ...
            arch if arch.starts_with("qwen3") => Self::Qwen,
            "gemma4" => Self::Gemma4,
            "gpt-oss" => Self::GptOss,
            _ => Self::Generic,
        }
    }

    /// Whether RamDoc can switch reasoning off for a turn. Harmony only goes
    /// down to `Reasoning: low`, so its "reasoning-off" tasks still reason.
    pub fn can_disable_reasoning(self) -> bool {
        !matches!(self, Self::GptOss)
    }

    /// Vendor base sampler and per-task temperatures. Values other than
    /// `Generic` must be validated with the referral sweep before they become
    /// defaults (issue #558, `docs/referral-quality-benchmark.md`).
    fn sampling(self) -> FamilySampling {
        match self {
            // Exactly the pre-#558 task samplers (`from_temperature`).
            Self::Generic => FamilySampling {
                base: base_sampler(40, 0.9, 0.05),
                chat: 0.55,
                improve: 0.4,
                document: 0.35,
                evidence: 0.2,
            },
            // Qwen3 thinking mode: temperature 0.6, top_p 0.95, top_k 20,
            // min_p 0, and "DO NOT use greedy decoding".
            // https://huggingface.co/Qwen/Qwen3-8B#best-practices
            // https://huggingface.co/Qwen/Qwen3-8B/blob/main/generation_config.json
            // Qwen 3.5 / 3.6 / 3.8 keep top_p 0.95, top_k 20, min_p 0; their
            // thinking temperature is 1.0 (general) or 0.6 (precise tasks).
            // https://huggingface.co/Qwen/Qwen3.5-9B
            // https://huggingface.co/Qwen/Qwen3.6-35B-A3B
            // https://huggingface.co/Qwen/Qwen3.8-27B
            // Presence penalty stays 0: the cards warn that it can mix
            // languages, which German clinical text cannot afford. Documents
            // use the published precise value 0.6; chat sits between 0.6 and 1.0.
            Self::Qwen => FamilySampling {
                base: base_sampler(20, 0.95, 0.0),
                chat: 0.7,
                improve: 0.65,
                document: 0.6,
                evidence: 0.6,
            },
            // "temperature=1.0, top_p=0.95, top_k=64" for all use cases.
            // https://huggingface.co/google/gemma-4-31B-it
            // https://huggingface.co/google/gemma-4-31B-it/blob/main/generation_config.json
            Self::Gemma4 => FamilySampling {
                base: base_sampler(64, 0.95, 0.0),
                chat: 1.0,
                improve: 0.9,
                document: 0.8,
                evidence: 0.8,
            },
            // "We recommend sampling with temperature=1.0 and top_p=1.0."
            // https://github.com/openai/gpt-oss
            // No top-k is published; 100 only trims the far tail
            // (`validate()` does not accept 0 = disabled).
            Self::GptOss => FamilySampling {
                base: base_sampler(100, 1.0, 0.0),
                chat: 1.0,
                improve: 0.9,
                document: 0.8,
                evidence: 0.8,
            },
            // "Use temperature=1.0 and top_p=0.95 across all tasks", with
            // reasoning on or off. No top-k published; llama.cpp's 40 is kept.
            // https://huggingface.co/ibm-granite/granite-4.2-30b
            Self::Granite => FamilySampling {
                base: base_sampler(40, 0.95, 0.0),
                chat: 1.0,
                improve: 0.9,
                document: 0.8,
                evidence: 0.8,
            },
            // "Temperature 1.0, Top_P 0.95" for reasoning on, off and tool
            // calling. No top-k published; llama.cpp's 40 is kept.
            // https://huggingface.co/nvidia/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-BF16
            Self::Nemotron => FamilySampling {
                base: base_sampler(40, 0.95, 0.0),
                chat: 1.0,
                improve: 0.9,
                document: 0.8,
                evidence: 0.8,
            },
            // "Use a temperature below 0.1 for daily-driver and production
            // environments." Non-reasoning, so a low temperature cannot loop a
            // think block. top_k / top_p unpublished; legacy values kept.
            // https://huggingface.co/mistralai/Ministral-3-14B-Instruct-2512
            Self::MinistralInstruct => FamilySampling {
                base: base_sampler(40, 0.9, 0.05),
                chat: 0.09,
                improve: 0.07,
                document: 0.05,
                evidence: 0.05,
            },
        }
    }
}

impl GenerationTask {
    /// Profile with the [`SamplerFamily::Generic`] (pre-#558) sampler. Use
    /// [`Self::profile_for`] when the loaded model is known.
    pub fn profile(self, user_effort: ThinkingEffort) -> GenerationProfile {
        self.profile_for(SamplerFamily::Generic, user_effort)
    }

    pub fn profile_for(
        self,
        family: SamplerFamily,
        user_effort: ThinkingEffort,
    ) -> GenerationProfile {
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
            sampler: self.sampler(family),
            effort,
            max_tokens,
        }
    }

    /// [`Self::profile_for`] with an optional user sampler, which always wins
    /// over the family default once it passes [`SamplerConfig::validate`].
    pub fn profile_with_override(
        self,
        family: SamplerFamily,
        user_effort: ThinkingEffort,
        sampler: Option<SamplerConfig>,
    ) -> Result<GenerationProfile, String> {
        let mut profile = self.profile_for(family, user_effort);
        if let Some(sampler) = sampler {
            profile.sampler = sampler.validate()?;
        }
        Ok(profile)
    }

    /// Whether this task may produce a reasoning block on `family`. ToolProbe
    /// and Extract run with reasoning switched off
    /// (`format_chat_history_reasoning_off`) wherever the family allows it.
    pub fn may_reason(self, family: SamplerFamily) -> bool {
        !matches!(self, Self::ToolProbe | Self::Extract) || !family.can_disable_reasoning()
    }

    fn sampler(self, family: SamplerFamily) -> SamplerConfig {
        let sampling = family.sampling();
        let with_temperature = |temperature: f32| SamplerConfig {
            temperature,
            ..sampling.base
        };
        let document = |temperature: f32| SamplerConfig {
            min_p: sampling.base.min_p.max(DOCUMENT_MIN_P),
            ..with_temperature(temperature)
        };
        match self {
            Self::ToolProbe => TOOL_PROBE_SAMPLER,
            // Greedy suits reasoning-off structured output, but greedy inside
            // a reasoning block loops, so a family that cannot switch
            // reasoning off gets the low-temperature probe sampler instead.
            Self::Extract if self.may_reason(family) => TOOL_PROBE_SAMPLER,
            Self::Extract => SamplerConfig::from_temperature(0.0),
            Self::Evidence => document(sampling.evidence),
            Self::Report | Self::Letter | Self::Summary => document(sampling.document),
            Self::Improve => with_temperature(sampling.improve),
            Self::Chat => with_temperature(sampling.chat),
        }
    }

    pub fn system_suffix(self) -> &'static str {
        match self {
            Self::ToolProbe => TOOL_PROBE_HINT,
            Self::Report | Self::Letter | Self::Summary => DOCUMENT_GROUNDING,
            Self::Evidence | Self::Chat => ANSWER_GROUNDING,
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
    fn rejects_unsafe_custom_sampler_values() {
        let mut sampler = SamplerConfig::from_temperature(0.35);
        sampler.top_p = 1.5;
        assert!(sampler.validate().is_err());
    }

    const ALL_FAMILIES: [SamplerFamily; 7] = [
        SamplerFamily::Generic,
        SamplerFamily::Qwen,
        SamplerFamily::Gemma4,
        SamplerFamily::GptOss,
        SamplerFamily::Granite,
        SamplerFamily::Nemotron,
        SamplerFamily::MinistralInstruct,
    ];
    const ALL_TASKS: [GenerationTask; 8] = [
        GenerationTask::Chat,
        GenerationTask::Report,
        GenerationTask::Letter,
        GenerationTask::Summary,
        GenerationTask::Evidence,
        GenerationTask::Improve,
        GenerationTask::Extract,
        GenerationTask::ToolProbe,
    ];
    const ALL_EFFORTS: [ThinkingEffort; 4] = [
        ThinkingEffort::Low,
        ThinkingEffort::Medium,
        ThinkingEffort::High,
        ThinkingEffort::ExtraHigh,
    ];
    const DOCUMENT_TASKS: [GenerationTask; 4] = [
        GenerationTask::Report,
        GenerationTask::Letter,
        GenerationTask::Summary,
        GenerationTask::Evidence,
    ];

    fn is_greedy(sampler: SamplerConfig) -> bool {
        sampler.temperature <= 0.0 || sampler.top_k <= 1
    }

    #[test]
    fn resolves_every_catalogue_model_to_its_family() {
        use crate::llm::download::MODELS;
        for model in MODELS {
            let name = model.filename.to_ascii_lowercase();
            let expected = if name.contains("medgemma") || name.contains("phi-4") {
                SamplerFamily::Generic
            } else if name.contains("gemma-4") {
                SamplerFamily::Gemma4
            } else if name.contains("qwen3") {
                SamplerFamily::Qwen
            } else if name.contains("gpt-oss") {
                SamplerFamily::GptOss
            } else if name.contains("granite") {
                SamplerFamily::Granite
            } else if name.contains("nemotron") {
                SamplerFamily::Nemotron
            } else if name.contains("ministral") {
                SamplerFamily::MinistralInstruct
            } else {
                panic!(
                    "catalogue model {} has no expected sampler family",
                    model.filename
                );
            };
            assert_eq!(
                SamplerFamily::resolve(model.filename, "unknown", ChatFamily::Native),
                expected,
                "{}",
                model.filename
            );
        }
    }

    #[test]
    fn chat_dialect_wins_over_filename() {
        assert_eq!(
            SamplerFamily::resolve("renamed.gguf", "unknown", ChatFamily::Harmony),
            SamplerFamily::GptOss
        );
        assert_eq!(
            SamplerFamily::resolve("renamed.gguf", "unknown", ChatFamily::Gemma4),
            SamplerFamily::Gemma4
        );
    }

    #[test]
    fn architecture_is_the_fallback_for_unknown_filenames() {
        for (arch, family) in [
            ("qwen3", SamplerFamily::Qwen),
            ("qwen3moe", SamplerFamily::Qwen),
            ("qwen35moe", SamplerFamily::Qwen),
            ("gemma4", SamplerFamily::Gemma4),
            ("gpt-oss", SamplerFamily::GptOss),
            // Shared across generations with different recommendations.
            ("granite", SamplerFamily::Generic),
            ("nemotron_h_moe", SamplerFamily::Generic),
            ("gemma3", SamplerFamily::Generic),
            ("llama", SamplerFamily::Generic),
            ("qwen2", SamplerFamily::Generic),
            ("unknown", SamplerFamily::Generic),
        ] {
            assert_eq!(
                SamplerFamily::resolve("custom-model.gguf", arch, ChatFamily::Native),
                family,
                "{arch}"
            );
        }
    }

    #[test]
    fn ministral_reasoning_variant_is_not_treated_as_instruct() {
        assert_eq!(
            SamplerFamily::resolve(
                "Ministral-3-14B-Reasoning-2512-Q4_K_M.gguf",
                "mistral3",
                ChatFamily::Native
            ),
            SamplerFamily::Generic
        );
    }

    #[test]
    fn every_family_task_and_effort_sampler_is_valid() {
        for family in ALL_FAMILIES {
            for task in ALL_TASKS {
                for effort in ALL_EFFORTS {
                    let profile = task.profile_for(family, effort);
                    assert!(
                        profile.sampler.validate().is_ok(),
                        "{family:?} {task:?} {effort:?}: {:?}",
                        profile.sampler
                    );
                }
            }
        }
    }

    #[test]
    fn reasoning_generations_are_never_greedy() {
        for family in ALL_FAMILIES {
            for task in ALL_TASKS {
                if !task.may_reason(family) {
                    continue;
                }
                for effort in ALL_EFFORTS {
                    let sampler = task.profile_for(family, effort).sampler;
                    assert!(!is_greedy(sampler), "{family:?} {task:?} {effort:?}");
                }
            }
        }
    }

    #[test]
    fn greedy_is_reserved_for_reasoning_off_extraction() {
        for family in ALL_FAMILIES {
            for task in ALL_TASKS {
                let greedy = is_greedy(task.profile_for(family, ThinkingEffort::Medium).sampler);
                let expected = task == GenerationTask::Extract && family.can_disable_reasoning();
                assert_eq!(greedy, expected, "{family:?} {task:?}");
            }
        }
    }

    #[test]
    fn documents_sample_cooler_than_chat_in_every_family() {
        for family in ALL_FAMILIES {
            let chat = GenerationTask::Chat.profile_for(family, ThinkingEffort::Medium);
            for task in DOCUMENT_TASKS {
                let document = task.profile_for(family, ThinkingEffort::Medium);
                assert!(
                    document.sampler.temperature < chat.sampler.temperature,
                    "{family:?} {task:?}"
                );
                assert!(
                    document.sampler.min_p >= DOCUMENT_MIN_P,
                    "{family:?} {task:?}"
                );
            }
        }
    }

    #[test]
    fn tool_probe_stays_low_temperature_in_every_family() {
        for family in ALL_FAMILIES {
            let probe = GenerationTask::ToolProbe.profile_for(family, ThinkingEffort::ExtraHigh);
            assert_eq!(probe.sampler, TOOL_PROBE_SAMPLER, "{family:?}");
            assert_eq!(probe.effort, ThinkingEffort::Low);
        }
    }

    #[test]
    fn generic_family_matches_pre_558_samplers() {
        let legacy = [
            (GenerationTask::Chat, SamplerConfig::from_temperature(0.55)),
            (
                GenerationTask::Improve,
                SamplerConfig::from_temperature(0.4),
            ),
            (
                GenerationTask::Report,
                SamplerConfig::from_temperature(0.35),
            ),
            (
                GenerationTask::Letter,
                SamplerConfig::from_temperature(0.35),
            ),
            (
                GenerationTask::Summary,
                SamplerConfig::from_temperature(0.35),
            ),
            (
                GenerationTask::Evidence,
                SamplerConfig::from_temperature(0.2),
            ),
            (
                GenerationTask::Extract,
                SamplerConfig::from_temperature(0.0),
            ),
            (
                GenerationTask::ToolProbe,
                SamplerConfig {
                    temperature: 0.1,
                    top_k: 20,
                    top_p: 0.9,
                    min_p: 0.1,
                    repeat_penalty: 1.0,
                    presence_penalty: 0.0,
                    seed: 0,
                },
            ),
        ];
        for (task, sampler) in legacy {
            for effort in ALL_EFFORTS {
                assert_eq!(
                    task.profile_for(SamplerFamily::Generic, effort).sampler,
                    sampler,
                    "{task:?} {effort:?}"
                );
                assert_eq!(
                    task.profile(effort),
                    task.profile_for(SamplerFamily::Generic, effort)
                );
            }
        }
    }

    #[test]
    fn user_sampler_overrides_the_family_default() {
        let custom = SamplerConfig::from_temperature(0.2);
        for family in ALL_FAMILIES {
            let profile = GenerationTask::Report
                .profile_with_override(family, ThinkingEffort::Low, Some(custom))
                .unwrap();
            assert_eq!(profile.sampler, custom, "{family:?}");
            assert_eq!(profile.effort, ThinkingEffort::Low);
        }
        let mut unsafe_sampler = custom;
        unsafe_sampler.top_p = 1.5;
        assert!(GenerationTask::Report
            .profile_with_override(
                SamplerFamily::Qwen,
                ThinkingEffort::Low,
                Some(unsafe_sampler)
            )
            .is_err());
        assert_eq!(
            GenerationTask::Report
                .profile_with_override(SamplerFamily::Qwen, ThinkingEffort::Low, None)
                .unwrap(),
            GenerationTask::Report.profile_for(SamplerFamily::Qwen, ThinkingEffort::Low)
        );
    }

    #[test]
    fn qwen_follows_the_published_thinking_profile() {
        // https://huggingface.co/Qwen/Qwen3-8B#best-practices
        let report = GenerationTask::Report.profile_for(SamplerFamily::Qwen, ThinkingEffort::Low);
        assert_eq!(report.sampler.temperature, 0.6);
        assert_eq!(report.sampler.top_k, 20);
        assert_eq!(report.sampler.top_p, 0.95);
        assert_eq!(report.sampler.presence_penalty, 0.0);
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
        for task in [
            GenerationTask::Report,
            GenerationTask::Letter,
            GenerationTask::Summary,
            GenerationTask::Evidence,
            GenerationTask::Chat,
        ] {
            assert!(task
                .apply_to_system_prompt("base")
                .contains("Erfinden Sie keine Medikamente"));
        }
    }

    #[test]
    fn documents_omit_missing_data_while_answers_name_it() {
        // Reports used to be told both to omit missing data (report prompt)
        // and to mark it as missing (this suffix).
        let report = GenerationTask::Report.system_suffix();
        assert!(report.contains("lassen Sie weg"));
        assert!(!report.contains("ausdrücklich"));
        assert!(GenerationTask::Evidence
            .system_suffix()
            .contains("sagen Sie das ausdrücklich"));
    }
}
