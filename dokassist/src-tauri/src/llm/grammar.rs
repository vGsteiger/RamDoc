//! GBNF grammars that constrain structured model output (#555).
//!
//! Small local models are the most likely to emit malformed JSON, unknown
//! tool names or prose around a tool call. A llama.cpp grammar sampler masks
//! every token that would leave the grammar, so the constrained part of the
//! output is well-formed by construction.
//!
//! * [`Grammar::tool_call`]: the agent's tool probe. Lazy: free text stays
//!   unconstrained (the probe's stop rule ends it), and only once the output
//!   opens `<tool_call>` where a call is expected must the rest be
//!   `{"name": <a tool of this scope>, "args": {…}}</tool_call>`.
//! * [`Grammar::file_metadata`]: metadata extraction, the `FileMetadata`
//!   JSON object.
//!
//! The grammar is about reliability, not security: tool dispatch still
//! validates scope and arguments.

use super::chat_format::ChatFamily;
use super::harness::SamplerConfig;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::GrammarError;

const ROOT: &str = "root";

/// JSON strings and whitespace, after llama.cpp's `grammars/json.gbnf`. Every
/// rule consumes its own trailing whitespace, and whitespace is bounded so a
/// greedy decoder cannot loop on it.
const JSON_STRING_RULES: &str = r#"string ::= "\"" char* "\"" ws
char ::= [^"\\\x7F\x00-\x1F] | "\\" ( ["\\/bfnrt] | "u" [0-9a-fA-F]{4} )
ws ::= | " " | "\n" [ \t]{0,20}"#;

/// Arbitrary JSON values, on top of [`JSON_STRING_RULES`].
const JSON_VALUE_RULES: &str = r#"value ::= object | array | string | number | ("true" | "false" | "null") ws
object ::= "{" ws ( string ":" ws value ( "," ws string ":" ws value )* )? "}" ws
array ::= "[" ws ( value ( "," ws value )* )? "]" ws
number ::= "-"? ( [0-9] | [1-9] [0-9]{0,15} ) ( "." [0-9]+ )? ( [eE] [-+]? [0-9]{1,15} )? ws"#;

/// Reasoning the model may emit before the constrained part, in the raw
/// (un-normalised) form the grammar sees: a plain `<think>` block, a Gemma 4
/// thought channel, or a Harmony analysis message followed by the header of
/// the final message. Reasoning that is still open never matches, so a
/// `<tool_call>` or `{` written *inside* reasoning does not trigger the
/// grammar.
const REASONING_PREAMBLE: &str = r"(?:\s*(?:<think>[\s\S]*?</think>|<\|channel>[\s\S]*?<channel\|>|<\|channel\|>analysis<\|message\|>[\s\S]*?<\|end\|>\s*<\|start\|>assistant))*\s*(?:<\|channel\|>final<\|message\|>)?\s*";

/// The fields of `FileMetadata` in the order the extraction prompt lists
/// them. `true` marks a string array, `false` a nullable string.
const METADATA_FIELDS: &[(&str, bool)] = &[
    ("document_type", false),
    ("date", false),
    ("author", false),
    ("diagnoses", true),
    ("medications", true),
    ("summary", false),
];

/// A GBNF grammar plus when it starts to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grammar {
    gbnf: String,
    /// Empty: the grammar constrains the output from the first token.
    /// Otherwise it is lazy: unconstrained until one of these regexes matches
    /// the whole output so far, and then applied from the regex's first
    /// capture group on (llama.cpp `grammar_lazy_patterns`).
    trigger_patterns: Vec<String>,
}

impl Grammar {
    /// Lazy grammar for the agent's tool probe.
    ///
    /// It triggers on a `<tool_call>` at the position the probe's stop rule
    /// accepts one: at the start of the output, after closed reasoning, or
    /// behind a Markdown code fence. `starts_in_think` is set when the prompt
    /// already opened a `<think>` block, so the output may begin mid-reasoning.
    pub fn tool_call(tool_names: &[&str], starts_in_think: bool) -> Self {
        let names = tool_names
            .iter()
            .map(|name| gbnf_literal(&json_string(name)))
            .collect::<Vec<_>>()
            .join(" | ");
        let gbnf = format!(
            "{ROOT} ::= \"<tool_call>\" ws \"{{\" ws \"\\\"name\\\"\" ws \":\" ws tool-name \",\" ws \
             \"\\\"args\\\"\" ws \":\" ws object \"}}\" ws \"</tool_call>\"\n\
             tool-name ::= ( {names} ) ws\n\
             {JSON_VALUE_RULES}\n\
             {JSON_STRING_RULES}\n"
        );
        let open_reasoning = if starts_in_think {
            r"(?:[\s\S]*?</think>)?"
        } else {
            ""
        };
        let pattern = format!(
            r"^{open_reasoning}{REASONING_PREAMBLE}(?:```[A-Za-z0-9]*\s*)?(<tool_call>)[\s\S]*$"
        );
        Self {
            gbnf,
            trigger_patterns: vec![pattern],
        }
    }

    /// Grammar for the `FileMetadata` JSON object of metadata extraction.
    ///
    /// Families whose output is plain text are constrained from the first
    /// token, which also keeps a model that reasons despite the switch from
    /// writing a `<think>` block. Gemma 4 and Harmony may need native channel
    /// tokens before the answer (Harmony cannot switch reasoning off), so
    /// their grammar is lazy and starts at the object's `{` after any closed
    /// reasoning.
    pub fn file_metadata(family: ChatFamily) -> Self {
        let mut root = format!("{ROOT} ::= \"{{\" ws");
        for (index, (name, is_array)) in METADATA_FIELDS.iter().enumerate() {
            if index > 0 {
                root.push_str(" \",\" ws");
            }
            let value = if *is_array {
                "string-array"
            } else {
                "nullable-string"
            };
            root.push_str(&format!(
                " {} ws \":\" ws {value}",
                gbnf_literal(&json_string(name))
            ));
        }
        root.push_str(" \"}\"");
        let gbnf = format!(
            "{root}\n\
             nullable-string ::= string | \"null\" ws\n\
             string-array ::= \"[\" ws ( string ( \",\" ws string )* )? \"]\" ws\n\
             {JSON_STRING_RULES}\n"
        );
        let trigger_patterns = match family {
            ChatFamily::Native => Vec::new(),
            ChatFamily::Gemma4 | ChatFamily::Harmony => vec![format!(
                r"^{REASONING_PREAMBLE}(?:```(?:json)?\s*)?(\{{)[\s\S]*$"
            )],
        };
        Self {
            gbnf,
            trigger_patterns,
        }
    }

    pub fn gbnf(&self) -> &str {
        &self.gbnf
    }

    pub fn is_lazy(&self) -> bool {
        !self.trigger_patterns.is_empty()
    }

    fn sampler(&self, model: &LlamaModel) -> Result<LlamaSampler, GrammarError> {
        if self.is_lazy() {
            LlamaSampler::grammar_lazy_patterns(
                model,
                self.gbnf(),
                ROOT,
                &self.trigger_patterns,
                &[],
            )
        } else {
            LlamaSampler::grammar(model, self.gbnf(), ROOT)
        }
    }
}

/// The sampler chain for one generation: the grammar (if any) first, so the
/// penalties, temperature and truncation samplers only see tokens the grammar
/// allows, then `config`'s usual chain.
///
/// A grammar llama.cpp rejects is logged and dropped; the generation then runs
/// unconstrained, as it did before grammars existed, and the caller's parser
/// still validates the output.
pub(crate) fn sampler_chain(
    model: &LlamaModel,
    config: SamplerConfig,
    grammar: Option<&Grammar>,
) -> LlamaSampler {
    let base = config.build(model.n_vocab());
    let Some(grammar) = grammar else {
        return base;
    };
    match grammar.sampler(model) {
        Ok(constraint) => LlamaSampler::chain_simple([constraint, base]),
        Err(error) => {
            log::warn!("Grammar rejected by llama.cpp, generating unconstrained: {error}");
            base
        }
    }
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("serialising a string cannot fail")
}

/// `value` as a GBNF string literal.
fn gbnf_literal(value: &str) -> String {
    let mut literal = String::with_capacity(value.len() + 2);
    literal.push('"');
    for ch in value.chars() {
        match ch {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7F => {
                literal.push_str(&format!("\\x{:02X}", c as u32));
            }
            c => literal.push(c),
        }
    }
    literal.push('"');
    literal
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    /// Rule name → names of the rules its body references.
    ///
    /// A structural check of generated GBNF: one rule per line, string
    /// literals and character classes closed, parentheses balanced. llama.cpp
    /// can only parse a grammar against a loaded vocabulary, so the
    /// model-backed test in `engine.rs` covers the real parser.
    fn parse_rules(gbnf: &str) -> Result<BTreeMap<String, BTreeSet<String>>, String> {
        let mut rules = BTreeMap::new();
        for line in gbnf.lines().filter(|line| !line.trim().is_empty()) {
            let (name, body) = line
                .split_once("::=")
                .ok_or_else(|| format!("not a rule: {line}"))?;
            let name = name.trim();
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return Err(format!("bad rule name: {name:?}"));
            }
            let refs = body_references(body)?;
            if rules.insert(name.to_string(), refs).is_some() {
                return Err(format!("rule {name} defined twice"));
            }
        }
        Ok(rules)
    }

    fn body_references(body: &str) -> Result<BTreeSet<String>, String> {
        let mut refs = BTreeSet::new();
        let mut depth = 0_i32;
        let mut chars = body.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '"' | '[' => {
                    let close = if ch == '"' { '"' } else { ']' };
                    loop {
                        match chars.next() {
                            Some('\\') => {
                                chars.next().ok_or("escape at end of body")?;
                            }
                            Some(c) if c == close => break,
                            Some(_) => {}
                            None => return Err(format!("unclosed {ch} in {body}")),
                        }
                    }
                }
                '{' => {
                    for c in chars.by_ref() {
                        if c == '}' {
                            break;
                        }
                        if !(c.is_ascii_digit() || c == ',') {
                            return Err(format!("bad repetition in {body}"));
                        }
                    }
                }
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth < 0 {
                        return Err(format!("unbalanced ) in {body}"));
                    }
                }
                c if c.is_ascii_alphabetic() => {
                    let mut name = c.to_string();
                    while let Some(&next) = chars.peek() {
                        if next.is_ascii_alphanumeric() || next == '-' {
                            name.push(next);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    refs.insert(name);
                }
                ' ' | '|' | '*' | '+' | '?' => {}
                other => return Err(format!("unexpected {other:?} in {body}")),
            }
        }
        if depth != 0 {
            return Err(format!("unbalanced ( in {body}"));
        }
        Ok(refs)
    }

    fn assert_well_formed(gbnf: &str) {
        let rules = parse_rules(gbnf).unwrap();
        assert!(rules.contains_key(ROOT), "no root rule");
        for (name, refs) in &rules {
            for referenced in refs {
                assert!(
                    rules.contains_key(referenced),
                    "{name} references undefined rule {referenced}"
                );
            }
        }
        // Every rule is reachable from root.
        let mut reached = BTreeSet::from([ROOT.to_string()]);
        let mut pending = vec![ROOT.to_string()];
        while let Some(name) = pending.pop() {
            for referenced in &rules[&name] {
                if reached.insert(referenced.clone()) {
                    pending.push(referenced.clone());
                }
            }
        }
        assert_eq!(reached.len(), rules.len(), "unreachable rules in {gbnf}");
    }

    /// The JSON-quoted names the `tool-name` rule allows.
    fn allowed_tool_names(grammar: &Grammar) -> BTreeSet<String> {
        let line = grammar
            .gbnf()
            .lines()
            .find(|line| line.starts_with("tool-name ::="))
            .unwrap();
        let mut names = BTreeSet::new();
        let mut rest = line;
        // Each alternative is a literal `"\"name\""`.
        while let Some(start) = rest.find("\"\\\"") {
            let after = &rest[start + 3..];
            let end = after.find("\\\"\"").unwrap();
            names.insert(after[..end].to_string());
            rest = &after[end + 3..];
        }
        names
    }

    #[test]
    fn tool_grammar_allows_exactly_the_given_names() {
        let grammar = Grammar::tool_call(&["get_patient", "search"], false);
        assert_eq!(
            allowed_tool_names(&grammar),
            BTreeSet::from(["get_patient".to_string(), "search".to_string()])
        );
        assert!(grammar.is_lazy());
        assert_well_formed(grammar.gbnf());
    }

    #[test]
    fn tool_grammar_shape_matches_the_parser() {
        let gbnf = Grammar::tool_call(&["list_patients"], false)
            .gbnf()
            .to_string();
        let root = gbnf.lines().next().unwrap();
        assert!(root.starts_with(r#"root ::= "<tool_call>""#));
        assert!(root.contains(r#""\"name\"""#));
        assert!(root.contains(r#""\"args\"" ws ":" ws object"#));
        assert!(root.ends_with(r#""</tool_call>""#));
    }

    #[test]
    fn gbnf_literals_escape_quotes_backslashes_and_controls() {
        assert_eq!(gbnf_literal("plain_name"), r#""plain_name""#);
        assert_eq!(gbnf_literal(r#"a"b"#), r#""a\"b""#);
        assert_eq!(gbnf_literal(r"a\b"), r#""a\\b""#);
        assert_eq!(gbnf_literal("a\nb\tc"), r#""a\nb\tc""#);
        assert_eq!(gbnf_literal("\u{1}"), r#""\x01""#);
        assert_eq!(
            gbnf_literal(&json_string("get_patient")),
            r#""\"get_patient\"""#
        );
        // A name needing JSON escapes stays a single, well-formed literal.
        let grammar = Grammar::tool_call(&[r#"odd"na\me"#], false);
        assert_well_formed(grammar.gbnf());
        assert!(grammar.gbnf().contains(r#""\"odd\\\"na\\\\me\"""#));
    }

    #[test]
    fn tool_trigger_respects_reasoning_opened_by_the_prompt() {
        let closed = Grammar::tool_call(&["search"], false);
        let open = Grammar::tool_call(&["search"], true);
        for grammar in [&closed, &open] {
            let pattern = &grammar.trigger_patterns[0];
            assert!(pattern.starts_with('^') && pattern.ends_with('$'));
            assert!(pattern.contains("(<tool_call>)"));
        }
        assert!(!closed.trigger_patterns[0].starts_with(r"^(?:[\s\S]*?</think>)?"));
        assert!(open.trigger_patterns[0].starts_with(r"^(?:[\s\S]*?</think>)?"));
    }

    #[test]
    fn metadata_grammar_is_well_formed_and_lists_every_field() {
        let grammar = Grammar::file_metadata(ChatFamily::Native);
        assert!(!grammar.is_lazy());
        assert_well_formed(grammar.gbnf());
        let root = grammar.gbnf().lines().next().unwrap();
        let mut last = 0;
        for (name, is_array) in METADATA_FIELDS {
            let key = format!(r#""\"{name}\"" ws ":" ws "#);
            let at = root.find(&key).unwrap_or_else(|| panic!("{name} missing"));
            assert!(at > last, "{name} out of order");
            last = at;
            let value = &root[at + key.len()..];
            let expected = if *is_array {
                "string-array"
            } else {
                "nullable-string"
            };
            assert!(value.starts_with(expected), "{name} has the wrong type");
        }
    }

    #[test]
    fn metadata_fields_match_the_struct() {
        let metadata = super::super::extract::FileMetadata {
            document_type: None,
            date: None,
            author: None,
            diagnoses: Vec::new(),
            medications: Vec::new(),
            summary: None,
        };
        let value = serde_json::to_value(&metadata).unwrap();
        let keys: BTreeSet<_> = value.as_object().unwrap().keys().cloned().collect();
        let fields: BTreeSet<_> = METADATA_FIELDS.iter().map(|(n, _)| n.to_string()).collect();
        assert_eq!(keys, fields);
        for (name, is_array) in METADATA_FIELDS {
            assert_eq!(value[*name].is_array(), *is_array, "{name}");
        }
    }

    #[test]
    fn metadata_grammar_is_lazy_only_for_channel_families() {
        assert!(!Grammar::file_metadata(ChatFamily::Native).is_lazy());
        for family in [ChatFamily::Gemma4, ChatFamily::Harmony] {
            let grammar = Grammar::file_metadata(family);
            assert!(grammar.is_lazy());
            assert!(grammar.trigger_patterns[0].contains(r"(\{)"));
            assert_eq!(
                grammar.gbnf(),
                Grammar::file_metadata(ChatFamily::Native).gbnf()
            );
        }
    }
}
