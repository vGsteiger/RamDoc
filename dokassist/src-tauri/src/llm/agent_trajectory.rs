//! Multi-turn agent trajectory benchmark (#560).
//!
//! Each case is one agent turn: a user message (optionally after earlier
//! turns), the tool calls the agent is expected to make with canned results,
//! and facts the final answer must or must not contain. "No tool" cases
//! measure the opposite misroute: calling a tool when the answer is already
//! in the prompt or is general knowledge.
//!
//! The runner drives the production loop ([`run_agent_turn`]) with a scripted
//! tool executor instead of the database, so it measures the probe, parser,
//! grammar, history layout and answer path that ship. Results are reported per
//! model family (chat format and sampler family), since #563/#565/#567/#571/#573
//! all changed behaviour per family.
//!
//! Scoring and fixture validation are pure and run in `cargo test --lib`;
//! the model-backed run is behind the `benchmark-harness` feature.

use super::agent::{
    build_system_prompt_for_date, run_agent_turn, scope_tool_names, AgentScope, AgentTurnInput,
    ProbeTrace, ToolCallRequest, MAX_ITERATIONS,
};
use super::benchmark_harness::{score_expected, sha256_path, ClinicalScore, ExpectedAnswer};
use super::context_cache::InferenceSession;
use super::engine::{AgentMessage, LlmEngine};
use super::thinking::ThinkingEffort;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

const TRAJECTORY_JSON: &str =
    include_str!("../../../../benchmarks/agent-trajectory/trajectory-cases.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectorySuite {
    pub schema_version: u32,
    pub suite_id: String,
    pub data_classification: String,
    /// Fixed "today" for the system prompt, so runs are comparable.
    pub today: String,
    pub thinking_effort: ThinkingEffort,
    pub cases: Vec<TrajectoryCase>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaseKind {
    /// The answer needs at least one tool call.
    Tool,
    /// The answer is in the prompt or is general knowledge.
    NoTool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryCase {
    pub id: String,
    pub kind: CaseKind,
    pub scope: AgentScope,
    #[serde(default)]
    pub patient_context: Option<String>,
    #[serde(default)]
    pub history: Vec<HistoryMessage>,
    pub user_message: String,
    /// Whether `expected_calls` must happen in this order. Unordered cases
    /// still require the same calls, as a multiset.
    #[serde(default = "default_ordered")]
    pub ordered: bool,
    pub expected_calls: Vec<ExpectedCall>,
    /// Grounding checks for the final answer (`tool_call` is not allowed).
    pub answer: ExpectedAnswer,
    /// An answer that satisfies `answer`, checked in CI.
    pub reference_answer: String,
}

fn default_ordered() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedCall {
    pub name: String,
    /// Required arguments, matched as a subset (see [`args_match`]).
    pub args: Value,
    /// Canned tool result returned to the agent.
    pub result: Value,
}

pub fn suite() -> Result<TrajectorySuite, String> {
    serde_json::from_str(TRAJECTORY_JSON)
        .map_err(|error| format!("parse trajectory benchmark cases: {error}"))
}

const HISTORY_ROLES: &[&str] = &["user", "assistant", "tool_call", "tool_result"];

/// Validate the embedded suite: every case parses, is synthetic, names only
/// tools of its scope, and its reference answer passes its own checks.
pub fn validate_suite(suite: &TrajectorySuite) -> Result<(), String> {
    if suite.schema_version != 1 {
        return Err("only trajectory schema version 1 is supported".into());
    }
    if suite.data_classification != "synthetic_deidentified" {
        return Err("trajectory cases must be classified synthetic_deidentified".into());
    }
    if !(10..=20).contains(&suite.cases.len()) {
        return Err(format!(
            "trajectory suite should hold 10-20 cases, has {}",
            suite.cases.len()
        ));
    }
    let mut ids = BTreeSet::new();
    let mut covered = BTreeSet::new();
    let mut multi_step = 0usize;
    for case in &suite.cases {
        let fail = |reason: String| Err(format!("trajectory case '{}': {reason}", case.id));
        if !ids.insert(case.id.as_str()) {
            return fail("duplicate id".into());
        }
        if case.id.trim().is_empty() || case.user_message.trim().is_empty() {
            return fail("empty id or user message".into());
        }
        let scope_label = match &case.scope {
            AgentScope::Global => {
                if case.patient_context.is_some() {
                    return fail("global scope cannot carry patient context".into());
                }
                "global"
            }
            AgentScope::Patient { patient_id } => {
                if !patient_id.starts_with("synthetic-") {
                    return fail(format!("patient id '{patient_id}' is not synthetic"));
                }
                if let Some(context) = &case.patient_context {
                    serde_json::from_str::<Value>(context)
                        .map_err(|error| format!("{}: patient context: {error}", case.id))?;
                    if !context.to_lowercase().contains("synthet") {
                        return fail("patient context must identify itself as synthetic".into());
                    }
                }
                "patient"
            }
        };
        covered.insert((scope_label, case.kind));
        for message in &case.history {
            if !HISTORY_ROLES.contains(&message.role.as_str()) {
                return fail(format!("unknown history role '{}'", message.role));
            }
        }
        match (case.kind, case.expected_calls.is_empty()) {
            (CaseKind::Tool, true) => return fail("tool case without expected calls".into()),
            (CaseKind::NoTool, false) => return fail("no_tool case with expected calls".into()),
            _ => {}
        }
        if case.expected_calls.len() > 1 {
            multi_step += 1;
        }
        if case.expected_calls.len() >= MAX_ITERATIONS {
            return fail("more expected calls than the loop allows".into());
        }
        let tools = scope_tool_names(&case.scope);
        for call in &case.expected_calls {
            if !tools.contains(&call.name.as_str()) {
                return fail(format!(
                    "tool '{}' is not available in {scope_label} scope",
                    call.name
                ));
            }
            if !call.args.is_object() {
                return fail(format!("arguments of '{}' must be an object", call.name));
            }
        }
        let answer = &case.answer;
        if answer.tool_call.is_some() || answer.exact.is_some() {
            return fail("answer checks may not use tool_call or exact".into());
        }
        if answer.contains_all.is_empty()
            && answer.contains_any.is_empty()
            && answer.excludes.is_empty()
        {
            return fail("final answer has no grounding checks".into());
        }
        let reference = score_expected(answer, &case.reference_answer);
        if !reference.passed {
            return fail(format!(
                "reference answer fails its checks: {:?}",
                reference.checks
            ));
        }
    }
    for scope in ["patient", "global"] {
        for kind in [CaseKind::Tool, CaseKind::NoTool] {
            if !covered.contains(&(scope, kind)) {
                return Err(format!("no {kind:?} case in {scope} scope"));
            }
        }
    }
    if multi_step < 2 {
        return Err("at least two cases must need more than one tool call".into());
    }
    Ok(())
}

fn folded(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Whether `actual` contains the `expected` arguments. Objects match as
/// subsets; strings match case-insensitively by containment, so a search
/// query "Bruno Beispiel" satisfies "Bruno"; other values must be equal,
/// except that numbers also match their string form ("50" vs 50).
pub fn args_match(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected.iter().all(|(key, value)| {
            actual
                .get(key)
                .is_some_and(|found| args_match(found, value))
        }),
        (Value::Array(actual), Value::Array(expected)) => expected
            .iter()
            .all(|value| actual.iter().any(|found| args_match(found, value))),
        (Value::String(actual), Value::String(expected)) => {
            folded(actual).contains(&folded(expected))
        }
        (Value::String(actual), Value::Number(expected)) => {
            actual.trim().parse::<f64>().ok() == expected.as_f64()
        }
        _ => actual == expected,
    }
}

/// Canned tool results for one case. A call gets the result of the first
/// unused expected call with the same name and matching arguments, then of
/// any unused one with the same name, then (for a repeated call) of the last
/// expected call with that name. Anything else gets an error result, as
/// dispatch would return for a call the fixture does not cover.
pub struct ScriptedTools<'a> {
    calls: &'a [ExpectedCall],
    used: Vec<bool>,
}

impl<'a> ScriptedTools<'a> {
    pub fn new(calls: &'a [ExpectedCall]) -> Self {
        Self {
            calls,
            used: vec![false; calls.len()],
        }
    }

    pub fn execute(&mut self, call: &ToolCallRequest) -> String {
        let same_name = |index: &usize| self.calls[*index].name == call.name;
        let unused = (0..self.calls.len()).filter(|index| !self.used[*index] && same_name(index));
        let picked = unused
            .clone()
            .find(|index| args_match(&call.args, &self.calls[*index].args))
            .or_else(|| unused.clone().next())
            .or_else(|| {
                (0..self.calls.len())
                    .rev()
                    .find(|index| self.used[*index] && same_name(index))
            });
        match picked {
            Some(index) => {
                self.used[index] = true;
                self.calls[index].result.to_string()
            }
            None => serde_json::json!({
                "error": format!(
                    "Testfall: kein Ergebnis für Tool '{}' hinterlegt.",
                    call.name
                )
            })
            .to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Misroute {
    /// A tool was needed, but the agent answered without calling any.
    AnsweredWithoutTool,
    /// No tool was needed, but the agent called one.
    CalledWithoutNeed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryScore {
    pub passed: bool,
    /// Called tool names equal the expected ones (in order, if ordered).
    pub sequence_match: bool,
    pub expected_calls: Vec<String>,
    pub actual_calls: Vec<String>,
    /// Expected calls that were made with all required arguments.
    pub args_correct: usize,
    pub args_total: usize,
    /// Probes that tried a call but produced no valid call of a scope tool:
    /// an unparseable `<tool_call>` block, or an unknown tool name.
    pub malformed_calls: usize,
    pub misroute: Option<Misroute>,
    /// Probes run, i.e. loop iterations.
    pub iterations: usize,
    pub hit_iteration_limit: bool,
    pub probe_tokens: usize,
    /// Final answer checks: required facts present, forbidden facts absent.
    pub grounding: ClinicalScore,
    /// Failed `excludes` checks, i.e. forbidden facts in the answer.
    pub forbidden_facts: usize,
}

fn is_malformed(probe: &ProbeTrace, tools: &[&str]) -> bool {
    match &probe.call {
        Some(call) => !tools.contains(&call.name.as_str()),
        None => probe.output.contains("<tool_call>"),
    }
}

/// Count expected calls matched by an actual call with the same name and the
/// required arguments, each actual call used at most once.
fn count_args_correct(expected: &[ExpectedCall], actual: &[&ToolCallRequest]) -> usize {
    let mut used = vec![false; actual.len()];
    let mut correct = 0;
    for call in expected {
        let found = actual.iter().enumerate().position(|(index, made)| {
            !used[index] && made.name == call.name && args_match(&made.args, &call.args)
        });
        if let Some(index) = found {
            used[index] = true;
            correct += 1;
        }
    }
    correct
}

/// Score one observed trajectory against its case. Pure, so it is unit
/// tested without a model.
pub fn score_trajectory(
    case: &TrajectoryCase,
    probes: &[ProbeTrace],
    final_answer: &str,
) -> TrajectoryScore {
    let tools = scope_tool_names(&case.scope);
    let actual: Vec<&ToolCallRequest> = probes.iter().filter_map(|p| p.call.as_ref()).collect();
    let actual_calls: Vec<String> = actual.iter().map(|call| call.name.clone()).collect();
    let expected_calls: Vec<String> = case
        .expected_calls
        .iter()
        .map(|call| call.name.clone())
        .collect();
    let sequence_match = if case.ordered {
        actual_calls == expected_calls
    } else {
        let mut actual_sorted = actual_calls.clone();
        let mut expected_sorted = expected_calls.clone();
        actual_sorted.sort();
        expected_sorted.sort();
        actual_sorted == expected_sorted
    };
    let args_total = case.expected_calls.len();
    let args_correct = count_args_correct(&case.expected_calls, &actual);
    let malformed_calls = probes.iter().filter(|p| is_malformed(p, &tools)).count();
    let misroute = match case.kind {
        CaseKind::Tool if actual.is_empty() => Some(Misroute::AnsweredWithoutTool),
        CaseKind::NoTool if !actual.is_empty() => Some(Misroute::CalledWithoutNeed),
        _ => None,
    };
    let hit_iteration_limit =
        probes.len() >= MAX_ITERATIONS && probes.last().is_some_and(|p| p.call.is_some());
    let grounding = score_expected(&case.answer, final_answer);
    let forbidden_facts = grounding
        .checks
        .iter()
        .filter(|check| check.check.starts_with("excludes:") && !check.passed)
        .count();
    TrajectoryScore {
        passed: sequence_match
            && args_correct == args_total
            && malformed_calls == 0
            && misroute.is_none()
            && !hit_iteration_limit
            && grounding.passed,
        sequence_match,
        expected_calls,
        actual_calls,
        args_correct,
        args_total,
        malformed_calls,
        misroute,
        iterations: probes.len(),
        hit_iteration_limit,
        probe_tokens: probes.iter().map(|p| p.generated_tokens).sum(),
        grounding,
        forbidden_facts,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseRun {
    pub case_id: String,
    pub kind: CaseKind,
    pub scope: String,
    pub repetition: usize,
    pub family: String,
    pub elapsed_ms: f64,
    pub probes: Vec<ProbeTrace>,
    pub final_answer: String,
    pub score: Option<TrajectoryScore>,
    pub error: Option<String>,
}

/// Per-family aggregate. Rates are fractions in 0..=1; `None` when the
/// denominator is zero.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FamilySummary {
    pub runs: usize,
    pub runs_passed: usize,
    pub errors: usize,
    pub tool_runs: usize,
    pub no_tool_runs: usize,
    pub sequence_matches: usize,
    pub args_correct: usize,
    pub args_total: usize,
    pub probes: usize,
    pub malformed_calls: usize,
    /// Tool runs answered without any call.
    pub answered_without_tool: usize,
    /// No-tool runs that called a tool.
    pub called_without_need: usize,
    pub grounding_passed: usize,
    pub forbidden_facts: usize,
    pub iteration_limit_hits: usize,
    pub probe_tokens: usize,
    pub pass_rate: Option<f64>,
    pub sequence_match_rate: Option<f64>,
    pub args_accuracy: Option<f64>,
    /// Malformed calls per probe.
    pub malformed_call_rate: Option<f64>,
    /// Misrouted runs (either direction) per scored run.
    pub misroute_rate: Option<f64>,
    pub grounding_rate: Option<f64>,
    pub mean_iterations: Option<f64>,
    pub mean_probe_tokens: Option<f64>,
}

fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then(|| numerator as f64 / denominator as f64)
}

/// Aggregate runs by `family`. Errored runs count in `runs` and `errors`
/// only, since no trajectory was observed.
pub fn summarize_by_family(runs: &[CaseRun]) -> BTreeMap<String, FamilySummary> {
    let mut summaries: BTreeMap<String, FamilySummary> = BTreeMap::new();
    for run in runs {
        let summary = summaries.entry(run.family.clone()).or_default();
        summary.runs += 1;
        let Some(score) = &run.score else {
            summary.errors += 1;
            continue;
        };
        match run.kind {
            CaseKind::Tool => summary.tool_runs += 1,
            CaseKind::NoTool => summary.no_tool_runs += 1,
        }
        summary.runs_passed += usize::from(score.passed);
        summary.sequence_matches += usize::from(score.sequence_match);
        summary.args_correct += score.args_correct;
        summary.args_total += score.args_total;
        summary.probes += score.iterations;
        summary.malformed_calls += score.malformed_calls;
        match score.misroute {
            Some(Misroute::AnsweredWithoutTool) => summary.answered_without_tool += 1,
            Some(Misroute::CalledWithoutNeed) => summary.called_without_need += 1,
            None => {}
        }
        summary.grounding_passed += usize::from(score.grounding.passed);
        summary.forbidden_facts += score.forbidden_facts;
        summary.iteration_limit_hits += usize::from(score.hit_iteration_limit);
        summary.probe_tokens += score.probe_tokens;
    }
    for summary in summaries.values_mut() {
        let scored = summary.tool_runs + summary.no_tool_runs;
        summary.pass_rate = ratio(summary.runs_passed, summary.runs);
        summary.sequence_match_rate = ratio(summary.sequence_matches, scored);
        summary.args_accuracy = ratio(summary.args_correct, summary.args_total);
        summary.malformed_call_rate = ratio(summary.malformed_calls, summary.probes);
        summary.misroute_rate = ratio(
            summary.answered_without_tool + summary.called_without_need,
            scored,
        );
        summary.grounding_rate = ratio(summary.grounding_passed, scored);
        summary.mean_iterations = ratio(summary.probes, scored);
        summary.mean_probe_tokens = ratio(summary.probe_tokens, scored);
    }
    summaries
}

fn percent(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |v| format!("{:.0}%", v * 100.0))
}

fn decimal(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |v| format!("{v:.1}"))
}

/// Markdown table of per-family metrics.
pub fn render_family_table(summaries: &BTreeMap<String, FamilySummary>) -> String {
    let mut out = String::from(
        "| Family | Runs | Pass | Sequence | Args | Malformed/probe | Misroute | \
         Answered w/o tool | Called w/o need | Grounding | Forbidden facts | \
         Mean iterations | Mean probe tokens | Errors |\n\
         | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
    );
    for (family, s) in summaries {
        out.push_str(&format!(
            "| {family} | {} | {} | {} | {} | {} | {} | {}/{} | {}/{} | {} | {} | {} | {} | {} |\n",
            s.runs,
            percent(s.pass_rate),
            percent(s.sequence_match_rate),
            percent(s.args_accuracy),
            percent(s.malformed_call_rate),
            percent(s.misroute_rate),
            s.answered_without_tool,
            s.tool_runs,
            s.called_without_need,
            s.no_tool_runs,
            percent(s.grounding_rate),
            s.forbidden_facts,
            decimal(s.mean_iterations),
            decimal(s.mean_probe_tokens),
            s.errors,
        ));
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryResult {
    pub schema_version: u32,
    pub suite_id: String,
    pub suite_sha256: String,
    pub model_filename: String,
    pub model_sha256: Option<String>,
    pub inference_profile: String,
    pub chat_family: String,
    pub sampler_family: String,
    pub git_commit: Option<String>,
    pub git_dirty: Option<bool>,
    pub repetitions: usize,
    pub runs: Vec<CaseRun>,
    pub families: BTreeMap<String, FamilySummary>,
}

#[derive(Debug)]
struct RunOptions {
    model: PathBuf,
    output: PathBuf,
    profile: String,
    repetitions: usize,
    case_ids: Option<BTreeSet<String>>,
    hash_model: bool,
}

fn parse_run_options(raw: &[String]) -> Result<RunOptions, String> {
    let mut values = BTreeMap::new();
    let mut hash_model = true;
    let mut index = 0;
    while index < raw.len() {
        let key = raw[index]
            .strip_prefix("--")
            .ok_or_else(|| format!("unexpected argument '{}'", raw[index]))?;
        if key == "skip-model-hash" {
            hash_model = false;
            index += 1;
            continue;
        }
        let value = raw
            .get(index + 1)
            .ok_or_else(|| format!("--{key} requires a value"))?;
        values.insert(key.to_string(), value.clone());
        index += 2;
    }
    let required = |name: &str| {
        values
            .get(name)
            .cloned()
            .ok_or_else(|| format!("missing required argument --{name}"))
    };
    let repetitions = match values.get("repetitions") {
        Some(value) => value
            .parse()
            .ok()
            .filter(|count| *count > 0)
            .ok_or_else(|| "--repetitions must be a positive number".to_string())?,
        None => 1,
    };
    Ok(RunOptions {
        model: PathBuf::from(required("model")?),
        output: PathBuf::from(required("output")?),
        profile: values
            .get("profile")
            .cloned()
            .unwrap_or_else(|| "governed".to_string()),
        repetitions,
        case_ids: values.get("cases").map(|ids| {
            ids.split(',')
                .map(|id| id.trim().to_string())
                .filter(|id| !id.is_empty())
                .collect()
        }),
        hash_model,
    })
}

fn git_output(args: &[&str]) -> Option<std::process::Output> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
}

fn scope_label(scope: &AgentScope) -> &'static str {
    match scope {
        AgentScope::Global => "global",
        AgentScope::Patient { .. } => "patient",
    }
}

fn run_case(
    engine: &Arc<LlmEngine>,
    suite: &TrajectorySuite,
    case: &TrajectoryCase,
    repetition: usize,
    family: &str,
) -> CaseRun {
    let patient_id = match &case.scope {
        AgentScope::Patient { patient_id } => Some(patient_id.clone()),
        AgentScope::Global => None,
    };
    let input = AgentTurnInput {
        inference_session: InferenceSession::agent(
            format!("trajectory-{}-{repetition}", case.id),
            patient_id,
            Some("trajectory-fixture-v1".to_string()),
        ),
        scope: case.scope.clone(),
        patient_context: case.patient_context.clone(),
        history: case
            .history
            .iter()
            .map(|message| AgentMessage {
                role: message.role.clone(),
                content: message.content.clone(),
            })
            .collect(),
        user_message: case.user_message.clone(),
        thinking_effort: suite.thinking_effort,
    };
    let system_prompt =
        build_system_prompt_for_date(&case.scope, case.patient_context.as_deref(), &suite.today);
    let mut tools = ScriptedTools::new(&case.expected_calls);
    let started = Instant::now();
    let outcome = run_agent_turn(
        engine,
        None,
        input,
        &system_prompt,
        &mut |call: &ToolCallRequest| -> Result<String, AppError> { Ok(tools.execute(call)) },
        &|_: &str| {},
    );
    let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let mut run = CaseRun {
        case_id: case.id.clone(),
        kind: case.kind,
        scope: scope_label(&case.scope).to_string(),
        repetition,
        family: family.to_string(),
        elapsed_ms,
        probes: Vec::new(),
        final_answer: String::new(),
        score: None,
        error: None,
    };
    match outcome {
        Ok(result) => {
            run.score = Some(score_trajectory(
                case,
                &result.probe_trace,
                &result.final_answer,
            ));
            run.probes = result.probe_trace;
            run.final_answer = result.final_answer;
        }
        Err(error) => run.error = Some(error.to_string()),
    }
    run
}

fn run(options: &RunOptions) -> Result<TrajectoryResult, String> {
    let suite = suite()?;
    validate_suite(&suite)?;
    if let Some(ids) = &options.case_ids {
        let known: BTreeSet<&str> = suite.cases.iter().map(|case| case.id.as_str()).collect();
        let unknown: Vec<&String> = ids
            .iter()
            .filter(|id| !known.contains(id.as_str()))
            .collect();
        if !unknown.is_empty() {
            return Err(format!("unknown cases: {unknown:?}"));
        }
    }
    let model_filename = options
        .model
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "model path has no UTF-8 filename".to_string())?
        .to_string();
    let model_sha256 = if options.hash_model {
        Some(sha256_path(&options.model)?)
    } else {
        None
    };
    let engine = Arc::new(
        LlmEngine::load_with_profile(
            options.model.clone(),
            model_filename.clone(),
            &options.profile,
        )
        .map_err(|error| error.to_string())?,
    );
    let chat_family = format!("{:?}", engine.chat_family());
    let sampler_family = format!("{:?}", engine.sampler_family());
    let family = format!("{sampler_family}/{chat_family}");

    let mut runs = Vec::new();
    for repetition in 0..options.repetitions {
        for case in suite.cases.iter().filter(|case| {
            options
                .case_ids
                .as_ref()
                .is_none_or(|ids| ids.contains(&case.id))
        }) {
            let run = run_case(&engine, &suite, case, repetition, &family);
            println!(
                "{} #{repetition}: {}",
                case.id,
                match (&run.score, &run.error) {
                    (Some(score), _) if score.passed => "pass".to_string(),
                    (Some(score), _) => format!(
                        "fail (calls {:?}, misroute {:?}, grounding {}/{})",
                        score.actual_calls,
                        score.misroute,
                        score.grounding.checks_passed,
                        score.grounding.checks_total
                    ),
                    (None, Some(error)) => format!("error: {error}"),
                    (None, None) => "error".to_string(),
                }
            );
            runs.push(run);
        }
    }

    let git_commit = git_output(&["rev-parse", "HEAD"])
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string());
    let git_dirty = git_output(&["status", "--porcelain", "--untracked-files=no"])
        .map(|output| !output.stdout.is_empty());
    let families = summarize_by_family(&runs);
    Ok(TrajectoryResult {
        schema_version: 1,
        suite_id: suite.suite_id,
        suite_sha256: hex::encode(
            ring::digest::digest(&ring::digest::SHA256, TRAJECTORY_JSON.as_bytes()).as_ref(),
        ),
        model_filename,
        model_sha256,
        inference_profile: options.profile.clone(),
        chat_family,
        sampler_family,
        git_commit,
        git_dirty,
        repetitions: options.repetitions,
        runs,
        families,
    })
}

fn read_result(path: &Path) -> Result<TrajectoryResult, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("read '{}': {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("parse '{}': {error}", path.display()))
}

pub fn cli_main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("validate") => {
            let suite = suite()?;
            validate_suite(&suite)?;
            println!("{}: {} cases valid", suite.suite_id, suite.cases.len());
            Ok(())
        }
        Some("run") => {
            let options = parse_run_options(&args[1..])?;
            let result = run(&options)?;
            if let Some(parent) = options.output.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("create '{}': {error}", parent.display()))?;
            }
            let json = serde_json::to_string_pretty(&result)
                .map_err(|error| format!("serialize result: {error}"))?;
            std::fs::write(&options.output, format!("{json}\n"))
                .map_err(|error| format!("write '{}': {error}", options.output.display()))?;
            println!("\n{}", render_family_table(&result.families));
            println!("wrote {}", options.output.display());
            Ok(())
        }
        Some("report") if args.len() > 1 => {
            // Merge several result files (one per model) into one table.
            let mut runs = Vec::new();
            for path in &args[1..] {
                runs.extend(read_result(Path::new(path))?.runs);
            }
            print!("{}", render_family_table(&summarize_by_family(&runs)));
            Ok(())
        }
        _ => Err(
            "usage: agent-trajectory-benchmark <validate | run --model PATH --output PATH \
             [--profile NAME] [--repetitions N] [--cases ID,ID] [--skip-model-hash] | \
             report RESULT.json...>"
                .into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn case(id: &str) -> TrajectoryCase {
        suite()
            .unwrap()
            .cases
            .into_iter()
            .find(|case| case.id == id)
            .unwrap()
    }

    fn probe(iteration: usize, output: &str, call: Option<(&str, Value)>) -> ProbeTrace {
        ProbeTrace {
            iteration,
            prompt_tokens: 1_000,
            folded: false,
            output: output.to_string(),
            generated_tokens: output.len().max(1),
            call: call.map(|(name, args)| ToolCallRequest {
                name: name.to_string(),
                args,
            }),
        }
    }

    /// The trajectory a perfect agent would produce for `case`.
    fn perfect_probes(case: &TrajectoryCase) -> Vec<ProbeTrace> {
        let mut probes: Vec<ProbeTrace> = case
            .expected_calls
            .iter()
            .enumerate()
            .map(|(index, call)| {
                let text = json!({"name": call.name, "args": call.args});
                probe(
                    index,
                    &format!("<tool_call>{text}</tool_call>"),
                    Some((call.name.as_str(), call.args.clone())),
                )
            })
            .collect();
        probes.push(probe(probes.len(), "D", None));
        probes
    }

    fn run_for(case: &TrajectoryCase, score: TrajectoryScore, family: &str) -> CaseRun {
        CaseRun {
            case_id: case.id.clone(),
            kind: case.kind,
            scope: scope_label(&case.scope).into(),
            repetition: 0,
            family: family.into(),
            elapsed_ms: 1.0,
            probes: Vec::new(),
            final_answer: String::new(),
            score: Some(score),
            error: None,
        }
    }

    #[test]
    fn embedded_suite_is_valid_and_covers_both_scopes_and_kinds() {
        let suite = suite().unwrap();
        validate_suite(&suite).unwrap();
        assert!((10..=20).contains(&suite.cases.len()));
    }

    #[test]
    fn every_expected_tool_exists_in_its_scope() {
        for case in suite().unwrap().cases {
            let tools = scope_tool_names(&case.scope);
            for call in &case.expected_calls {
                assert!(
                    tools.contains(&call.name.as_str()),
                    "{}: {}",
                    case.id,
                    call.name
                );
            }
        }
    }

    #[test]
    fn validation_rejects_a_tool_outside_the_scope() {
        let mut suite = suite().unwrap();
        let patient_case = suite
            .cases
            .iter_mut()
            .find(|case| {
                matches!(case.scope, AgentScope::Patient { .. }) && case.kind == CaseKind::Tool
            })
            .unwrap();
        patient_case.expected_calls[0].name = "list_patients".into();
        let error = validate_suite(&suite).unwrap_err();
        assert!(error.contains("not available in patient scope"), "{error}");

        let mut suite = super::suite().unwrap();
        suite.cases[0].expected_calls[0].name = "delete_everything".into();
        assert!(validate_suite(&suite).is_err());
    }

    #[test]
    fn validation_rejects_a_reference_answer_that_fails_its_checks() {
        let mut suite = suite().unwrap();
        suite.cases[0].reference_answer = "Keine Angaben.".into();
        assert!(validate_suite(&suite)
            .unwrap_err()
            .contains("reference answer"));
    }

    #[test]
    fn a_perfect_trajectory_passes_every_case() {
        for case in suite().unwrap().cases {
            let probes = perfect_probes(&case);
            let score = score_trajectory(&case, &probes, &case.reference_answer);
            assert!(score.passed, "{}: {score:?}", case.id);
            assert_eq!(score.iterations, case.expected_calls.len() + 1);
            assert_eq!(score.args_correct, score.args_total);
        }
    }

    #[test]
    fn answering_without_the_needed_tool_is_a_misroute() {
        let case = case("pt-current-medication");
        let probes = [probe(0, "S", None)];
        let score = score_trajectory(&case, &probes, "Sertralin 100 mg und Quetiapin 25 mg.");
        assert_eq!(score.misroute, Some(Misroute::AnsweredWithoutTool));
        assert!(!score.sequence_match);
        assert!(!score.passed);
        assert_eq!(score.args_correct, 0);
    }

    #[test]
    fn calling_a_tool_without_need_is_a_misroute() {
        let case = case("pt-gp-from-loaded-context");
        let probes = [
            probe(
                0,
                "<tool_call>…</tool_call>",
                Some((
                    "get_patient",
                    json!({"patient_id": "synthetic-patient-560-a"}),
                )),
            ),
            probe(1, "D", None),
        ];
        let score = score_trajectory(&case, &probes, "Dr. med. Lena Muster.");
        assert_eq!(score.misroute, Some(Misroute::CalledWithoutNeed));
        assert!(score.grounding.passed);
        assert!(!score.passed);
    }

    #[test]
    fn malformed_and_unknown_calls_are_counted() {
        let case = case("pt-current-medication");
        let probes = [
            probe(
                0,
                "<tool_call>{\"name\": \"list_medications\", </tool_call>",
                None,
            ),
            probe(
                1,
                "<tool_call>…</tool_call>",
                Some(("list_patients", json!({}))),
            ),
            probe(
                2,
                "<tool_call>…</tool_call>",
                Some((
                    "list_medications",
                    json!({"patient_id": "synthetic-patient-560-a"}),
                )),
            ),
            probe(3, "F", None),
        ];
        let score = score_trajectory(&case, &probes, &case.reference_answer);
        assert_eq!(score.malformed_calls, 2);
        assert!(!score.passed);
        // A plain answer is not a malformed call.
        assert_eq!(
            score_trajectory(&case, &perfect_probes(&case), &case.reference_answer).malformed_calls,
            0
        );
    }

    #[test]
    fn wrong_arguments_fail_even_with_the_right_tool() {
        let case = case("pt-current-medication");
        let probes = [
            probe(
                0,
                "<tool_call>…</tool_call>",
                Some((
                    "list_medications",
                    json!({"patient_id": "synthetic-patient-560-b"}),
                )),
            ),
            probe(1, "F", None),
        ];
        let score = score_trajectory(&case, &probes, &case.reference_answer);
        assert!(score.sequence_match);
        assert_eq!((score.args_correct, score.args_total), (0, 1));
        assert!(!score.passed);
    }

    #[test]
    fn ordered_cases_reject_a_swapped_sequence_and_unordered_cases_accept_it() {
        let ordered = case("pt-medication-then-reference");
        assert!(ordered.ordered);
        let mut swapped = perfect_probes(&ordered);
        swapped.swap(0, 1);
        assert!(!score_trajectory(&ordered, &swapped, &ordered.reference_answer).sequence_match);

        let unordered = case("pt-diagnoses-and-plans");
        assert!(!unordered.ordered);
        let mut swapped = perfect_probes(&unordered);
        swapped.swap(0, 1);
        let score = score_trajectory(&unordered, &swapped, &unordered.reference_answer);
        assert!(score.sequence_match);
        assert!(score.passed, "{score:?}");
    }

    #[test]
    fn a_forbidden_fact_fails_grounding() {
        let case = case("pt-current-medication");
        let score = score_trajectory(
            &case,
            &perfect_probes(&case),
            "Sertralin 150 mg statt 100 mg, dazu Quetiapin 25 mg.",
        );
        assert!(!score.grounding.passed);
        assert_eq!(score.forbidden_facts, 1);
        assert!(!score.passed);
    }

    #[test]
    fn running_into_the_iteration_limit_fails() {
        let case = case("pt-current-medication");
        let call = (
            "list_medications",
            json!({"patient_id": "synthetic-patient-560-a"}),
        );
        let probes: Vec<ProbeTrace> = (0..MAX_ITERATIONS)
            .map(|index| probe(index, "<tool_call>…</tool_call>", Some(call.clone())))
            .collect();
        let score = score_trajectory(&case, &probes, &case.reference_answer);
        assert!(score.hit_iteration_limit);
        assert!(!score.passed);
    }

    #[test]
    fn argument_matching_is_a_case_insensitive_subset() {
        assert!(args_match(
            &json!({"query": "Bruno Beispiel", "extra": 1}),
            &json!({"query": "bruno"})
        ));
        assert!(args_match(
            &json!({"duration_minutes": "50"}),
            &json!({"duration_minutes": 50})
        ));
        assert!(args_match(
            &json!({"duration_minutes": 50}),
            &json!({"duration_minutes": 50})
        ));
        assert!(!args_match(
            &json!({"duration_minutes": 45}),
            &json!({"duration_minutes": 50})
        ));
        assert!(!args_match(&json!({}), &json!({"patient_id": "p"})));
        assert!(args_match(&json!({"anything": true}), &json!({})));
    }

    #[test]
    fn scripted_tools_return_canned_results_by_name_and_arguments() {
        let case = case("gl-compare-two-medications");
        let mut tools = ScriptedTools::new(&case.expected_calls);
        let call = |name: &str, args: Value| ToolCallRequest {
            name: name.into(),
            args,
        };
        let escitalopram = tools.execute(&call(
            "lookup_medication_reference",
            json!({"query": "Escitalopram"}),
        ));
        assert!(escitalopram.contains("synthetic-ref-escitalopram"));
        // Unmatched arguments fall back to the next unused result of the tool.
        let other = tools.execute(&call("lookup_medication_reference", json!({"query": "X"})));
        assert!(other.contains("synthetic-ref-sertralin"));
        // A repeated call gets the last scripted result of that tool again.
        let again = tools.execute(&call("lookup_medication_reference", json!({"query": "X"})));
        assert!(again.contains("synthetic-ref-escitalopram"));
        let unscripted = tools.execute(&call("list_patients", json!({})));
        assert!(serde_json::from_str::<Value>(&unscripted).unwrap()["error"].is_string());
    }

    #[test]
    fn family_summary_reports_rates_per_family() {
        let tool_case = case("pt-current-medication");
        let no_tool_case = case("gl-icd-code-meaning");
        let good_tool = score_trajectory(
            &tool_case,
            &perfect_probes(&tool_case),
            &tool_case.reference_answer,
        );
        let missed_tool = score_trajectory(&tool_case, &[probe(0, "S", None)], "Keine Angabe.");
        let needless_call = score_trajectory(
            &no_tool_case,
            &[
                probe(
                    0,
                    "<tool_call>…</tool_call>",
                    Some(("search", json!({"query": "F43.1"}))),
                ),
                probe(1, "<tool_call>{bad</tool_call>", None),
                probe(2, "P", None),
            ],
            "Posttraumatische Belastungsstörung.",
        );
        let mut runs = vec![
            run_for(&tool_case, good_tool, "Qwen/Native"),
            run_for(&tool_case, missed_tool, "Qwen/Native"),
            run_for(&no_tool_case, needless_call, "Gemma4/Gemma4"),
        ];
        let mut errored = run_for(
            &tool_case,
            score_trajectory(&tool_case, &[], ""),
            "Qwen/Native",
        );
        errored.score = None;
        errored.error = Some("load failed".into());
        runs.push(errored);

        let summaries = summarize_by_family(&runs);
        let qwen = &summaries["Qwen/Native"];
        assert_eq!((qwen.runs, qwen.runs_passed, qwen.errors), (3, 1, 1));
        assert_eq!(qwen.answered_without_tool, 1);
        assert_eq!(qwen.misroute_rate, Some(0.5));
        assert_eq!(qwen.args_accuracy, Some(0.5));
        assert_eq!(qwen.mean_iterations, Some(1.5));
        assert_eq!(qwen.malformed_call_rate, Some(0.0));

        let gemma = &summaries["Gemma4/Gemma4"];
        assert_eq!(gemma.called_without_need, 1);
        assert_eq!(gemma.misroute_rate, Some(1.0));
        assert_eq!(gemma.malformed_call_rate, Some(1.0 / 3.0));
        assert_eq!(gemma.grounding_rate, Some(1.0));
        assert_eq!(gemma.args_accuracy, None);

        let table = render_family_table(&summaries);
        assert!(table.contains("| Qwen/Native | 3 | 33% |"), "{table}");
        assert!(table.contains("| Gemma4/Gemma4 | 1 |"), "{table}");
    }

    #[test]
    fn run_options_require_model_and_output() {
        let args = |raw: &[&str]| raw.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(parse_run_options(&args(&["--model", "/m.gguf"])).is_err());
        let options = parse_run_options(&args(&[
            "--model",
            "/m.gguf",
            "--output",
            "/tmp/r.json",
            "--cases",
            "a, b",
            "--skip-model-hash",
        ]))
        .unwrap();
        assert_eq!(options.repetitions, 1);
        assert_eq!(options.profile, "governed");
        assert!(!options.hash_model);
        assert_eq!(options.case_ids.unwrap().len(), 2);
        assert!(parse_run_options(&args(&[
            "--model",
            "m",
            "--output",
            "o",
            "--repetitions",
            "0"
        ]))
        .is_err());
    }
}
