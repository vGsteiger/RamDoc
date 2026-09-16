//! User-facing reasoning effort, mapped onto think-token budgets.
//!
//! Local GGUFs do not expose a native Codex "effort" knob. Qwen-style models
//! emit `<think>` blocks and gpt-oss Harmony records a reasoning level inside
//! the system turn. The reliable, model-agnostic lever is the same two-phase
//! generator already used for reports: cap tokens spent inside `<think>`, then
//! force the model to write the actual answer.

use super::context_cache::InferenceSession;
use super::engine::LlmEngine;
use super::harness::{GenerationProfile, GenerationTask};
use super::prompts;
use super::utf8;
use crate::error::AppError;
use serde::{Deserialize, Serialize};

/// Codex-style reasoning effort. Default matches the previous hardcoded
/// report think budget of 1024 tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingEffort {
    Low,
    #[default]
    Medium,
    High,
    ExtraHigh,
}

impl ThinkingEffort {
    /// Tokens allowed inside a `<think>` block before it is force-closed.
    pub fn think_token_budget(self) -> usize {
        match self {
            Self::Low => 128,
            Self::Medium => 1024,
            Self::High => 2048,
            Self::ExtraHigh => 4096,
        }
    }

    /// Total completion tokens for the turn, including thinking.
    pub fn max_tokens(self) -> usize {
        match self {
            Self::Low => 4096,
            Self::Medium => 4096,
            Self::High => 6144,
            Self::ExtraHigh => 8192,
        }
    }

    /// Appended to the system prompt so models that honour instructions spend
    /// more or less time in `<think>`. Empty for Medium (previous behaviour).
    pub fn system_suffix(self) -> &'static str {
        match self {
            Self::Low => {
                "\n\nAntworte direkt und knapp. Verwende keinen oder nur einen sehr kurzen <think>-Block."
            }
            Self::Medium => "",
            Self::High => {
                "\n\nDenke zuerst in einem <think>-Block gründlich nach, bevor du die eigentliche Antwort schreibst."
            }
            Self::ExtraHigh => {
                "\n\nDenke ausführlich in einem <think>-Block nach. Prüfe klinische Angaben, Dosierungen, Daten und Widersprüche, bevor du die eigentliche Antwort schreibst."
            }
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

/// Two-phase generation that caps the `<think>` block according to `effort`.
///
/// **Phase 1** – normal streaming; if the think budget is consumed before
/// `</think>` appears, generation stops early.
///
/// **Phase 2** (only if the budget was hit) – a `</think>` marker is injected,
/// then generation resumes through the GGUF's own chat template.
pub fn generate_with_think_budget(
    engine: &LlmEngine,
    system_prompt: &str,
    user_message: &str,
    task: GenerationTask,
    effort: ThinkingEffort,
    emit: &dyn Fn(&str),
) -> Result<String, AppError> {
    let profile = task.profile(effort);
    let system_prompt = profile.effort.apply_to_system_prompt(system_prompt);
    let system_prompt = task.apply_to_system_prompt(&system_prompt);
    generate_with_think_budget_from_prompt(
        engine,
        &system_prompt,
        None,
        user_message,
        profile,
        None,
        emit,
    )
}

/// Like [`generate_with_think_budget`], but phase 1 uses a pre-formatted prompt
/// (multi-turn agent history). `system_prompt` must already include effort and
/// task suffixes. Phase 2 still continues through the GGUF chat template.
pub fn generate_with_think_budget_from_prompt(
    engine: &LlmEngine,
    system_prompt: &str,
    formatted_prompt: Option<&str>,
    user_message: &str,
    profile: GenerationProfile,
    session: Option<&InferenceSession>,
    emit: &dyn Fn(&str),
) -> Result<String, AppError> {
    let max_tokens = profile.max_tokens;
    let max_think_tokens = profile.effort.think_token_budget();
    let mut output = String::new();
    let mut think_tokens: usize = 0;
    let mut budget_hit = false;
    let mut in_think = false;
    let mut tag_tail = String::new();

    let phase1 = |on_token: &mut dyn FnMut(&str) -> bool| -> Result<(), AppError> {
        if let Some(prompt) = formatted_prompt {
            if let Some(session) = session {
                engine.generate_streaming_session_with_sampler(
                    session,
                    system_prompt,
                    prompt,
                    max_tokens,
                    profile.sampler,
                    on_token,
                )
            } else {
                engine.generate_streaming_raw_with_sampler(
                    prompt,
                    max_tokens,
                    profile.sampler,
                    on_token,
                )
            }
        } else {
            let prompt = engine.format_chat_history(
                system_prompt,
                &[super::engine::AgentMessage {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                }],
            )?;
            engine.generate_streaming_raw_with_sampler(
                &prompt,
                max_tokens,
                profile.sampler,
                on_token,
            )
        }
    };

    phase1(&mut |token| {
        output.push_str(token);
        tag_tail.push_str(token);

        if !in_think && tag_tail.contains("<think>") {
            in_think = true;
        }
        if in_think && tag_tail.contains("</think>") {
            in_think = false;
        }
        if tag_tail.len() > 16 {
            let drain_end = tag_tail.len() - 16;
            let drain_end = utf8::find_boundary_backward(&tag_tail, drain_end);
            tag_tail.drain(..drain_end);
        }

        if in_think {
            think_tokens += 1;
            if think_tokens >= max_think_tokens {
                budget_hit = true;
                return false;
            }
        }

        emit(token);
        true
    })?;

    let phase1_stats = engine.last_generation_stats();

    if budget_hit {
        let close_tag = "</think>\n\n";
        output.push_str(close_tag);
        emit(close_tag);

        let tail_start = output.len().saturating_sub(1_200);
        let tail_start = utf8::find_boundary_forward(&output, tail_start);
        let continuation = format!(
            "Setze die Antwort auf die folgende ursprüngliche Aufgabe unmittelbar fort. \
             Wiederhole nichts und gib nur den fertigen Inhalt aus.\n\nAufgabe:\n{user_message}\n\n\
             Bisheriges Ende:\n{}",
            &output[tail_start..]
        );

        if let Some(prompt) = formatted_prompt {
            let continued = format!("{prompt}{output}");
            engine.generate_streaming_raw_with_sampler(
                &continued,
                max_tokens.saturating_sub(max_think_tokens),
                profile.sampler,
                |token| {
                    output.push_str(token);
                    emit(token);
                    true
                },
            )?;
        } else {
            let continuation_prompt = engine.format_chat_history(
                system_prompt,
                &[super::engine::AgentMessage {
                    role: "user".to_string(),
                    content: continuation,
                }],
            )?;
            engine.generate_streaming_raw_with_sampler(
                &continuation_prompt,
                max_tokens.saturating_sub(max_think_tokens),
                profile.sampler,
                |token| {
                    output.push_str(token);
                    emit(token);
                    true
                },
            )?;
        }
    } else if let Some(stats) = phase1_stats {
        let ctx_size = engine.context_size();
        let was_cut_off = stats.completion_tokens > 0
            && stats.prompt_tokens + stats.completion_tokens + 10 >= ctx_size;

        if was_cut_off {
            let tail_start = output.len().saturating_sub(800);
            let tail_start = utf8::find_boundary_forward(&output, tail_start);
            let tail = &output[tail_start..];
            let continuation_msg = prompts::continuation_prompt(tail);

            let continuation_prompt = engine.format_chat_history(
                system_prompt,
                &[super::engine::AgentMessage {
                    role: "user".to_string(),
                    content: continuation_msg,
                }],
            )?;
            engine.generate_streaming_raw_with_sampler(
                &continuation_prompt,
                max_tokens,
                profile.sampler,
                |token| {
                    output.push_str(token);
                    emit(token);
                    true
                },
            )?;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medium_preserves_the_previous_hardcoded_budget() {
        assert_eq!(ThinkingEffort::Medium.think_token_budget(), 1024);
        assert_eq!(ThinkingEffort::Medium.max_tokens(), 4096);
        assert!(ThinkingEffort::Medium.system_suffix().is_empty());
    }

    #[test]
    fn extra_high_allows_a_longer_think_block_than_low() {
        assert!(
            ThinkingEffort::ExtraHigh.think_token_budget()
                > ThinkingEffort::Low.think_token_budget()
        );
        assert!(ThinkingEffort::ExtraHigh.max_tokens() > ThinkingEffort::Low.max_tokens());
    }

    #[test]
    fn unknown_json_does_not_silently_become_medium() {
        let parsed: Result<ThinkingEffort, _> = serde_json::from_str("\"ludicrous\"");
        assert!(parsed.is_err());
    }

    #[test]
    fn snake_case_json_round_trips() {
        let json = serde_json::to_string(&ThinkingEffort::ExtraHigh).unwrap();
        assert_eq!(json, "\"extra_high\"");
        let parsed: ThinkingEffort = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ThinkingEffort::ExtraHigh);
    }
}
