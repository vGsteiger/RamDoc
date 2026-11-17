//! Model-family chat formatting and reasoning normalisation.
//!
//! llama.cpp's `llama_chat_apply_template` only knows a fixed list of
//! templates, detected by substring heuristics. Two families in the catalog
//! fall outside what that path handles well:
//!
//! * **Gemma 4** (`<|turn>role … <turn|>`) is not detected at all, so applying
//!   its template fails outright.
//! * **gpt-oss** (Harmony) is detected, but the built-in formatter omits the
//!   system header that declares the reasoning level and valid channels.
//!
//! Both families also mark reasoning with *control* tokens rather than the
//! plain-text `<think>` tags Qwen uses. Control tokens are dropped when decoded
//! without `special`, which leaks the bare channel names ("analysis…",
//! "thought…") into clinical output. The engine therefore decodes special
//! tokens for these families and passes each piece through a
//! [`ReasoningNormalizer`], which rewrites the native markers into canonical
//! `<think>…</think>`. Everything downstream — the think budget, tool-call
//! extraction and the frontend's think stripping — keeps a single format.

use super::engine::AgentMessage;
use super::thinking::ThinkingEffort;

/// Chat dialect of the loaded GGUF, detected from its embedded template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatFamily {
    /// Gemma 4: `<|turn>role\n…<turn|>`, reasoning in `<|channel>thought…<channel|>`.
    Gemma4,
    /// gpt-oss Harmony: `<|start|>role<|message|>…<|end|>`, reasoning in the
    /// `analysis` channel, answer in `final`.
    Harmony,
    /// Everything llama.cpp formats itself (ChatML/Qwen, Phi, Gemma 3, …).
    /// Reasoning, if any, is plain-text `<think>`.
    Native,
}

/// Canonical reasoning markers seen by every caller of the engine.
pub const THINK_OPEN: &str = "<think>";
pub const THINK_CLOSE: &str = "</think>";

const GEMMA_CHANNEL_OPEN: &str = "<|channel>";
const GEMMA_CHANNEL_CLOSE: &str = "<channel|>";
const GEMMA_THINK_SWITCH: &str = "<|think|>";

const HARMONY_START: &str = "<|start|>";
const HARMONY_CHANNEL: &str = "<|channel|>";
const HARMONY_MESSAGE: &str = "<|message|>";
const HARMONY_END: &str = "<|end|>";
const HARMONY_FINAL_PREFIX: &str = "<|channel|>final<|message|>";

impl ChatFamily {
    pub fn detect(template: &str) -> Self {
        if template.contains("<|turn>") && template.contains("<turn|>") {
            Self::Gemma4
        } else if template.contains(HARMONY_START) && template.contains(HARMONY_CHANNEL) {
            Self::Harmony
        } else {
            Self::Native
        }
    }

    /// Whether reasoning markers are control tokens that must be decoded with
    /// `special = true` to be visible at all.
    pub fn decodes_special_tokens(self) -> bool {
        matches!(self, Self::Gemma4 | Self::Harmony)
    }

    /// Whether the effort hint should talk about `<think>` blocks. Families
    /// with a native reasoning switch get that switch instead; telling Gemma
    /// or gpt-oss to write `<think>` would only produce literal tags.
    pub fn uses_plain_think_tags(self) -> bool {
        matches!(self, Self::Native)
    }

    /// Assistant prefix that disables reasoning for one turn, appended after
    /// the generation prompt. For `<think>` templates this is the empty think
    /// block that Qwen-style templates render for `enable_thinking=False`.
    /// Gemma 4 is switched off in [`ChatFamily::format`] at Low effort, and
    /// Harmony has no off switch (the lowest level is `Reasoning: low`).
    pub fn reasoning_off_prefix(self, template_uses_think_tags: bool) -> &'static str {
        match self {
            Self::Native if template_uses_think_tags => "<think>\n\n</think>\n\n",
            _ => "",
        }
    }

    pub fn normalizer(self) -> ReasoningNormalizer {
        ReasoningNormalizer::new(self)
    }

    /// Format a conversation for families llama.cpp cannot template correctly.
    /// Returns `None` for [`ChatFamily::Native`]; the caller uses llama.cpp.
    pub fn format(
        self,
        system_prompt: &str,
        messages: &[AgentMessage],
        effort: ThinkingEffort,
    ) -> Option<String> {
        match self {
            Self::Gemma4 => Some(format_gemma4(system_prompt, messages, effort)),
            Self::Harmony => Some(format_harmony(system_prompt, messages, effort)),
            Self::Native => None,
        }
    }

    /// Render canonical assistant output back into the model's native form so
    /// it can be appended to a formatted prompt and continued, for example
    /// after the think budget force-closes reasoning.
    pub fn native_assistant_prefix(self, canonical: &str) -> String {
        match self {
            Self::Native => canonical.to_string(),
            Self::Gemma4 => {
                let (reasoning, answer) = split_canonical(canonical);
                match reasoning {
                    Some(r) => {
                        format!("{GEMMA_CHANNEL_OPEN}thought\n{r}{GEMMA_CHANNEL_CLOSE}{answer}")
                    }
                    None => answer.to_string(),
                }
            }
            Self::Harmony => {
                let (reasoning, answer) = split_canonical(canonical);
                match reasoning {
                    // An unclosed think block continues inside `analysis`.
                    Some(r) if !canonical.contains(THINK_CLOSE) => {
                        format!("{HARMONY_CHANNEL}analysis{HARMONY_MESSAGE}{r}")
                    }
                    Some(r) => format!(
                        "{HARMONY_CHANNEL}analysis{HARMONY_MESSAGE}{r}{HARMONY_END}\
                         {HARMONY_START}assistant{HARMONY_FINAL_PREFIX}{answer}"
                    ),
                    None => format!("{HARMONY_FINAL_PREFIX}{answer}"),
                }
            }
        }
    }
}

/// Split canonical output into `(reasoning, answer)`. Whitespace that the
/// normaliser inserts after `</think>` is not part of the answer.
fn split_canonical(canonical: &str) -> (Option<&str>, &str) {
    let Some(start) = canonical.find(THINK_OPEN) else {
        return (None, canonical);
    };
    let body = &canonical[start + THINK_OPEN.len()..];
    match body.find(THINK_CLOSE) {
        Some(end) => (
            Some(&body[..end]),
            body[end + THINK_CLOSE.len()..].trim_start(),
        ),
        None => (Some(body), ""),
    }
}

fn gemma_thinking_enabled(effort: ThinkingEffort) -> bool {
    !matches!(effort, ThinkingEffort::Low)
}

/// Mirrors the generation-prompt path of the Gemma 4 chat template for plain
/// text turns. BOS is omitted because the tokenizer adds it.
fn format_gemma4(system_prompt: &str, messages: &[AgentMessage], effort: ThinkingEffort) -> String {
    let mut out = String::new();
    let system = system_prompt.trim();
    let thinking = gemma_thinking_enabled(effort);
    if thinking || !system.is_empty() {
        out.push_str("<|turn>system\n");
        if thinking {
            out.push_str(GEMMA_THINK_SWITCH);
            out.push('\n');
        }
        out.push_str(system);
        out.push_str("<turn|>\n");
    }
    for message in messages {
        let role = match message.role.as_str() {
            "assistant" | "tool_call" => "model",
            _ => "user",
        };
        out.push_str("<|turn>");
        out.push_str(role);
        out.push('\n');
        if role == "model" {
            // The template drops prior reasoning from history.
            let (_, answer) = split_canonical(&message.content);
            out.push_str(answer.trim());
        } else {
            out.push_str(message.content.trim());
        }
        out.push_str("<turn|>\n");
    }
    out.push_str("<|turn>model\n");
    out
}

fn harmony_reasoning_level(effort: ThinkingEffort) -> &'static str {
    match effort {
        ThinkingEffort::Low => "low",
        ThinkingEffort::Medium => "medium",
        ThinkingEffort::High | ThinkingEffort::ExtraHigh => "high",
    }
}

/// Harmony layout from the gpt-oss model card: a system header declaring the
/// reasoning level and valid channels, then the application's instructions as
/// the developer message. Tools are described textually in those
/// instructions, so only `analysis` and `final` are declared.
fn format_harmony(
    system_prompt: &str,
    messages: &[AgentMessage],
    effort: ThinkingEffort,
) -> String {
    let mut out = format!(
        "{HARMONY_START}system{HARMONY_MESSAGE}Reasoning: {}\n\n\
         # Valid channels: analysis, final. Channel must be included for every message.{HARMONY_END}",
        harmony_reasoning_level(effort)
    );
    let system = system_prompt.trim();
    if !system.is_empty() {
        out.push_str(&format!(
            "{HARMONY_START}developer{HARMONY_MESSAGE}# Instructions\n\n{system}{HARMONY_END}"
        ));
    }
    for message in messages {
        match message.role.as_str() {
            "assistant" | "tool_call" => {
                let (_, answer) = split_canonical(&message.content);
                out.push_str(&format!(
                    "{HARMONY_START}assistant{HARMONY_FINAL_PREFIX}{}{HARMONY_END}",
                    answer.trim()
                ));
            }
            _ => out.push_str(&format!(
                "{HARMONY_START}user{HARMONY_MESSAGE}{}{HARMONY_END}",
                message.content.trim()
            )),
        }
    }
    out.push_str(HARMONY_START);
    out.push_str("assistant");
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NormState {
    /// Ordinary text; passes through.
    Text,
    /// Inside a Harmony `<|start|>` role header; swallowed.
    RoleHeader,
    /// Inside a channel header (`analysis`, `final`, `thought`, …); collected.
    ChannelHeader,
}

/// Streaming rewriter from native reasoning markers to canonical `<think>`.
///
/// Pieces may split markers arbitrarily, so a suffix that could still become
/// a marker is held back until the next piece or [`ReasoningNormalizer::finish`].
#[derive(Debug, Clone)]
pub struct ReasoningNormalizer {
    family: ChatFamily,
    state: NormState,
    pending: String,
    header: String,
    in_think: bool,
}

impl ReasoningNormalizer {
    pub fn new(family: ChatFamily) -> Self {
        Self {
            family,
            state: NormState::Text,
            pending: String::new(),
            header: String::new(),
            in_think: false,
        }
    }

    fn markers(&self) -> &'static [&'static str] {
        match self.family {
            ChatFamily::Gemma4 => &[GEMMA_CHANNEL_OPEN, GEMMA_CHANNEL_CLOSE],
            ChatFamily::Harmony => &[HARMONY_START, HARMONY_CHANNEL, HARMONY_MESSAGE, HARMONY_END],
            ChatFamily::Native => &[],
        }
    }

    /// Feed one decoded piece; returns the canonical text to emit (possibly empty).
    pub fn push(&mut self, piece: &str) -> String {
        if self.family == ChatFamily::Native {
            return piece.to_string();
        }
        self.pending.push_str(piece);
        let mut out = String::new();
        loop {
            let markers = self.markers();
            let next = markers
                .iter()
                .filter_map(|m| self.pending.find(m).map(|at| (at, *m)))
                .min_by_key(|(at, _)| *at);
            match next {
                Some((at, marker)) => {
                    let before: String = self.pending[..at].to_string();
                    self.consume_text(&before, &mut out);
                    self.pending.drain(..at + marker.len());
                    self.on_marker(marker, &mut out);
                }
                None => {
                    // Hold back the longest suffix that is a prefix of a marker.
                    let keep = markers
                        .iter()
                        .map(|m| partial_suffix_len(&self.pending, m))
                        .max()
                        .unwrap_or(0);
                    let flush_to = self.pending.len() - keep;
                    let text: String = self.pending[..flush_to].to_string();
                    self.consume_text(&text, &mut out);
                    self.pending.drain(..flush_to);
                    return out;
                }
            }
        }
    }

    /// Flush anything held back at end of generation.
    pub fn finish(&mut self) -> String {
        let mut out = String::new();
        let rest = std::mem::take(&mut self.pending);
        self.consume_text(&rest, &mut out);
        out
    }

    fn consume_text(&mut self, text: &str, out: &mut String) {
        if text.is_empty() {
            return;
        }
        match self.state {
            NormState::Text => out.push_str(text),
            NormState::RoleHeader => {}
            NormState::ChannelHeader => {
                self.header.push_str(text);
                // Gemma ends the channel name with a newline, not a marker.
                if self.family == ChatFamily::Gemma4 {
                    if let Some(nl) = self.header.find('\n') {
                        let rest = self.header[nl + 1..].to_string();
                        self.header.clear();
                        self.state = NormState::Text;
                        self.open_think(out);
                        out.push_str(&rest);
                    }
                }
            }
        }
    }

    fn open_think(&mut self, out: &mut String) {
        if !self.in_think {
            out.push_str(THINK_OPEN);
            self.in_think = true;
        }
    }

    fn close_think(&mut self, out: &mut String) {
        if self.in_think {
            out.push_str(THINK_CLOSE);
            out.push_str("\n\n");
            self.in_think = false;
        }
    }

    fn on_marker(&mut self, marker: &str, out: &mut String) {
        match (self.family, marker) {
            (ChatFamily::Gemma4, GEMMA_CHANNEL_OPEN) => {
                self.header.clear();
                self.state = NormState::ChannelHeader;
            }
            (ChatFamily::Gemma4, GEMMA_CHANNEL_CLOSE) => {
                if self.state == NormState::ChannelHeader {
                    // Empty thought channel with no newline after its name.
                    self.header.clear();
                    self.state = NormState::Text;
                    self.open_think(out);
                }
                self.close_think(out);
            }
            (ChatFamily::Harmony, HARMONY_START) => self.state = NormState::RoleHeader,
            (ChatFamily::Harmony, HARMONY_CHANNEL) => {
                self.header.clear();
                self.state = NormState::ChannelHeader;
            }
            (ChatFamily::Harmony, HARMONY_MESSAGE) => {
                if self.state == NormState::ChannelHeader {
                    let channel = self.header.trim().to_string();
                    if channel.starts_with("analysis") {
                        self.open_think(out);
                    } else {
                        self.close_think(out);
                    }
                }
                self.header.clear();
                self.state = NormState::Text;
            }
            // The answer channel header that follows decides whether reasoning
            // closed, so `<|end|>` itself emits nothing.
            (ChatFamily::Harmony, HARMONY_END) => self.state = NormState::Text,
            _ => {}
        }
    }
}

/// Length of the longest proper suffix of `text` that is a prefix of `marker`.
fn partial_suffix_len(text: &str, marker: &str) -> usize {
    let max = marker.len().saturating_sub(1).min(text.len());
    (1..=max)
        .rev()
        .find(|&n| {
            text.is_char_boundary(text.len() - n) && marker.starts_with(&text[text.len() - n..])
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(content: &str) -> AgentMessage {
        AgentMessage {
            role: "user".into(),
            content: content.into(),
        }
    }

    fn normalize(family: ChatFamily, pieces: &[&str]) -> String {
        let mut n = family.normalizer();
        let mut out: String = pieces.iter().map(|p| n.push(p)).collect();
        out.push_str(&n.finish());
        out
    }

    #[test]
    fn reasoning_off_prefills_an_empty_think_block_only_for_think_templates() {
        assert_eq!(
            ChatFamily::Native.reasoning_off_prefix(true),
            "<think>\n\n</think>\n\n"
        );
        assert_eq!(ChatFamily::Native.reasoning_off_prefix(false), "");
        assert_eq!(ChatFamily::Gemma4.reasoning_off_prefix(true), "");
        assert_eq!(ChatFamily::Harmony.reasoning_off_prefix(true), "");
    }

    #[test]
    fn detects_families_from_embedded_templates() {
        assert_eq!(
            ChatFamily::detect("{{- '<|turn>system\\n' -}}…{{- '<turn|>\\n' -}}"),
            ChatFamily::Gemma4
        );
        assert_eq!(
            ChatFamily::detect("<|start|>assistant<|channel|>final<|message|>"),
            ChatFamily::Harmony
        );
        assert_eq!(ChatFamily::detect("<|im_start|>user"), ChatFamily::Native);
        assert_eq!(
            ChatFamily::detect("<start_of_turn>user"),
            ChatFamily::Native
        );
    }

    #[test]
    fn gemma4_prompt_matches_the_official_generation_layout() {
        let prompt = ChatFamily::Gemma4
            .format(" Sys ", &[user(" Hallo ")], ThinkingEffort::Low)
            .unwrap();
        assert_eq!(
            prompt,
            "<|turn>system\nSys<turn|>\n<|turn>user\nHallo<turn|>\n<|turn>model\n"
        );
    }

    #[test]
    fn gemma4_enables_native_thinking_above_low_effort() {
        let prompt = ChatFamily::Gemma4
            .format("Sys", &[user("Hallo")], ThinkingEffort::High)
            .unwrap();
        assert!(prompt.starts_with("<|turn>system\n<|think|>\nSys<turn|>\n"));
    }

    #[test]
    fn gemma4_history_drops_prior_reasoning_and_maps_assistant_to_model() {
        let history = [
            user("Frage"),
            AgentMessage {
                role: "assistant".into(),
                content: "<think>intern</think>\n\nAntwort".into(),
            },
            user("Weiter"),
        ];
        let prompt = ChatFamily::Gemma4
            .format("", &history, ThinkingEffort::Low)
            .unwrap();
        assert!(prompt.contains("<|turn>model\nAntwort<turn|>\n"));
        assert!(!prompt.contains("intern"));
        assert!(!prompt.contains("<|turn>system"));
    }

    #[test]
    fn harmony_prompt_declares_reasoning_level_and_developer_instructions() {
        let prompt = ChatFamily::Harmony
            .format(
                "Sie sind Assistent.",
                &[user("Hallo")],
                ThinkingEffort::ExtraHigh,
            )
            .unwrap();
        assert!(prompt.starts_with(
            "<|start|>system<|message|>Reasoning: high\n\n# Valid channels: analysis, final."
        ));
        assert!(prompt
            .contains("<|start|>developer<|message|># Instructions\n\nSie sind Assistent.<|end|>"));
        assert!(prompt.ends_with("<|start|>user<|message|>Hallo<|end|><|start|>assistant"));
    }

    #[test]
    fn harmony_channels_become_canonical_think_blocks() {
        let out = normalize(
            ChatFamily::Harmony,
            &[
                "<|channel|>",
                "analysis",
                "<|message|>",
                "Prüfe Daten.",
                "<|end|>",
                "<|start|>",
                "assistant",
                "<|channel|>",
                "final",
                "<|message|>",
                "Bericht",
            ],
        );
        assert_eq!(out, "<think>Prüfe Daten.</think>\n\nBericht");
    }

    #[test]
    fn harmony_final_only_output_has_no_think_block() {
        let out = normalize(
            ChatFamily::Harmony,
            &["<|channel|>final<|message|>", "Nur Antwort"],
        );
        assert_eq!(out, "Nur Antwort");
    }

    #[test]
    fn gemma4_thought_channel_becomes_canonical_think_block() {
        let out = normalize(
            ChatFamily::Gemma4,
            &[
                "<|channel>",
                "thought",
                "\n",
                "Überlege.",
                "<channel|>",
                "Antwort",
            ],
        );
        assert_eq!(out, "<think>Überlege.</think>\n\nAntwort");
    }

    #[test]
    fn markers_split_across_pieces_are_still_recognised() {
        let out = normalize(
            ChatFamily::Harmony,
            &[
                "<|chan",
                "nel|>anal",
                "ysis<|mess",
                "age|>x<|e",
                "nd|><|start|>assistant<|channel|>final<|message|>y",
            ],
        );
        assert_eq!(out, "<think>x</think>\n\ny");
    }

    #[test]
    fn text_resembling_a_marker_prefix_is_flushed_at_finish() {
        assert_eq!(normalize(ChatFamily::Gemma4, &["a <|chan"]), "a <|chan");
    }

    #[test]
    fn native_family_passes_pieces_through() {
        assert_eq!(
            normalize(ChatFamily::Native, &["<think>", "x", "</think>", "y"]),
            "<think>x</think>y"
        );
    }

    #[test]
    fn native_prefix_round_trips_a_force_closed_think_block() {
        let canonical = "<think>Plan</think>\n\n";
        assert_eq!(
            ChatFamily::Gemma4.native_assistant_prefix(canonical),
            "<|channel>thought\nPlan<channel|>"
        );
        assert_eq!(
            ChatFamily::Harmony.native_assistant_prefix(canonical),
            "<|channel|>analysis<|message|>Plan<|end|><|start|>assistant<|channel|>final<|message|>"
        );
        assert_eq!(
            ChatFamily::Native.native_assistant_prefix(canonical),
            canonical
        );
    }

    #[test]
    fn harmony_prefix_keeps_an_open_analysis_channel_open() {
        assert_eq!(
            ChatFamily::Harmony.native_assistant_prefix("<think>still"),
            "<|channel|>analysis<|message|>still"
        );
    }

    #[test]
    fn normalised_output_round_trips_through_the_native_prefix() {
        let pieces = ["<|channel>", "thought\n", "a", "<channel|>", "b"];
        let canonical = normalize(ChatFamily::Gemma4, &pieces);
        assert_eq!(
            ChatFamily::Gemma4.native_assistant_prefix(&canonical),
            pieces.concat()
        );
    }
}
