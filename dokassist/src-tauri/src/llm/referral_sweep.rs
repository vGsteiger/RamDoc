//! Synthetic-only referral-letter sampler sweep.
//!
//! Raw generations are deliberately written only to an absolute path outside
//! the repository. The embedded fixture contains no real patient information.

use super::benchmark_harness::{score_case, ClinicalCase, ClinicalScore, ExpectedAnswer};
use super::engine::{GenerationStats, LlmEngine};
use super::harness::{GenerationProfile, GenerationTask, SamplerConfig};
use super::inference::InferenceDiagnostics;
use super::prompts::{self, ReportType, SYSTEM_PROMPT_DE};
use super::thinking::{self, ThinkingEffort};
use ring::digest::{Context as DigestContext, SHA256};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const SWEEP_JSON: &str =
    include_str!("../../../../benchmarks/local-inference/referral-quality-sweep.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SweepConfig {
    schema_version: u32,
    suite_id: String,
    data_classification: String,
    model: SweepModel,
    arms: Vec<SweepArm>,
    cases: Vec<ReferralCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SweepModel {
    filename: String,
    artifact_sha256: String,
    artifact_size_bytes: u64,
    inference_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SweepArm {
    id: String,
    description: String,
    thinking_effort: ThinkingEffort,
    no_think: bool,
    sampler: SamplerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReferralCase {
    id: String,
    split: String,
    patient_context: String,
    session_notes: String,
    additional_context: Option<String>,
    instructions: String,
    expected: ExpectedAnswer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SweepRun {
    arm_id: String,
    case_id: String,
    split: String,
    repetition: usize,
    sampler: SamplerConfig,
    thinking_effort: ThinkingEffort,
    no_think: bool,
    elapsed_ms: f64,
    raw_output: String,
    visible_output: String,
    score: ClinicalScore,
    stats: Option<GenerationStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ArmSummary {
    checks_passed: usize,
    checks_total: usize,
    cases_passed: usize,
    cases_total: usize,
    unsupported_claim_flags: usize,
    mean_elapsed_ms: f64,
    mean_visible_chars: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SweepResult {
    schema_version: u32,
    suite_id: String,
    data_classification: String,
    split: String,
    repetitions: usize,
    model_path_filename: String,
    model_sha256: String,
    git_commit: Option<String>,
    git_dirty: Option<bool>,
    host_os: String,
    host_architecture: String,
    physical_memory_bytes: u64,
    config_sha256: String,
    effective_inference: Option<InferenceDiagnostics>,
    runs: Vec<SweepRun>,
    arm_summaries: BTreeMap<String, ArmSummary>,
}

#[derive(Debug)]
struct RunOptions {
    model: PathBuf,
    output: PathBuf,
    split: String,
    arm_ids: Option<BTreeSet<String>>,
    repetitions: usize,
}

fn config() -> Result<SweepConfig, String> {
    serde_json::from_str(SWEEP_JSON).map_err(|error| format!("invalid sweep config: {error}"))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    hex::encode(ring::digest::digest(&SHA256, bytes).as_ref())
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

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn validate_config(config: &SweepConfig) -> Result<(), String> {
    if config.schema_version != 1 || config.data_classification != "synthetic_deidentified" {
        return Err("sweep must use schema v1 and synthetic_deidentified data".into());
    }
    if config.arms.is_empty() || config.cases.is_empty() {
        return Err("sweep requires at least one arm and one case".into());
    }
    let mut ids = BTreeSet::new();
    for arm in &config.arms {
        if !ids.insert(format!("arm:{}", arm.id)) {
            return Err(format!("duplicate arm id '{}'", arm.id));
        }
        arm.sampler.validate()?;
    }
    for case in &config.cases {
        if !ids.insert(format!("case:{}", case.id)) {
            return Err(format!("duplicate case id '{}'", case.id));
        }
        if !matches!(case.split.as_str(), "development" | "holdout") {
            return Err(format!("case '{}' has an invalid split", case.id));
        }
        if case.patient_context.trim().is_empty()
            || case.session_notes.trim().is_empty()
            || case.instructions.trim().is_empty()
        {
            return Err(format!("case '{}' has empty required input", case.id));
        }
    }
    Ok(())
}

fn parse_options(raw: &[String]) -> Result<RunOptions, String> {
    let mut values = BTreeMap::new();
    let mut index = 0;
    while index < raw.len() {
        let key = raw[index]
            .strip_prefix("--")
            .ok_or_else(|| format!("unexpected argument '{}'", raw[index]))?;
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
    let split = required("split")?;
    if !matches!(split.as_str(), "development" | "holdout" | "all") {
        return Err("--split must be development, holdout, or all".into());
    }
    let repetitions = values
        .get("repetitions")
        .map_or(Ok(1), |value| value.parse::<usize>())
        .map_err(|_| "--repetitions must be a positive integer".to_string())?;
    if repetitions == 0 || repetitions > 20 {
        return Err("--repetitions must be between 1 and 20".into());
    }
    let arm_ids = values.get("arms").map(|value| {
        value
            .split(',')
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .collect()
    });
    Ok(RunOptions {
        model: PathBuf::from(required("model")?),
        output: PathBuf::from(required("output")?),
        split,
        arm_ids,
        repetitions,
    })
}

fn validate_external_output(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("--output must be an absolute path outside the repository".into());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "--output must have a parent directory".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("create '{}': {error}", parent.display()))?;
    let parent = parent
        .canonicalize()
        .map_err(|error| format!("resolve '{}': {error}", parent.display()))?;
    if parent.starts_with(repository_root()) {
        return Err("raw generated letters must not be written inside the repository".into());
    }
    Ok(())
}

fn clean_visible_output(output: &str) -> String {
    super::harness::strip_think_blocks(output)
        .replace('ß', "ss")
        .lines()
        .filter(|line| {
            let normalized = line.trim().to_ascii_lowercase();
            !(normalized.starts_with("[name")
                || normalized.starts_with("[praxis")
                || normalized.starts_with("[adresse")
                || normalized.starts_with("[telefon")
                || normalized.starts_with("[email")
                || normalized.starts_with("[e-mail")
                || normalized.starts_with("[unterschrift"))
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn score_referral(case: &ReferralCase, answer: &str) -> ClinicalScore {
    score_case(
        &ClinicalCase {
            id: case.id.clone(),
            scenario: "cold_prompt".into(),
            categories: vec!["referral_letter".into()],
            ci: false,
            system_prompt: String::new(),
            context: case.patient_context.clone(),
            setup_prompt: None,
            question: case.instructions.clone(),
            max_tokens: 4096,
            pad_to_context: false,
            expected: case.expected.clone(),
            ci_reference_answer: String::new(),
        },
        answer,
    )
}

fn run_sweep(options: &RunOptions) -> Result<SweepResult, String> {
    let config = config()?;
    validate_config(&config)?;
    validate_external_output(&options.output)?;

    let metadata = std::fs::metadata(&options.model)
        .map_err(|error| format!("inspect '{}': {error}", options.model.display()))?;
    if metadata.len() != config.model.artifact_size_bytes {
        return Err("model size does not match the pinned sweep artifact".into());
    }
    let model_sha256 = sha256_path(&options.model)?;
    if !model_sha256.eq_ignore_ascii_case(&config.model.artifact_sha256) {
        return Err("model SHA-256 does not match the pinned sweep artifact".into());
    }

    if let Some(requested) = &options.arm_ids {
        let known: BTreeSet<&str> = config.arms.iter().map(|arm| arm.id.as_str()).collect();
        let unknown: Vec<&String> = requested
            .iter()
            .filter(|id| !known.contains(id.as_str()))
            .collect();
        if !unknown.is_empty() {
            return Err(format!("unknown arms: {unknown:?}"));
        }
    }

    let model_name = options
        .model
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "model path has no UTF-8 filename".to_string())?
        .to_string();
    let engine = LlmEngine::load_with_profile(
        options.model.clone(),
        model_name.clone(),
        &config.model.inference_profile,
    )
    .map_err(|error| error.to_string())?;
    let effective_inference = engine.status().inference_config;

    let mut runs = Vec::new();
    for arm in config.arms.iter().filter(|arm| {
        options
            .arm_ids
            .as_ref()
            .is_none_or(|ids| ids.contains(&arm.id))
    }) {
        for case in config
            .cases
            .iter()
            .filter(|case| options.split == "all" || case.split == options.split)
        {
            for repetition in 0..options.repetitions {
                let mut sampler = arm.sampler;
                sampler.seed = sampler.seed.wrapping_add(repetition as u32);
                let profile = GenerationProfile {
                    sampler,
                    effort: arm.thinking_effort,
                    max_tokens: arm.thinking_effort.max_tokens(),
                };
                let mut user_message = prompts::report_generation_prompt(
                    ReportType::Ueberweisungsschreiben,
                    &case.patient_context,
                    &case.session_notes,
                    case.additional_context.as_deref(),
                    Some(&case.instructions),
                );
                if arm.no_think {
                    user_message.push_str("\n\n/no_think");
                }
                let system_prompt = arm.thinking_effort.apply_to_system_prompt(SYSTEM_PROMPT_DE);
                let system_prompt = GenerationTask::Report.apply_to_system_prompt(&system_prompt);
                let started = Instant::now();
                let raw_output = thinking::generate_with_think_budget_from_prompt(
                    &engine,
                    &system_prompt,
                    None,
                    &user_message,
                    profile,
                    None,
                    &|_| {},
                )
                .map_err(|error| format!("arm '{}' case '{}': {error}", arm.id, case.id))?;
                let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;
                let visible_output = clean_visible_output(&raw_output);
                let score = score_referral(case, &visible_output);
                runs.push(SweepRun {
                    arm_id: arm.id.clone(),
                    case_id: case.id.clone(),
                    split: case.split.clone(),
                    repetition,
                    sampler,
                    thinking_effort: arm.thinking_effort,
                    no_think: arm.no_think,
                    elapsed_ms,
                    raw_output,
                    visible_output,
                    score,
                    stats: engine.last_generation_stats(),
                });
            }
        }
    }

    let mut arm_summaries = BTreeMap::new();
    for run in &runs {
        let summary = arm_summaries
            .entry(run.arm_id.clone())
            .or_insert_with(ArmSummary::default);
        summary.checks_passed += run.score.checks_passed;
        summary.checks_total += run.score.checks_total;
        summary.cases_total += 1;
        summary.cases_passed += usize::from(run.score.passed);
        summary.unsupported_claim_flags += run
            .score
            .checks
            .iter()
            .filter(|check| check.check.starts_with("excludes:") && !check.passed)
            .count();
        summary.mean_elapsed_ms += run.elapsed_ms;
        summary.mean_visible_chars += run.visible_output.chars().count() as f64;
    }
    for summary in arm_summaries.values_mut() {
        if summary.cases_total > 0 {
            summary.mean_elapsed_ms /= summary.cases_total as f64;
            summary.mean_visible_chars /= summary.cases_total as f64;
        }
    }

    let git_commit = Command::new("git")
        .arg("-C")
        .arg(repository_root())
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string());
    let git_dirty = Command::new("git")
        .arg("-C")
        .arg(repository_root())
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| !output.stdout.is_empty());
    Ok(SweepResult {
        schema_version: 1,
        suite_id: config.suite_id,
        data_classification: config.data_classification,
        split: options.split.clone(),
        repetitions: options.repetitions,
        model_path_filename: model_name,
        model_sha256,
        git_commit,
        git_dirty,
        host_os: std::env::consts::OS.into(),
        host_architecture: std::env::consts::ARCH.into(),
        physical_memory_bytes: LlmEngine::total_ram(),
        config_sha256: sha256_bytes(SWEEP_JSON.as_bytes()),
        effective_inference,
        runs,
        arm_summaries,
    })
}

fn write_result(path: &Path, result: &SweepResult) -> Result<(), String> {
    let json = serde_json::to_string_pretty(result)
        .map_err(|error| format!("serialize sweep result: {error}"))?;
    std::fs::write(path, format!("{json}\n"))
        .map_err(|error| format!("write '{}': {error}", path.display()))
}

pub fn cli_main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        return Err("usage: referral-quality-sweep <validate|run> [options]".into());
    };
    match command {
        "validate" => {
            let config = config()?;
            validate_config(&config)?;
            println!(
                "{}",
                serde_json::json!({
                    "suite_id": config.suite_id,
                    "arms": config.arms.len(),
                    "development_cases": config.cases.iter().filter(|case| case.split == "development").count(),
                    "holdout_cases": config.cases.iter().filter(|case| case.split == "holdout").count(),
                    "data_classification": config.data_classification,
                })
            );
            Ok(())
        }
        "run" => {
            let options = parse_options(&args[1..])?;
            let result = run_sweep(&options)?;
            write_result(&options.output, &result)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&result.arm_summaries)
                    .map_err(|error| format!("serialize summaries: {error}"))?
            );
            eprintln!(
                "raw synthetic generations written to {}",
                options.output.display()
            );
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
    fn embedded_sweep_is_synthetic_and_partitioned() {
        let config = config().unwrap();
        validate_config(&config).unwrap();
        assert_eq!(config.arms.len(), 9);
        assert_eq!(
            config
                .cases
                .iter()
                .filter(|case| case.split == "development")
                .count(),
            3
        );
        assert_eq!(
            config
                .cases
                .iter()
                .filter(|case| case.split == "holdout")
                .count(),
            2
        );
    }

    #[test]
    fn raw_outputs_are_rejected_inside_the_repository() {
        let path = repository_root().join("benchmark-results/referral.json");
        assert!(validate_external_output(&path).is_err());
    }

    #[test]
    fn visible_cleanup_preserves_clinical_brackets() {
        let cleaned = clean_visible_output(
            "<think>hidden</think>\nF33.1 [gesichert]\n[Name des Psychiaters]\nGrüße",
        );
        assert_eq!(cleaned, "F33.1 [gesichert]\nGrüsse");
    }
}
