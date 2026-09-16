//! Reproducible local-inference benchmark contract and hardware runner.
//!
//! The pure validation and scoring layer is compiled in tests. The hardware
//! runner is also exposed through the opt-in `benchmark-harness` feature and
//! is orchestrated by `scripts/benchmark-local-inference.py`. Each invocation
//! measures one context profile in a fresh process because llama.cpp backend
//! initialization and process memory high-water marks are process scoped.

use super::context_cache::InferenceSession;
use super::engine::{AgentMessage, GenerationStats, LlmEngine};
use super::inference::{InferenceDiagnostics, InferenceProfile, KvCacheQuantization};
use super::memory_governor::{MemoryGovernor, MemoryGovernorDiagnostics};
use ring::digest::{Context as DigestContext, SHA256};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CStr;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const MANIFEST_JSON: &str = include_str!("../../../../benchmarks/local-inference/manifest.json");
const CLINICAL_CASES_JSON: &str =
    include_str!("../../../../benchmarks/local-inference/clinical-cases.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkManifest {
    pub schema_version: u32,
    pub suite_id: String,
    pub data_classification: String,
    pub random_seed: u64,
    pub temperature: f32,
    pub repetitions: usize,
    pub sample_interval_ms: u64,
    pub steady_state_delay_ms: u64,
    pub long_prompt_fill_ratio: f64,
    pub scenarios: Vec<String>,
    pub required_categories: Vec<String>,
    pub context_profiles: Vec<ContextProfile>,
    pub baseline: Baseline,
    pub regression_thresholds: RegressionThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextProfile {
    pub label: String,
    pub context_tokens: usize,
    pub kv_cache: String,
    pub n_batch: u32,
    pub n_ubatch: u32,
    pub completion_headroom: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub model: BaselineModel,
    pub configuration: BaselineConfiguration,
    pub llama_cpp: LlamaCppBuild,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineModel {
    pub name: String,
    pub filename: String,
    pub artifact_sha256: String,
    pub artifact_size_bytes: u64,
    pub quantization: String,
    pub native_context_tokens: usize,
    pub license: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineConfiguration {
    pub profile: String,
    pub effective_profile_on_16gib: String,
    pub context_tokens: usize,
    pub kv_cache_k: String,
    pub kv_cache_v: String,
    pub n_batch: u32,
    pub n_ubatch: u32,
    pub completion_headroom: usize,
    pub flash_attention: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaCppBuild {
    pub rust_crate: String,
    pub rust_crate_version: String,
    pub sys_crate: String,
    pub sys_crate_version: String,
    pub sys_crate_checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionThresholds {
    pub max_quality_score_drop: f64,
    pub max_ttft_increase_ratio: f64,
    pub max_prefill_increase_ratio: f64,
    pub max_total_latency_increase_ratio: f64,
    pub max_decode_throughput_decrease_ratio: f64,
    pub max_peak_rss_increase_bytes: u64,
    pub max_swap_growth_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClinicalSuite {
    schema_version: u32,
    data_classification: String,
    cases: Vec<ClinicalCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalCase {
    pub id: String,
    pub scenario: String,
    pub categories: Vec<String>,
    pub ci: bool,
    pub system_prompt: String,
    pub context: String,
    #[serde(default)]
    pub setup_prompt: Option<String>,
    pub question: String,
    pub max_tokens: usize,
    pub pad_to_context: bool,
    pub expected: ExpectedAnswer,
    pub ci_reference_answer: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExpectedAnswer {
    #[serde(default)]
    pub contains_all: Vec<String>,
    #[serde(default)]
    pub contains_any: Vec<Vec<String>>,
    #[serde(default)]
    pub excludes: Vec<String>,
    #[serde(default)]
    pub exact: Option<String>,
    #[serde(default)]
    pub tool_call: Option<ToolExpectation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExpectation {
    pub name: String,
    pub args: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub check: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClinicalScore {
    pub passed: bool,
    pub checks_passed: usize,
    pub checks_total: usize,
    pub checks: Vec<CheckResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSummary {
    pub suite_id: String,
    pub cases: usize,
    pub ci_cases: usize,
    pub scenarios: usize,
    pub categories: usize,
    pub context_profiles: usize,
}

pub fn manifest() -> Result<BenchmarkManifest, String> {
    serde_json::from_str(MANIFEST_JSON)
        .map_err(|error| format!("parse benchmark manifest: {error}"))
}

fn clinical_suite() -> Result<ClinicalSuite, String> {
    serde_json::from_str(CLINICAL_CASES_JSON)
        .map_err(|error| format!("parse clinical benchmark cases: {error}"))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_kv_cache(value: &str) -> Result<KvCacheQuantization, String> {
    match value.to_ascii_uppercase().as_str() {
        "F16" => Ok(KvCacheQuantization::F16),
        "Q8" | "Q8_0" => Ok(KvCacheQuantization::Q8),
        "Q4" | "Q4_0" => Ok(KvCacheQuantization::Q4),
        other => Err(format!("unsupported KV-cache type '{other}'")),
    }
}

/// Validate the versioned manifest and the complete synthetic clinical suite.
/// This is the deterministic CI slice: it needs no model and catches fixture,
/// scoring, baseline and context-matrix drift.
pub fn validate_embedded_suite() -> Result<ValidationSummary, String> {
    let manifest = manifest()?;
    let suite = clinical_suite()?;

    if manifest.schema_version != 1 || suite.schema_version != 1 {
        return Err("only benchmark schema version 1 is supported".to_string());
    }
    if manifest.data_classification != "synthetic_deidentified"
        || suite.data_classification != "synthetic_deidentified"
    {
        return Err("benchmark inputs must be classified synthetic_deidentified".to_string());
    }
    if !(0.5..=0.9).contains(&manifest.long_prompt_fill_ratio) {
        return Err("long_prompt_fill_ratio must stay between 0.5 and 0.9".to_string());
    }
    if manifest.random_seed != 0 || manifest.temperature != 0.0 {
        return Err(
            "deterministic benchmark sampling must remain at seed 0 and temperature 0".into(),
        );
    }
    if manifest.repetitions < 2 {
        return Err("at least two repetitions are required for cold/warm load comparison".into());
    }
    if manifest.sample_interval_ms == 0 || manifest.sample_interval_ms > 1_000 {
        return Err("memory sample interval must be between 1 and 1000 ms".into());
    }
    if manifest.steady_state_delay_ms < manifest.sample_interval_ms
        || manifest.steady_state_delay_ms > 10_000
    {
        return Err(
            "steady-state delay must span at least one sample and at most 10 seconds".into(),
        );
    }

    let expected_contexts = [2_048, 8_192, 16_384, 32_768, 65_536, 131_072];
    let actual_contexts: Vec<usize> = manifest
        .context_profiles
        .iter()
        .map(|profile| profile.context_tokens)
        .collect();
    if actual_contexts != expected_contexts {
        return Err(format!(
            "context matrix must be exactly 2K/8K/16K/32K/64K/128K, got {actual_contexts:?}"
        ));
    }
    let mut profile_labels = BTreeSet::new();
    for profile in &manifest.context_profiles {
        if !profile_labels.insert(profile.label.as_str()) {
            return Err(format!(
                "duplicate context profile label '{}'",
                profile.label
            ));
        }
        let kv = parse_kv_cache(&profile.kv_cache)?;
        InferenceProfile::for_benchmark(
            profile.context_tokens,
            kv,
            profile.n_batch,
            profile.n_ubatch,
            profile.completion_headroom,
        )
        .map_err(|error| format!("invalid context profile '{}': {error}", profile.label))?;
    }

    let expected_scenarios: BTreeSet<&str> = [
        "cold_prompt",
        "shared_prefix",
        "continued_session",
        "agent_tool_call",
    ]
    .into_iter()
    .collect();
    let declared_scenarios: BTreeSet<&str> =
        manifest.scenarios.iter().map(String::as_str).collect();
    if declared_scenarios != expected_scenarios {
        return Err(format!(
            "scenario matrix is incomplete: {declared_scenarios:?}"
        ));
    }

    if !is_sha256(&manifest.baseline.model.artifact_sha256)
        || !is_sha256(&manifest.baseline.llama_cpp.sys_crate_checksum)
    {
        return Err("baseline artifact and llama.cpp checksums must be SHA-256 values".into());
    }
    if manifest.baseline.model.filename != "Qwen3-8B-Q4_K_M.gguf"
        || manifest.baseline.model.quantization != "Q4_K_M"
        || manifest.baseline.configuration.profile != "governed"
        || manifest.baseline.configuration.effective_profile_on_16gib != "governed-f16"
        || manifest.baseline.configuration.context_tokens != 16_384
        || manifest.baseline.configuration.kv_cache_k != "F16"
        || manifest.baseline.configuration.kv_cache_v != "F16"
        || manifest.baseline.configuration.n_batch != 2_048
        || manifest.baseline.configuration.n_ubatch != 512
    {
        return Err("the captured Qwen3-8B baseline no longer matches the shipped profile".into());
    }

    let mut ids = BTreeSet::new();
    let mut covered_scenarios = BTreeSet::new();
    let mut covered_categories = BTreeSet::new();
    let mut ci_cases = 0usize;
    let mut padded_cases = 0usize;
    for case in &suite.cases {
        if !ids.insert(case.id.as_str()) {
            return Err(format!("duplicate clinical case id '{}'", case.id));
        }
        if case.id.trim().is_empty()
            || case.system_prompt.trim().is_empty()
            || case.context.trim().is_empty()
            || case.question.trim().is_empty()
            || case.max_tokens == 0
        {
            return Err(format!(
                "clinical case '{}' has an empty required field",
                case.id
            ));
        }
        if !case.context.to_ascii_lowercase().contains("synthet") {
            return Err(format!(
                "clinical case '{}' must identify its source text as synthetic",
                case.id
            ));
        }
        if !declared_scenarios.contains(case.scenario.as_str()) {
            return Err(format!(
                "clinical case '{}' uses undeclared scenario '{}'",
                case.id, case.scenario
            ));
        }
        if case.categories.is_empty() {
            return Err(format!("clinical case '{}' has no category", case.id));
        }
        if case.expected.contains_all.is_empty()
            && case.expected.contains_any.is_empty()
            && case.expected.excludes.is_empty()
            && case.expected.exact.is_none()
            && case.expected.tool_call.is_none()
        {
            return Err(format!(
                "clinical case '{}' has no deterministic checks",
                case.id
            ));
        }
        if case.scenario == "agent_tool_call" && case.expected.tool_call.is_none() {
            return Err(format!(
                "agent tool case '{}' must declare the exact tool call",
                case.id
            ));
        }
        if case.pad_to_context {
            padded_cases += 1;
        }
        if case.ci {
            ci_cases += 1;
            let score = score_case(case, &case.ci_reference_answer);
            if !score.passed {
                return Err(format!(
                    "CI reference answer for '{}' does not satisfy its checks: {:?}",
                    case.id, score.checks
                ));
            }
        }
        covered_scenarios.insert(case.scenario.as_str());
        covered_categories.extend(case.categories.iter().map(String::as_str));
    }
    if covered_scenarios != expected_scenarios {
        return Err(format!(
            "clinical cases do not cover every scenario: {covered_scenarios:?}"
        ));
    }
    let required_categories: BTreeSet<&str> = manifest
        .required_categories
        .iter()
        .map(String::as_str)
        .collect();
    let missing: Vec<&str> = required_categories
        .difference(&covered_categories)
        .copied()
        .collect();
    if !missing.is_empty() {
        return Err(format!("clinical suite is missing categories: {missing:?}"));
    }
    if ci_cases == 0 || ci_cases == suite.cases.len() {
        return Err("CI subset must be non-empty and smaller than the full suite".into());
    }
    if padded_cases == 0 {
        return Err("at least one case must exercise middle-of-context padding".into());
    }

    Ok(ValidationSummary {
        suite_id: manifest.suite_id,
        cases: suite.cases.len(),
        ci_cases,
        scenarios: covered_scenarios.len(),
        categories: covered_categories.len(),
        context_profiles: manifest.context_profiles.len(),
    })
}

fn folded(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn tool_call_from_answer(answer: &str) -> Result<ToolExpectation, String> {
    let trimmed = answer.trim();
    if !trimmed.starts_with("<tool_call>") || !trimmed.ends_with("</tool_call>") {
        return Err("tool response must contain only one <tool_call> block".to_string());
    }
    let json = &trimmed["<tool_call>".len()..trimmed.len() - "</tool_call>".len()];
    serde_json::from_str(json.trim()).map_err(|error| format!("invalid tool-call JSON: {error}"))
}

fn json_contains(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected.iter().all(|(key, value)| {
            actual
                .get(key)
                .is_some_and(|found| json_contains(found, value))
        }),
        (Value::Array(actual), Value::Array(expected)) => expected
            .iter()
            .all(|value| actual.iter().any(|found| json_contains(found, value))),
        _ => actual == expected,
    }
}

/// Score a model answer without fuzzy or model-graded judgments. Exact
/// clinical tokens, exclusions and tool arguments remain individually visible
/// so an aggregate score cannot hide a harmful category regression.
pub fn score_case(case: &ClinicalCase, answer: &str) -> ClinicalScore {
    let normalized = folded(answer);
    let mut checks = Vec::new();
    for expected in &case.expected.contains_all {
        let passed = normalized.contains(&folded(expected));
        checks.push(CheckResult {
            check: format!("contains:{expected}"),
            passed,
            detail: if passed {
                "required text present".to_string()
            } else {
                format!("missing required text '{expected}'")
            },
        });
    }
    for alternatives in &case.expected.contains_any {
        let passed = alternatives
            .iter()
            .any(|expected| normalized.contains(&folded(expected)));
        checks.push(CheckResult {
            check: format!("contains_any:{}", alternatives.join("|")),
            passed,
            detail: if passed {
                "one required alternative present".to_string()
            } else {
                format!("none of the alternatives were present: {alternatives:?}")
            },
        });
    }
    for excluded in &case.expected.excludes {
        let passed = !normalized.contains(&folded(excluded));
        checks.push(CheckResult {
            check: format!("excludes:{excluded}"),
            passed,
            detail: if passed {
                "forbidden text absent".to_string()
            } else {
                format!("forbidden text '{excluded}' was present")
            },
        });
    }
    if let Some(exact) = &case.expected.exact {
        let passed = normalized == folded(exact);
        checks.push(CheckResult {
            check: "exact".to_string(),
            passed,
            detail: if passed {
                "answer matched exactly".to_string()
            } else {
                "answer did not match the exact deterministic target".to_string()
            },
        });
    }
    if let Some(expected) = &case.expected.tool_call {
        match tool_call_from_answer(answer) {
            Ok(actual) => {
                let name_passed = actual.name == expected.name;
                checks.push(CheckResult {
                    check: "tool_call.name".to_string(),
                    passed: name_passed,
                    detail: if name_passed {
                        "tool name matched".to_string()
                    } else {
                        format!(
                            "expected tool '{}', received '{}'",
                            expected.name, actual.name
                        )
                    },
                });
                let args_passed = json_contains(&actual.args, &expected.args);
                checks.push(CheckResult {
                    check: "tool_call.args".to_string(),
                    passed: args_passed,
                    detail: if args_passed {
                        "required tool arguments matched".to_string()
                    } else {
                        format!(
                            "required arguments {} were not contained in {}",
                            expected.args, actual.args
                        )
                    },
                });
            }
            Err(error) => checks.push(CheckResult {
                check: "tool_call.json".to_string(),
                passed: false,
                detail: error,
            }),
        }
    }
    let checks_passed = checks.iter().filter(|check| check.passed).count();
    ClinicalScore {
        passed: !checks.is_empty() && checks_passed == checks.len(),
        checks_passed,
        checks_total: checks.len(),
        checks,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemorySnapshot {
    /// Current RSS of this benchmark process, which owns the full RamDoc Rust
    /// runtime and LLM engine. This is intentionally not a model-file size.
    pub process_rss_bytes: Option<u64>,
    /// System-wide wired memory provides pressure context for unified memory.
    pub system_wired_bytes: Option<u64>,
    /// System-wide compressor occupancy provides pressure context on macOS.
    pub system_compressed_bytes: Option<u64>,
    pub system_swap_used_bytes: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemorySummary {
    pub samples: usize,
    pub baseline: MemorySnapshot,
    pub peak: MemorySnapshot,
    pub steady: MemorySnapshot,
    pub swap_delta_bytes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TimedMemorySnapshot {
    elapsed_ms: f64,
    memory: MemorySnapshot,
}

struct MemorySampler {
    started: Instant,
    stop: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<TimedMemorySnapshot>>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl MemorySampler {
    fn start(interval_ms: u64) -> Self {
        let started = Instant::now();
        let stop = Arc::new(AtomicBool::new(false));
        let samples = Arc::new(Mutex::new(Vec::new()));
        push_memory_sample(&samples, started);
        let thread_stop = Arc::clone(&stop);
        let thread_samples = Arc::clone(&samples);
        let handle = thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(interval_ms));
                push_memory_sample(&thread_samples, started);
            }
        });
        Self {
            started,
            stop,
            samples,
            thread: Some(handle),
        }
    }

    fn capture_now(&self) {
        push_memory_sample(&self.samples, self.started);
    }

    fn mark(&self) -> usize {
        self.samples.lock().map(|items| items.len()).unwrap_or(0)
    }

    fn summary_since(&self, mark: usize) -> MemorySummary {
        self.samples
            .lock()
            .ok()
            .map(|items| summarize_memory(&items[mark.min(items.len())..]))
            .unwrap_or_default()
    }

    fn finish(mut self) -> MemorySummary {
        self.capture_now();
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
        self.capture_now();
        self.samples
            .lock()
            .ok()
            .map(|items| summarize_memory(items.as_slice()))
            .unwrap_or_default()
    }
}

fn push_memory_sample(samples: &Mutex<Vec<TimedMemorySnapshot>>, started: Instant) {
    if let Ok(mut items) = samples.lock() {
        items.push(TimedMemorySnapshot {
            elapsed_ms: started.elapsed().as_secs_f64() * 1_000.0,
            memory: capture_memory(),
        });
    }
}

fn maximum(values: impl Iterator<Item = Option<u64>>) -> Option<u64> {
    values.flatten().max()
}

fn signed_delta(after: u64, before: u64) -> i64 {
    let difference = after as i128 - before as i128;
    difference.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

fn summarize_memory(samples: &[TimedMemorySnapshot]) -> MemorySummary {
    let Some(first) = samples.first() else {
        return MemorySummary::default();
    };
    let last = samples.last().unwrap_or(first);
    MemorySummary {
        samples: samples.len(),
        baseline: first.memory.clone(),
        peak: MemorySnapshot {
            process_rss_bytes: maximum(samples.iter().map(|item| item.memory.process_rss_bytes)),
            system_wired_bytes: maximum(samples.iter().map(|item| item.memory.system_wired_bytes)),
            system_compressed_bytes: maximum(
                samples
                    .iter()
                    .map(|item| item.memory.system_compressed_bytes),
            ),
            system_swap_used_bytes: maximum(
                samples
                    .iter()
                    .map(|item| item.memory.system_swap_used_bytes),
            ),
        },
        steady: last.memory.clone(),
        swap_delta_bytes: first
            .memory
            .system_swap_used_bytes
            .zip(last.memory.system_swap_used_bytes)
            .map(|(before, after)| signed_delta(after, before)),
    }
}

#[cfg(target_os = "macos")]
#[allow(deprecated)] // libc exposes Mach host statistics behind a deprecated binding.
fn capture_memory() -> MemorySnapshot {
    let process_rss_bytes = {
        let mut info = std::mem::MaybeUninit::<libc::proc_taskinfo>::zeroed();
        let size = std::mem::size_of::<libc::proc_taskinfo>() as i32;
        let read = unsafe {
            libc::proc_pidinfo(
                std::process::id() as i32,
                libc::PROC_PIDTASKINFO,
                0,
                info.as_mut_ptr() as *mut libc::c_void,
                size,
            )
        };
        (read == size).then(|| unsafe { info.assume_init() }.pti_resident_size)
    };

    let (system_wired_bytes, system_compressed_bytes) = {
        let mut stats = std::mem::MaybeUninit::<libc::vm_statistics64>::zeroed();
        let mut count = libc::HOST_VM_INFO64_COUNT;
        let result = unsafe {
            libc::host_statistics64(
                libc::mach_host_self(),
                libc::HOST_VM_INFO64,
                stats.as_mut_ptr() as libc::host_info64_t,
                &mut count,
            )
        };
        if result == libc::KERN_SUCCESS {
            let stats = unsafe { stats.assume_init() };
            let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
            if page_size > 0 {
                let page_size = page_size as u64;
                (
                    Some((stats.wire_count as u64).saturating_mul(page_size)),
                    Some((stats.compressor_page_count as u64).saturating_mul(page_size)),
                )
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    };

    MemorySnapshot {
        process_rss_bytes,
        system_wired_bytes,
        system_compressed_bytes,
        system_swap_used_bytes: swap_used_bytes(),
    }
}

#[cfg(target_os = "macos")]
fn swap_used_bytes() -> Option<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::xsw_usage>::zeroed();
    let mut len = std::mem::size_of::<libc::xsw_usage>();
    let name = std::ffi::CString::new("vm.swapusage").expect("static sysctl name");
    let ok = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            usage.as_mut_ptr() as *mut libc::c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    } == 0;
    ok.then(|| unsafe { usage.assume_init() }.xsu_used)
}

#[cfg(not(target_os = "macos"))]
fn capture_memory() -> MemorySnapshot {
    MemorySnapshot::default()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildMetadata {
    pub ramdoc_version: String,
    pub git_commit: Option<String>,
    pub git_dirty: Option<bool>,
    pub executable_sha256: Option<String>,
    pub cargo_lock_sha256: Option<String>,
    pub rustc: Option<String>,
    pub llama_cpp: LlamaCppBuild,
    pub llama_system_info: Option<String>,
    pub manifest_sha256: String,
    pub clinical_cases_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostMetadata {
    pub os: String,
    pub os_version: Option<String>,
    pub architecture: String,
    pub hardware_model: Option<String>,
    pub cpu: Option<String>,
    pub physical_memory_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub filename: String,
    pub artifact_sha256: String,
    pub artifact_size_bytes: u64,
    pub quantization: String,
    pub native_context_tokens: usize,
    pub matches_captured_qwen_baseline: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestedConfiguration {
    pub label: String,
    pub context_tokens: usize,
    pub kv_cache: String,
    pub n_batch: u32,
    pub n_ubatch: u32,
    pub completion_headroom: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreCount {
    pub passed: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub case_id: String,
    pub scenario: String,
    pub categories: Vec<String>,
    pub target_prompt_tokens: Option<usize>,
    pub answer: String,
    pub score: ClinicalScore,
    pub infrastructure_failures: Vec<String>,
    pub stats: Option<GenerationStats>,
    pub warmup_stats: Option<GenerationStats>,
    pub memory: MemorySummary,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    pub schema_version: u32,
    pub suite_id: String,
    pub run_id: String,
    pub started_at: String,
    pub status: String,
    pub reason: Option<String>,
    pub load_state: String,
    pub repetition: usize,
    pub quick: bool,
    pub build: BuildMetadata,
    pub host: HostMetadata,
    pub model: ModelMetadata,
    pub requested_configuration: RequestedConfiguration,
    pub effective_configuration: Option<InferenceDiagnostics>,
    pub planning: MemoryGovernorDiagnostics,
    pub load_ms: Option<f64>,
    pub memory: MemorySummary,
    pub scenarios: Vec<ScenarioResult>,
    pub category_scores: BTreeMap<String, ScoreCount>,
    pub cases_passed: usize,
    pub cases_total: usize,
}

#[derive(Debug, Clone)]
struct RunOptions {
    model_path: PathBuf,
    model_sha256: Option<String>,
    artifact_quantization: String,
    profile_label: String,
    context_tokens: usize,
    kv_cache: KvCacheQuantization,
    kv_cache_label: String,
    n_batch: u32,
    n_ubatch: u32,
    completion_headroom: usize,
    load_state: String,
    repetition: usize,
    run_id: String,
    quick: bool,
    output: PathBuf,
}

fn required_arg(args: &BTreeMap<String, String>, name: &str) -> Result<String, String> {
    args.get(name)
        .cloned()
        .ok_or_else(|| format!("missing required argument --{name}"))
}

fn numeric_arg<T: std::str::FromStr>(
    args: &BTreeMap<String, String>,
    name: &str,
) -> Result<T, String> {
    required_arg(args, name)?
        .parse()
        .map_err(|_| format!("--{name} must be a valid number"))
}

fn parse_run_options(raw: &[String]) -> Result<RunOptions, String> {
    let mut values = BTreeMap::new();
    let mut quick = false;
    let mut index = 0;
    while index < raw.len() {
        let key = raw[index]
            .strip_prefix("--")
            .ok_or_else(|| format!("unexpected argument '{}'", raw[index]))?;
        if key == "quick" {
            quick = true;
            index += 1;
            continue;
        }
        let value = raw
            .get(index + 1)
            .ok_or_else(|| format!("--{key} requires a value"))?;
        values.insert(key.to_string(), value.clone());
        index += 2;
    }

    let kv_cache_label = required_arg(&values, "kv-cache")?;
    let model_sha256 = values.get("model-sha256").cloned();
    if model_sha256.as_deref().is_some_and(|hash| !is_sha256(hash)) {
        return Err("--model-sha256 must contain 64 hexadecimal characters".into());
    }
    Ok(RunOptions {
        model_path: PathBuf::from(required_arg(&values, "model")?),
        model_sha256,
        artifact_quantization: required_arg(&values, "artifact-quantization")?,
        profile_label: required_arg(&values, "profile-label")?,
        context_tokens: numeric_arg(&values, "context")?,
        kv_cache: parse_kv_cache(&kv_cache_label)?,
        kv_cache_label,
        n_batch: numeric_arg(&values, "n-batch")?,
        n_ubatch: numeric_arg(&values, "n-ubatch")?,
        completion_headroom: numeric_arg(&values, "headroom")?,
        load_state: required_arg(&values, "load-state")?,
        repetition: numeric_arg(&values, "repetition")?,
        run_id: required_arg(&values, "run-id")?,
        quick,
        output: PathBuf::from(required_arg(&values, "output")?),
    })
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let digest = ring::digest::digest(&SHA256, bytes);
    hex::encode(digest.as_ref())
}

fn sha256_path(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|error| format!("open '{}': {error}", path.display()))?;
    let mut context = DigestContext::new(&SHA256);
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("read '{}': {error}", path.display()))?;
        if read == 0 {
            break;
        }
        context.update(&buffer[..read]);
    }
    Ok(hex::encode(context.finish().as_ref()))
}

fn command_output(command: &mut Command) -> Option<String> {
    let output = command.output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn build_metadata(manifest: &BenchmarkManifest) -> BuildMetadata {
    let root = repository_root();
    let git_commit = command_output(
        Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["rev-parse", "HEAD"]),
    );
    let git_dirty = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| !output.stdout.is_empty());
    let executable_sha256 = std::env::current_exe()
        .ok()
        .and_then(|path| sha256_path(&path).ok());
    let cargo_lock_sha256 =
        sha256_path(&Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock")).ok();
    let rustc = command_output(Command::new("rustc").arg("--version"));
    let llama_system_info = unsafe {
        let pointer = llama_cpp_sys_2::llama_print_system_info();
        (!pointer.is_null()).then(|| CStr::from_ptr(pointer).to_string_lossy().into_owned())
    };
    BuildMetadata {
        ramdoc_version: env!("CARGO_PKG_VERSION").to_string(),
        git_commit,
        git_dirty,
        executable_sha256,
        cargo_lock_sha256,
        rustc,
        llama_cpp: manifest.baseline.llama_cpp.clone(),
        llama_system_info,
        manifest_sha256: sha256_bytes(MANIFEST_JSON.as_bytes()),
        clinical_cases_sha256: sha256_bytes(CLINICAL_CASES_JSON.as_bytes()),
    }
}

#[cfg(target_os = "macos")]
fn sysctl_string(key: &str) -> Option<String> {
    let key = std::ffi::CString::new(key).ok()?;
    let mut length = 0usize;
    if unsafe {
        libc::sysctlbyname(
            key.as_ptr(),
            std::ptr::null_mut(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    } != 0
        || length == 0
    {
        return None;
    }
    let mut bytes = vec![0u8; length];
    if unsafe {
        libc::sysctlbyname(
            key.as_ptr(),
            bytes.as_mut_ptr() as *mut libc::c_void,
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    } != 0
    {
        return None;
    }
    bytes.truncate(length);
    while matches!(bytes.last(), Some(0)) {
        bytes.pop();
    }
    String::from_utf8(bytes).ok()
}

#[cfg(not(target_os = "macos"))]
fn sysctl_string(_key: &str) -> Option<String> {
    None
}

fn host_metadata() -> HostMetadata {
    HostMetadata {
        os: std::env::consts::OS.to_string(),
        os_version: command_output(Command::new("sw_vers").arg("-productVersion")),
        architecture: std::env::consts::ARCH.to_string(),
        hardware_model: sysctl_string("hw.model"),
        cpu: sysctl_string("machdep.cpu.brand_string"),
        physical_memory_bytes: LlmEngine::total_ram(),
    }
}

fn model_metadata(
    options: &RunOptions,
    manifest: &BenchmarkManifest,
    hash: String,
    size: u64,
    native_context_tokens: usize,
) -> Result<ModelMetadata, String> {
    let filename = options
        .model_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "model path has no UTF-8 filename".to_string())?
        .to_string();
    let matches_captured_qwen_baseline =
        (filename == manifest.baseline.model.filename).then(|| {
            hash.eq_ignore_ascii_case(&manifest.baseline.model.artifact_sha256)
                && size == manifest.baseline.model.artifact_size_bytes
                && options.artifact_quantization == manifest.baseline.model.quantization
        });
    if matches_captured_qwen_baseline == Some(false) {
        return Err(format!(
            "{} does not match the captured Qwen3-8B artifact hash, size and quantization",
            filename
        ));
    }
    Ok(ModelMetadata {
        filename,
        artifact_sha256: hash,
        artifact_size_bytes: size,
        quantization: options.artifact_quantization.clone(),
        native_context_tokens,
        matches_captured_qwen_baseline,
    })
}

fn requested_configuration(options: &RunOptions) -> RequestedConfiguration {
    RequestedConfiguration {
        label: options.profile_label.clone(),
        context_tokens: options.context_tokens,
        kv_cache: options.kv_cache_label.clone(),
        n_batch: options.n_batch,
        n_ubatch: options.n_ubatch,
        completion_headroom: options.completion_headroom,
    }
}

fn empty_process_result(
    options: &RunOptions,
    manifest: &BenchmarkManifest,
    build: BuildMetadata,
    host: HostMetadata,
    model: ModelMetadata,
    planning: MemoryGovernorDiagnostics,
) -> ProcessResult {
    ProcessResult {
        schema_version: 1,
        suite_id: manifest.suite_id.clone(),
        run_id: options.run_id.clone(),
        started_at: chrono::Utc::now().to_rfc3339(),
        status: "pending".to_string(),
        reason: None,
        load_state: options.load_state.clone(),
        repetition: options.repetition,
        quick: options.quick,
        build,
        host,
        model,
        requested_configuration: requested_configuration(options),
        effective_configuration: None,
        planning,
        load_ms: None,
        memory: summarize_memory(&[TimedMemorySnapshot {
            elapsed_ms: 0.0,
            memory: capture_memory(),
        }]),
        scenarios: Vec::new(),
        category_scores: BTreeMap::new(),
        cases_passed: 0,
        cases_total: 0,
    }
}

fn make_user_prompt(context: &str, question: &str) -> String {
    format!("SYNTHETISCHE AKTE:\n{context}\n\nAUFGABE:\n{question}")
}

fn padded_context(
    engine: &LlmEngine,
    case: &ClinicalCase,
    context_tokens: usize,
    completion_headroom: usize,
    fill_ratio: f64,
) -> (String, usize) {
    let target =
        ((context_tokens.saturating_sub(completion_headroom)) as f64 * fill_ratio).floor() as usize;
    let filler = "Synthetischer Routineeintrag ohne entscheidende klinische Angabe. Das Befinden blieb unverändert und das Gespräch wurde fortgesetzt.\n";
    let base_user = make_user_prompt(&case.context, &case.question);
    let base_formatted = engine
        .format_chat_history(
            &case.system_prompt,
            &[AgentMessage {
                role: "user".into(),
                content: base_user,
            }],
        )
        .unwrap_or_else(|_| case.context.clone());
    let base_tokens = engine.count_tokens(&base_formatted);
    let filler_tokens = engine.count_tokens(filler).max(1);
    let mut repetitions = target.saturating_sub(base_tokens) / filler_tokens;
    loop {
        let before = filler.repeat(repetitions / 2);
        let after = filler.repeat(repetitions - repetitions / 2);
        let candidate = format!("{before}{}{after}", case.context);
        let formatted = engine
            .format_chat_history(
                &case.system_prompt,
                &[AgentMessage {
                    role: "user".into(),
                    content: make_user_prompt(&candidate, &case.question),
                }],
            )
            .unwrap_or_else(|_| candidate.clone());
        if engine.count_tokens(&formatted) <= target || repetitions == 0 {
            return (candidate, target);
        }
        repetitions = repetitions.saturating_sub((repetitions / 20).max(1));
    }
}

fn generate_session_turn(
    engine: &LlmEngine,
    session: &InferenceSession,
    system_prompt: &str,
    messages: &[AgentMessage],
    max_tokens: usize,
    temperature: f32,
) -> Result<(String, GenerationStats), String> {
    let prompt = engine
        .format_chat_history(system_prompt, messages)
        .map_err(|error| error.to_string())?;
    let mut answer = String::new();
    engine
        .generate_streaming_session(
            session,
            system_prompt,
            &prompt,
            max_tokens,
            temperature,
            |piece| {
                answer.push_str(piece);
                true
            },
        )
        .map_err(|error| error.to_string())?;
    let stats = engine
        .last_generation_stats()
        .ok_or_else(|| "generation completed without telemetry".to_string())?;
    Ok((answer, stats))
}

fn execute_case(
    engine: &LlmEngine,
    sampler: &MemorySampler,
    case: &ClinicalCase,
    options: &RunOptions,
    manifest: &BenchmarkManifest,
) -> ScenarioResult {
    sampler.capture_now();
    let mark = sampler.mark().saturating_sub(1);
    let (context, target_prompt_tokens) = if case.pad_to_context {
        let (context, target) = padded_context(
            engine,
            case,
            options.context_tokens,
            options.completion_headroom,
            manifest.long_prompt_fill_ratio,
        );
        (context, Some(target))
    } else {
        (case.context.clone(), None)
    };

    let mut warmup_stats = None;
    let generated = if case.scenario == "cold_prompt" {
        let prompt = make_user_prompt(&context, &case.question);
        engine
            .generate(
                &case.system_prompt,
                &prompt,
                case.max_tokens,
                manifest.temperature,
            )
            .map_err(|error| error.to_string())
            .and_then(|answer| {
                engine
                    .last_generation_stats()
                    .map(|stats| (answer, stats))
                    .ok_or_else(|| "generation completed without telemetry".to_string())
            })
    } else {
        let setup = case.setup_prompt.as_deref().unwrap_or(
            "Bestätige die synthetische Test-Patienten-ID kurz, ohne ein Tool aufzurufen.",
        );
        let setup_user = make_user_prompt(&context, setup);
        let session = InferenceSession::agent(
            format!("benchmark-{}-{}", options.run_id, case.id),
            Some("synthetic-patient-398".to_string()),
            Some("fixture-v1".to_string()),
        );
        let setup_messages = vec![AgentMessage {
            role: "user".into(),
            content: setup_user.clone(),
        }];
        match generate_session_turn(
            engine,
            &session,
            &case.system_prompt,
            &setup_messages,
            case.max_tokens,
            manifest.temperature,
        ) {
            Ok((setup_answer, setup_telemetry)) => {
                warmup_stats = Some(setup_telemetry);
                let messages = vec![
                    AgentMessage {
                        role: "user".into(),
                        content: setup_user,
                    },
                    AgentMessage {
                        role: "assistant".into(),
                        content: setup_answer,
                    },
                    AgentMessage {
                        role: "user".into(),
                        content: case.question.clone(),
                    },
                ];
                generate_session_turn(
                    engine,
                    &session,
                    &case.system_prompt,
                    &messages,
                    case.max_tokens,
                    manifest.temperature,
                )
            }
            Err(error) => Err(format!("warmup turn failed: {error}")),
        }
    };

    sampler.capture_now();
    let memory = sampler.summary_since(mark);
    match generated {
        Ok((answer, stats)) => {
            let mut score = score_case(case, answer.trim());
            let mut infrastructure_failures = Vec::new();
            if case.scenario != "cold_prompt"
                && (!stats.cache_hit || stats.reused_prompt_tokens == 0)
            {
                let detail = "warm scenario did not reuse any prompt tokens".to_string();
                infrastructure_failures.push(detail.clone());
                score.checks.push(CheckResult {
                    check: "context_cache_reuse".to_string(),
                    passed: false,
                    detail,
                });
                score.checks_total += 1;
                score.passed = false;
            }
            ScenarioResult {
                case_id: case.id.clone(),
                scenario: case.scenario.clone(),
                categories: case.categories.clone(),
                target_prompt_tokens,
                answer: answer.trim().to_string(),
                score,
                infrastructure_failures,
                stats: Some(stats),
                warmup_stats,
                memory,
                error: None,
            }
        }
        Err(error) => ScenarioResult {
            case_id: case.id.clone(),
            scenario: case.scenario.clone(),
            categories: case.categories.clone(),
            target_prompt_tokens,
            answer: String::new(),
            score: ClinicalScore {
                passed: false,
                checks_passed: 0,
                checks_total: 1,
                checks: vec![CheckResult {
                    check: "generation".to_string(),
                    passed: false,
                    detail: error.clone(),
                }],
            },
            infrastructure_failures: vec![error.clone()],
            stats: None,
            warmup_stats,
            memory,
            error: Some(error),
        },
    }
}

fn aggregate_scores(result: &mut ProcessResult) {
    result.cases_total = result.scenarios.len();
    result.cases_passed = result
        .scenarios
        .iter()
        .filter(|scenario| scenario.score.passed)
        .count();
    for scenario in &result.scenarios {
        for category in &scenario.categories {
            let count = result.category_scores.entry(category.clone()).or_default();
            count.total += 1;
            if scenario.score.passed {
                count.passed += 1;
            }
        }
    }
}

fn run_hardware(options: &RunOptions) -> Result<ProcessResult, String> {
    if !cfg!(target_os = "macos") {
        return Err("full local-inference benchmarks require macOS".to_string());
    }
    validate_embedded_suite()?;
    let manifest = manifest()?;
    let suite = clinical_suite()?;
    let profile = InferenceProfile::for_benchmark(
        options.context_tokens,
        options.kv_cache,
        options.n_batch,
        options.n_ubatch,
        options.completion_headroom,
    )
    .map_err(|error| format!("invalid benchmark profile: {error}"))?;
    let size = std::fs::metadata(&options.model_path)
        .map_err(|error| format!("inspect '{}': {error}", options.model_path.display()))?
        .len();
    let hash = match &options.model_sha256 {
        Some(hash) => hash.to_ascii_lowercase(),
        None => sha256_path(&options.model_path)?,
    };
    let governor = MemoryGovernor::inspect(&options.model_path, LlmEngine::total_ram())
        .map_err(|error| error.to_string())?;
    let (planned_profile, planning) = governor.plan(Some(&profile));
    let model = model_metadata(
        options,
        &manifest,
        hash.clone(),
        size,
        planning.architecture.native_context,
    )?;
    let mut result = empty_process_result(
        options,
        &manifest,
        build_metadata(&manifest),
        host_metadata(),
        model,
        planning.clone(),
    );

    if options.context_tokens > planning.architecture.native_context {
        result.status = "skipped".to_string();
        result.reason = Some(format!(
            "requested {} tokens, but model native context is {}",
            options.context_tokens, planning.architecture.native_context
        ));
        return Ok(result);
    }
    if planned_profile.n_ctx != options.context_tokens {
        result.status = "skipped".to_string();
        result.reason = Some(format!(
            "requested context resolved to {} tokens",
            planned_profile.n_ctx
        ));
        return Ok(result);
    }
    if !planning.safe {
        result.status = "skipped".to_string();
        result.reason = Some(format!(
            "memory governor refused profile: {}",
            planning.reason
        ));
        return Ok(result);
    }

    let sampler = MemorySampler::start(manifest.sample_interval_ms);
    let load_started = Instant::now();
    let model_name = result.model.filename.clone();
    let engine = match LlmEngine::load_benchmark_profile(
        options.model_path.clone(),
        model_name,
        profile,
        Some(hash),
    ) {
        Ok(engine) => engine,
        Err(error) => {
            result.status = "failed".to_string();
            result.reason = Some(format!("model load failed: {error}"));
            result.load_ms = Some(load_started.elapsed().as_secs_f64() * 1_000.0);
            result.memory = sampler.finish();
            return Ok(result);
        }
    };
    result.load_ms = Some(load_started.elapsed().as_secs_f64() * 1_000.0);
    result.effective_configuration = engine.status().inference_config;

    for case in suite.cases.iter().filter(|case| !options.quick || case.ci) {
        result
            .scenarios
            .push(execute_case(&engine, &sampler, case, options, &manifest));
    }
    thread::sleep(Duration::from_millis(manifest.steady_state_delay_ms));
    result.memory = sampler.finish();
    // GenerationStats uses the process-lifetime RSS high-water mark. Merge it
    // with the high-frequency sampler so a short allocation spike between two
    // samples cannot be lost; steady RSS still comes from the sampler.
    if let Some(high_water) = result
        .scenarios
        .iter()
        .filter_map(|scenario| scenario.stats.as_ref().map(|stats| stats.peak_rss_bytes))
        .max()
    {
        result.memory.peak.process_rss_bytes = Some(
            result
                .memory
                .peak
                .process_rss_bytes
                .map_or(high_water, |sampled| sampled.max(high_water)),
        );
    }
    aggregate_scores(&mut result);
    result.status = if result.cases_passed == result.cases_total {
        "completed".to_string()
    } else {
        "quality_failed".to_string()
    };
    Ok(result)
}

fn write_process_result(path: &Path, result: &ProcessResult) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create '{}': {error}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(result)
        .map_err(|error| format!("serialize process result: {error}"))?;
    std::fs::write(path, format!("{json}\n"))
        .map_err(|error| format!("write '{}': {error}", path.display()))
}

pub fn cli_main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        return Err("usage: local-inference-benchmark <validate|run> [options]".into());
    };
    match command {
        "validate" => {
            let summary = validate_embedded_suite()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&summary)
                    .map_err(|error| format!("serialize validation summary: {error}"))?
            );
            Ok(())
        }
        "run" => {
            let options = parse_run_options(&args[1..])?;
            let result = run_hardware(&options)?;
            write_process_result(&options.output, &result)?;
            println!("wrote {} ({})", options.output.display(), result.status);
            Ok(())
        }
        other => Err(format!(
            "unknown command '{other}'; expected validate or run"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_manifest_and_clinical_suite_are_complete() {
        let summary = validate_embedded_suite().unwrap();
        assert_eq!(summary.context_profiles, 6);
        assert_eq!(summary.scenarios, 4);
        assert!(summary.categories >= 11);
        assert!(summary.ci_cases < summary.cases);
    }

    #[test]
    fn captured_baseline_matches_the_download_whitelist_and_cargo_lock() {
        let manifest = manifest().unwrap();
        let model = super::super::download::find_model(&manifest.baseline.model.filename)
            .expect("captured baseline remains approved");
        assert_eq!(model.pinned_sha256, manifest.baseline.model.artifact_sha256);
        assert_eq!(
            model.size_bytes,
            manifest.baseline.model.artifact_size_bytes
        );
        assert_eq!(
            model.context_window_tokens as usize,
            manifest.baseline.model.native_context_tokens
        );

        let lock = include_str!("../../Cargo.lock");
        assert!(lock.contains(&format!(
            "name = \"{}\"\nversion = \"{}\"",
            manifest.baseline.llama_cpp.rust_crate, manifest.baseline.llama_cpp.rust_crate_version
        )));
        assert!(lock.contains(&format!(
            "name = \"{}\"\nversion = \"{}\"",
            manifest.baseline.llama_cpp.sys_crate, manifest.baseline.llama_cpp.sys_crate_version
        )));
        assert!(lock.contains(&format!(
            "checksum = \"{}\"",
            manifest.baseline.llama_cpp.sys_crate_checksum
        )));
    }

    #[test]
    fn every_ci_reference_answer_passes_deterministic_scoring() {
        let suite = clinical_suite().unwrap();
        let ci: Vec<&ClinicalCase> = suite.cases.iter().filter(|case| case.ci).collect();
        assert!(!ci.is_empty());
        for case in ci {
            let score = score_case(case, &case.ci_reference_answer);
            assert!(score.passed, "{}: {:?}", case.id, score.checks);
        }
    }

    #[test]
    fn scorer_exposes_harmful_negation_and_dose_failures() {
        let suite = clinical_suite().unwrap();
        let negation = suite
            .cases
            .iter()
            .find(|case| case.id == "negation-risk-cold-de")
            .unwrap();
        let score = score_case(negation, "Akute Suizidalität besteht.");
        assert!(!score.passed);
        assert!(score.checks.iter().any(|check| !check.passed));

        let dose = suite
            .cases
            .iter()
            .find(|case| case.id == "medication-dose-cold-de")
            .unwrap();
        let score = score_case(dose, "Sertralin wird mit 100 mg verordnet.");
        assert!(!score.passed);
        assert!(score
            .checks
            .iter()
            .any(|check| check.check == "contains:150 mg" && !check.passed));
    }

    #[test]
    fn tool_call_requires_valid_json_name_and_argument_subset() {
        let suite = clinical_suite().unwrap();
        let case = suite
            .cases
            .iter()
            .find(|case| case.id == "list-medications-agent-tool")
            .unwrap();
        assert!(score_case(case, &case.ci_reference_answer).passed);
        assert!(
            !score_case(
                case,
                "<tool_call>{\"name\":\"list_medications\",\"args\":{}}</tool_call>"
            )
            .passed
        );
        assert!(!score_case(case, "<tool_call>{not-json}</tool_call>").passed);
        assert!(!score_case(
            case,
            "I will call it now: <tool_call>{\"name\":\"list_medications\",\"args\":{\"patient_id\":\"synthetic-patient-398\"}}</tool_call>"
        )
        .passed);
    }

    #[test]
    fn memory_summary_preserves_unknowns_and_signed_swap_delta() {
        let samples = vec![
            TimedMemorySnapshot {
                elapsed_ms: 0.0,
                memory: MemorySnapshot {
                    process_rss_bytes: Some(100),
                    system_wired_bytes: None,
                    system_compressed_bytes: Some(40),
                    system_swap_used_bytes: Some(10),
                },
            },
            TimedMemorySnapshot {
                elapsed_ms: 1.0,
                memory: MemorySnapshot {
                    process_rss_bytes: Some(150),
                    system_wired_bytes: None,
                    system_compressed_bytes: Some(30),
                    system_swap_used_bytes: Some(5),
                },
            },
        ];
        let summary = summarize_memory(&samples);
        assert_eq!(summary.peak.process_rss_bytes, Some(150));
        assert_eq!(summary.peak.system_wired_bytes, None);
        assert_eq!(summary.peak.system_compressed_bytes, Some(40));
        assert_eq!(summary.swap_delta_bytes, Some(-5));
    }
}
