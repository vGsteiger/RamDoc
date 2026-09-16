#!/usr/bin/env python3
"""Orchestrate RamDoc's reproducible local-inference benchmark.

Only the Python standard library is used. The model-backed worker is the Rust
`local-inference-benchmark` binary, run once per context/repetition so process
RSS and llama.cpp state are isolated. This script owns artifact verification,
JSON/CSV/Markdown collation, and build-to-build regression comparison.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import os
import platform
import statistics
import subprocess
import sys
import tempfile
import time
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable


REPO_ROOT = Path(__file__).resolve().parents[1]
TAURI_DIR = REPO_ROOT / "dokassist" / "src-tauri"
MANIFEST_PATH = REPO_ROOT / "benchmarks" / "local-inference" / "manifest.json"
CASES_PATH = REPO_ROOT / "benchmarks" / "local-inference" / "clinical-cases.json"
BINARY_PATH = TAURI_DIR / "target" / "release" / "local-inference-benchmark"


class BenchmarkError(RuntimeError):
    pass


def read_json(path: Path) -> dict[str, Any]:
    try:
        with path.open(encoding="utf-8") as handle:
            value = json.load(handle)
    except (OSError, json.JSONDecodeError) as error:
        raise BenchmarkError(f"cannot read {path}: {error}") from error
    if not isinstance(value, dict):
        raise BenchmarkError(f"{path} must contain a JSON object")
    return value


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, indent=2, ensure_ascii=False, sort_keys=True) + "\n"
    with tempfile.NamedTemporaryFile(
        mode="w", encoding="utf-8", dir=path.parent, delete=False
    ) as handle:
        handle.write(payload)
        temporary = Path(handle.name)
    temporary.replace(path)


def sha256_file(path: Path) -> tuple[str, float]:
    digest = hashlib.sha256()
    started = time.perf_counter()
    try:
        with path.open("rb") as handle:
            while chunk := handle.read(8 * 1024 * 1024):
                digest.update(chunk)
    except OSError as error:
        raise BenchmarkError(f"cannot hash {path}: {error}") from error
    return digest.hexdigest(), (time.perf_counter() - started) * 1000.0


def validate_files(manifest: dict[str, Any], cases: dict[str, Any]) -> None:
    if manifest.get("schema_version") != 1 or cases.get("schema_version") != 1:
        raise BenchmarkError("only benchmark schema version 1 is supported")
    if manifest.get("data_classification") != "synthetic_deidentified":
        raise BenchmarkError("manifest must be classified synthetic_deidentified")
    if cases.get("data_classification") != "synthetic_deidentified":
        raise BenchmarkError("clinical cases must be classified synthetic_deidentified")
    contexts = [item.get("context_tokens") for item in manifest.get("context_profiles", [])]
    if contexts != [2048, 8192, 16384, 32768, 65536, 131072]:
        raise BenchmarkError("context matrix must be 2K, 8K, 16K, 32K, 64K, 128K")
    scenarios = set(manifest.get("scenarios", []))
    expected_scenarios = {
        "cold_prompt",
        "shared_prefix",
        "continued_session",
        "agent_tool_call",
    }
    if scenarios != expected_scenarios:
        raise BenchmarkError("manifest does not declare the four required scenarios")
    rows = cases.get("cases")
    if not isinstance(rows, list) or not rows:
        raise BenchmarkError("clinical suite must contain cases")
    ids = [row.get("id") for row in rows]
    if len(ids) != len(set(ids)):
        raise BenchmarkError("clinical case IDs must be unique")
    covered_scenarios = {row.get("scenario") for row in rows}
    if covered_scenarios != expected_scenarios:
        raise BenchmarkError("clinical cases do not cover every scenario")
    covered_categories = {
        category for row in rows for category in row.get("categories", [])
    }
    missing = set(manifest.get("required_categories", [])) - covered_categories
    if missing:
        raise BenchmarkError(f"clinical cases are missing categories: {sorted(missing)}")
    if not any(row.get("pad_to_context") for row in rows):
        raise BenchmarkError("clinical suite has no middle-of-context padded case")
    ci_count = sum(bool(row.get("ci")) for row in rows)
    if ci_count == 0 or ci_count == len(rows):
        raise BenchmarkError("CI subset must be non-empty and smaller than the full suite")


def mean(values: Iterable[float | int | None]) -> float | None:
    available = [float(value) for value in values if value is not None]
    return statistics.fmean(available) if available else None


def maximum(values: Iterable[int | float | None]) -> int | float | None:
    available = [value for value in values if value is not None]
    return max(available) if available else None


def runnable_runs(suite: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        run
        for run in suite.get("runs", [])
        if run.get("status") in {"completed", "quality_failed"}
    ]


def aggregate_suite(runs: list[dict[str, Any]]) -> dict[str, Any]:
    status_counts = Counter(run.get("status", "unknown") for run in runs)
    context_groups: dict[tuple[str, int], list[dict[str, Any]]] = defaultdict(list)
    category_counts: dict[str, dict[str, int]] = defaultdict(
        lambda: {"passed": 0, "total": 0}
    )
    scenario_values: dict[str, dict[str, list[float]]] = defaultdict(
        lambda: defaultdict(list)
    )

    for run in runs:
        config = run.get("requested_configuration", {})
        key = (str(config.get("label", "unknown")), int(config.get("context_tokens", 0)))
        context_groups[key].append(run)
        for result in run.get("scenarios", []):
            passed = bool(result.get("score", {}).get("passed"))
            for category in result.get("categories", []):
                category_counts[category]["total"] += 1
                category_counts[category]["passed"] += int(passed)
            stats = result.get("stats") or {}
            scenario = str(result.get("scenario", "unknown"))
            for metric in ("ttft_ms", "prefill_ms", "total_latency_ms", "tps"):
                value = stats.get(metric)
                if isinstance(value, (int, float)) and math.isfinite(value):
                    scenario_values[scenario][metric].append(float(value))

    contexts = []
    for (label, tokens), group in sorted(context_groups.items(), key=lambda item: item[0][1]):
        active = [
            run
            for run in group
            if run.get("status") in {"completed", "quality_failed"}
        ]
        memory = [run.get("memory", {}) for run in active]
        contexts.append(
            {
                "label": label,
                "context_tokens": tokens,
                "statuses": dict(Counter(run.get("status", "unknown") for run in group)),
                "load_ms_mean": mean(run.get("load_ms") for run in active),
                "load_ms_first": next(
                    (run.get("load_ms") for run in active if run.get("repetition") == 0),
                    None,
                ),
                "load_ms_warm_mean": mean(
                    run.get("load_ms")
                    for run in active
                    if int(run.get("repetition", 0)) > 0
                ),
                "peak_process_rss_bytes": maximum(
                    item.get("peak", {}).get("process_rss_bytes") for item in memory
                ),
                "steady_process_rss_bytes_mean": mean(
                    item.get("steady", {}).get("process_rss_bytes") for item in memory
                ),
                "peak_system_wired_bytes": maximum(
                    item.get("peak", {}).get("system_wired_bytes") for item in memory
                ),
                "peak_system_compressed_bytes": maximum(
                    item.get("peak", {}).get("system_compressed_bytes") for item in memory
                ),
                "max_swap_delta_bytes": maximum(
                    item.get("swap_delta_bytes") for item in memory
                ),
                "cases_passed": sum(int(run.get("cases_passed", 0)) for run in active),
                "cases_total": sum(int(run.get("cases_total", 0)) for run in active),
            }
        )

    categories = {
        category: {
            **counts,
            "score": counts["passed"] / counts["total"] if counts["total"] else None,
        }
        for category, counts in sorted(category_counts.items())
    }
    scenarios = {
        scenario: {metric: mean(values) for metric, values in sorted(metrics.items())}
        for scenario, metrics in sorted(scenario_values.items())
    }
    return {
        "status_counts": dict(status_counts),
        "contexts": contexts,
        "categories": categories,
        "scenarios": scenarios,
    }


def scenario_groups(suite: dict[str, Any]) -> dict[tuple[str, str, str], dict[str, Any]]:
    groups: dict[tuple[str, str, str], dict[str, Any]] = defaultdict(
        lambda: {
            "passed": [],
            "answers": [],
            "ttft_ms": [],
            "prefill_ms": [],
            "total_latency_ms": [],
            "tps": [],
        }
    )
    for run in runnable_runs(suite):
        config = run.get("requested_configuration", {})
        label = str(config.get("label", "unknown"))
        for result in run.get("scenarios", []):
            key = (label, str(result.get("scenario")), str(result.get("case_id")))
            group = groups[key]
            group["passed"].append(bool(result.get("score", {}).get("passed")))
            group["answers"].append(str(result.get("answer", "")))
            stats = result.get("stats") or {}
            for metric in ("ttft_ms", "prefill_ms", "total_latency_ms", "tps"):
                value = stats.get(metric)
                if isinstance(value, (int, float)) and math.isfinite(value):
                    group[metric].append(float(value))
    return groups


def context_groups(suite: dict[str, Any]) -> dict[str, list[dict[str, Any]]]:
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for run in suite.get("runs", []):
        label = str(run.get("requested_configuration", {}).get("label", "unknown"))
        groups[label].append(run)
    return groups


def append_regression(
    regressions: list[dict[str, Any]],
    kind: str,
    key: str,
    baseline: Any,
    candidate: Any,
    limit: Any,
    message: str,
) -> None:
    regressions.append(
        {
            "kind": kind,
            "key": key,
            "baseline": baseline,
            "candidate": candidate,
            "limit": limit,
            "message": message,
        }
    )


def compare_suites(
    baseline: dict[str, Any],
    candidate: dict[str, Any],
    thresholds: dict[str, Any],
) -> dict[str, Any]:
    if baseline.get("suite_id") != candidate.get("suite_id"):
        raise BenchmarkError("benchmark suites use different suite IDs")
    regressions: list[dict[str, Any]] = []
    warnings: list[str] = []
    answer_changes: list[dict[str, str]] = []
    baseline_model = baseline.get("model", {})
    candidate_model = candidate.get("model", {})
    if baseline_model.get("artifact_sha256") != candidate_model.get("artifact_sha256"):
        warnings.append("model artifact hashes differ; this is a model comparison, not a build-only comparison")
    if baseline.get("manifest_sha256") != candidate.get("manifest_sha256"):
        warnings.append("benchmark manifest hashes differ; inspect contract changes before attributing deltas to the build")

    baseline_cases = scenario_groups(baseline)
    candidate_cases = scenario_groups(candidate)
    quality_drop_limit = float(thresholds["max_quality_score_drop"])
    for key, base in sorted(baseline_cases.items()):
        key_text = "/".join(key)
        current = candidate_cases.get(key)
        if current is None:
            append_regression(
                regressions,
                "missing_case",
                key_text,
                len(base["passed"]),
                0,
                0,
                "candidate has no comparable result for a baseline case",
            )
            continue
        base_score = sum(base["passed"]) / len(base["passed"])
        current_score = sum(current["passed"]) / len(current["passed"])
        if base_score - current_score > quality_drop_limit:
            append_regression(
                regressions,
                "quality",
                key_text,
                base_score,
                current_score,
                quality_drop_limit,
                "deterministic clinical pass rate decreased",
            )
        if set(base["answers"]) != set(current["answers"]):
            answer_changes.append(
                {
                    "key": key_text,
                    "baseline_sha256": hashlib.sha256(
                        "\n".join(base["answers"]).encode()
                    ).hexdigest(),
                    "candidate_sha256": hashlib.sha256(
                        "\n".join(current["answers"]).encode()
                    ).hexdigest(),
                }
            )

        increase_limits = {
            "ttft_ms": "max_ttft_increase_ratio",
            "prefill_ms": "max_prefill_increase_ratio",
            "total_latency_ms": "max_total_latency_increase_ratio",
        }
        for metric, threshold_name in increase_limits.items():
            base_mean = mean(base[metric])
            current_mean = mean(current[metric])
            if base_mean and current_mean is not None:
                increase = current_mean / base_mean - 1.0
                limit = float(thresholds[threshold_name])
                if increase > limit:
                    append_regression(
                        regressions,
                        metric,
                        key_text,
                        base_mean,
                        current_mean,
                        limit,
                        f"{metric} increased by {increase:.1%}",
                    )
        base_tps = mean(base["tps"])
        current_tps = mean(current["tps"])
        if base_tps and current_tps is not None:
            decrease = 1.0 - current_tps / base_tps
            limit = float(thresholds["max_decode_throughput_decrease_ratio"])
            if decrease > limit:
                append_regression(
                    regressions,
                    "tps",
                    key_text,
                    base_tps,
                    current_tps,
                    limit,
                    f"decode throughput decreased by {decrease:.1%}",
                )

    base_contexts = context_groups(baseline)
    current_contexts = context_groups(candidate)
    for label, base_runs in sorted(base_contexts.items()):
        base_active = [
            run for run in base_runs if run.get("status") in {"completed", "quality_failed"}
        ]
        if not base_active:
            continue
        current_active = [
            run
            for run in current_contexts.get(label, [])
            if run.get("status") in {"completed", "quality_failed"}
        ]
        if not current_active:
            append_regression(
                regressions,
                "missing_context",
                label,
                "runnable",
                "not runnable",
                0,
                "candidate cannot run a context tier that the baseline ran",
            )
            continue
        base_peak = maximum(
            run.get("memory", {}).get("peak", {}).get("process_rss_bytes")
            for run in base_active
        )
        current_peak = maximum(
            run.get("memory", {}).get("peak", {}).get("process_rss_bytes")
            for run in current_active
        )
        if base_peak is not None and current_peak is not None:
            increase = int(current_peak) - int(base_peak)
            limit = int(thresholds["max_peak_rss_increase_bytes"])
            if increase > limit:
                append_regression(
                    regressions,
                    "peak_rss",
                    label,
                    base_peak,
                    current_peak,
                    limit,
                    f"whole-process peak RSS increased by {format_bytes(increase)}",
                )
        swap_limit = int(thresholds["max_swap_growth_bytes"])
        for run in current_active:
            swap = run.get("memory", {}).get("swap_delta_bytes")
            if swap is None:
                append_regression(
                    regressions,
                    "swap_unavailable",
                    label,
                    "measured",
                    None,
                    swap_limit,
                    "candidate swap telemetry is unavailable",
                )
                break
            if int(swap) > swap_limit:
                append_regression(
                    regressions,
                    "swap_growth",
                    label,
                    0,
                    swap,
                    swap_limit,
                    "candidate grew system swap",
                )
                break

    return {
        "status": "regression" if regressions else "pass",
        "regressions": regressions,
        "warnings": warnings,
        "deterministic_answer_changes": answer_changes,
        "baseline_build": baseline.get("build"),
        "candidate_build": candidate.get("build"),
    }


CSV_FIELDS = [
    "suite_id",
    "run_id",
    "git_commit",
    "model_sha256",
    "status",
    "reason",
    "load_state",
    "repetition",
    "profile",
    "context_tokens",
    "kv_cache",
    "n_batch",
    "n_ubatch",
    "load_ms",
    "scenario",
    "case_id",
    "categories",
    "passed",
    "checks_passed",
    "checks_total",
    "prompt_tokens",
    "evaluated_prompt_tokens",
    "reused_prompt_tokens",
    "completion_tokens",
    "ttft_ms",
    "prefill_ms",
    "total_latency_ms",
    "tps",
    "peak_process_rss_bytes",
    "steady_process_rss_bytes",
    "peak_system_wired_bytes",
    "peak_system_compressed_bytes",
    "swap_delta_bytes",
    "error",
]


def csv_rows(suite: dict[str, Any]) -> Iterable[dict[str, Any]]:
    for run in suite.get("runs", []):
        config = run.get("requested_configuration", {})
        build = run.get("build", {})
        model = run.get("model", {})
        memory = run.get("memory", {})
        base = {
            "suite_id": run.get("suite_id"),
            "run_id": run.get("run_id"),
            "git_commit": build.get("git_commit"),
            "model_sha256": model.get("artifact_sha256"),
            "status": run.get("status"),
            "reason": run.get("reason"),
            "load_state": run.get("load_state"),
            "repetition": run.get("repetition"),
            "profile": config.get("label"),
            "context_tokens": config.get("context_tokens"),
            "kv_cache": config.get("kv_cache"),
            "n_batch": config.get("n_batch"),
            "n_ubatch": config.get("n_ubatch"),
            "load_ms": run.get("load_ms"),
            "peak_process_rss_bytes": memory.get("peak", {}).get("process_rss_bytes"),
            "steady_process_rss_bytes": memory.get("steady", {}).get(
                "process_rss_bytes"
            ),
            "peak_system_wired_bytes": memory.get("peak", {}).get(
                "system_wired_bytes"
            ),
            "peak_system_compressed_bytes": memory.get("peak", {}).get(
                "system_compressed_bytes"
            ),
            "swap_delta_bytes": memory.get("swap_delta_bytes"),
        }
        scenarios = run.get("scenarios", [])
        if not scenarios:
            yield base
            continue
        for result in scenarios:
            score = result.get("score", {})
            stats = result.get("stats") or {}
            yield {
                **base,
                "scenario": result.get("scenario"),
                "case_id": result.get("case_id"),
                "categories": ";".join(result.get("categories", [])),
                "passed": score.get("passed"),
                "checks_passed": score.get("checks_passed"),
                "checks_total": score.get("checks_total"),
                "prompt_tokens": stats.get("prompt_tokens"),
                "evaluated_prompt_tokens": stats.get("evaluated_prompt_tokens"),
                "reused_prompt_tokens": stats.get("reused_prompt_tokens"),
                "completion_tokens": stats.get("completion_tokens"),
                "ttft_ms": stats.get("ttft_ms"),
                "prefill_ms": stats.get("prefill_ms"),
                "total_latency_ms": stats.get("total_latency_ms"),
                "tps": stats.get("tps"),
                "error": result.get("error"),
            }


def write_csv(path: Path, suite: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=CSV_FIELDS, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(csv_rows(suite))


def format_bytes(value: int | float | None) -> str:
    if value is None:
        return "unavailable"
    amount = float(value)
    sign = "-" if amount < 0 else ""
    amount = abs(amount)
    for unit in ("B", "KiB", "MiB", "GiB", "TiB"):
        if amount < 1024.0 or unit == "TiB":
            return f"{sign}{amount:.2f} {unit}"
        amount /= 1024.0
    raise AssertionError("unreachable")


def format_ms(value: int | float | None) -> str:
    return "unavailable" if value is None else f"{float(value):.1f}"


def render_report(suite: dict[str, Any], comparison: dict[str, Any] | None) -> str:
    summary = suite["summary"]
    model = suite.get("model", {})
    build = suite.get("build", {})
    first_load_ms = next(
        (
            run.get("load_ms")
            for run in suite.get("runs", [])
            if run.get("load_ms") is not None
        ),
        None,
    )
    verified_cold_path_ms = (
        float(suite.get("artifact_verification_ms")) + float(first_load_ms)
        if suite.get("artifact_verification_ms") is not None
        and first_load_ms is not None
        else None
    )
    lines = [
        "# RamDoc local-inference benchmark",
        "",
        f"- Suite: `{suite.get('suite_id')}`",
        f"- Generated: {suite.get('generated_at')}",
        f"- Model: `{model.get('filename', 'unknown')}` ({model.get('quantization', 'unknown')})",
        f"- Artifact SHA-256: `{model.get('artifact_sha256', 'unknown')}`",
        f"- RamDoc commit: `{build.get('git_commit') or 'unknown'}` (dirty: `{build.get('git_dirty')}`)",
        f"- llama.cpp Rust binding: `{build.get('llama_cpp', {}).get('sys_crate')} {build.get('llama_cpp', {}).get('sys_crate_version')}`",
        f"- Artifact verification: {format_ms(suite.get('artifact_verification_ms'))} ms",
        f"- Verified cold path to model-ready: {format_ms(verified_cold_path_ms)} ms",
        "",
        "## Context and memory",
        "",
        "| Tier | Status | First load ms | Warm load ms | Peak whole-process RSS | Steady RSS | Peak wired | Peak compressed | Max swap delta | Clinical |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for context in summary.get("contexts", []):
        statuses = ", ".join(
            f"{name}×{count}" for name, count in sorted(context["statuses"].items())
        )
        clinical = f"{context['cases_passed']}/{context['cases_total']}"
        lines.append(
            "| {label} | {statuses} | {first} | {warm} | {peak} | {steady} | {wired} | {compressed} | {swap} | {clinical} |".format(
                label=context["label"],
                statuses=statuses,
                first=format_ms(context.get("load_ms_first")),
                warm=format_ms(context.get("load_ms_warm_mean")),
                peak=format_bytes(context.get("peak_process_rss_bytes")),
                steady=format_bytes(context.get("steady_process_rss_bytes_mean")),
                wired=format_bytes(context.get("peak_system_wired_bytes")),
                compressed=format_bytes(context.get("peak_system_compressed_bytes")),
                swap=format_bytes(context.get("max_swap_delta_bytes")),
                clinical=clinical,
            )
        )

    lines.extend(
        [
            "",
            "RSS is sampled from the complete benchmark process that owns RamDoc's Rust runtime, model, KV cache, contexts, and allocators. Wired and compressed values are system-wide macOS pressure context.",
            "",
            "The first-load figure is a fresh process after the mandatory artifact hash pass; the artifact-verification duration is reported separately. Warm figures are fresh processes benefiting from the filesystem cache—this harness does not claim to evict macOS caches.",
            "",
            "## Clinical categories",
            "",
            "| Category | Passed | Total | Score |",
            "| --- | ---: | ---: | ---: |",
        ]
    )
    for category, score in summary.get("categories", {}).items():
        rendered = "unavailable" if score["score"] is None else f"{score['score']:.1%}"
        lines.append(
            f"| {category} | {score['passed']} | {score['total']} | {rendered} |"
        )

    lines.extend(
        [
            "",
            "## Scenario performance",
            "",
            "| Scenario | TTFT ms | Prefill ms | Total ms | Decode tok/s |",
            "| --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for scenario, metrics in summary.get("scenarios", {}).items():
        lines.append(
            f"| {scenario} | {format_ms(metrics.get('ttft_ms'))} | {format_ms(metrics.get('prefill_ms'))} | {format_ms(metrics.get('total_latency_ms'))} | {format_ms(metrics.get('tps'))} |"
        )

    skipped = [run for run in suite.get("runs", []) if run.get("status") == "skipped"]
    failed = [
        run
        for run in suite.get("runs", [])
        if run.get("status") in {"failed", "quality_failed"}
    ]
    if skipped:
        lines.extend(["", "## Skipped tiers", ""])
        for run in skipped:
            label = run.get("requested_configuration", {}).get("label")
            lines.append(f"- `{label}`: {run.get('reason')}")
    if failed:
        lines.extend(["", "## Failures", ""])
        for run in failed:
            label = run.get("requested_configuration", {}).get("label")
            lines.append(f"- `{label}` ({run.get('status')}): {run.get('reason') or 'one or more deterministic cases failed'}")

    if comparison is not None:
        lines.extend(
            [
                "",
                "## Build comparison",
                "",
                f"Verdict: **{comparison['status']}**",
                "",
            ]
        )
        for warning in comparison.get("warnings", []):
            lines.append(f"- Warning: {warning}")
        for regression in comparison.get("regressions", []):
            lines.append(f"- Regression `{regression['kind']}` at `{regression['key']}`: {regression['message']}")
        if not comparison.get("regressions"):
            lines.append("No declared quality, latency, throughput, memory, or swap threshold regressed.")
        changes = comparison.get("deterministic_answer_changes", [])
        if changes:
            lines.append(f"\nDeterministic answers changed in {len(changes)} comparable case(s); hashes are retained in `comparison.json` for review.")

    return "\n".join(lines) + "\n"


def select_profiles(
    manifest: dict[str, Any], requested: str | None, quick: bool
) -> list[dict[str, Any]]:
    profiles = list(manifest["context_profiles"])
    if requested:
        wanted = {item.strip().lower() for item in requested.split(",") if item.strip()}
        selected = [
            profile
            for profile in profiles
            if profile["label"].lower() in wanted
            or str(profile["context_tokens"]) in wanted
        ]
        found = {
            profile["label"].lower() for profile in selected
        } | {str(profile["context_tokens"]) for profile in selected}
        unknown = wanted - found
        if unknown:
            raise BenchmarkError(f"unknown context profiles: {sorted(unknown)}")
        return selected
    if quick:
        return [profile for profile in profiles if profile["label"] == "16k"]
    return profiles


def build_worker() -> None:
    features = ["benchmark-harness", "metal"]
    command = [
        "cargo",
        "build",
        "--release",
        "--features",
        ",".join(features),
        "--bin",
        "local-inference-benchmark",
    ]
    print("Building benchmark worker…", flush=True)
    completed = subprocess.run(command, cwd=TAURI_DIR, check=False)
    if completed.returncode != 0:
        raise BenchmarkError("benchmark worker build failed")


def run_worker(
    model: Path,
    model_hash: str,
    quantization: str,
    profile: dict[str, Any],
    load_state: str,
    repetition: int,
    run_id: str,
    output: Path,
    quick: bool,
) -> dict[str, Any]:
    command = [
        str(BINARY_PATH),
        "run",
        "--model",
        str(model),
        "--model-sha256",
        model_hash,
        "--artifact-quantization",
        quantization,
        "--profile-label",
        str(profile["label"]),
        "--context",
        str(profile["context_tokens"]),
        "--kv-cache",
        str(profile["kv_cache"]),
        "--n-batch",
        str(profile["n_batch"]),
        "--n-ubatch",
        str(profile["n_ubatch"]),
        "--headroom",
        str(profile["completion_headroom"]),
        "--load-state",
        load_state,
        "--repetition",
        str(repetition),
        "--run-id",
        run_id,
        "--output",
        str(output),
    ]
    if quick:
        command.append("--quick")
    completed = subprocess.run(command, cwd=REPO_ROOT, check=False)
    if completed.returncode != 0:
        raise BenchmarkError(
            f"benchmark worker exited {completed.returncode} for {profile['label']} repetition {repetition}"
        )
    return read_json(output)


def command_ci(_args: argparse.Namespace) -> int:
    manifest = read_json(MANIFEST_PATH)
    cases = read_json(CASES_PATH)
    validate_files(manifest, cases)
    command = ["cargo", "test", "--lib", "llm::benchmark_harness::tests"]
    completed = subprocess.run(command, cwd=TAURI_DIR, check=False)
    if completed.returncode != 0:
        return completed.returncode

    synthetic_run = {
        "status": "completed",
        "requested_configuration": {"label": "16k", "context_tokens": 16384},
        "memory": {
            "peak": {"process_rss_bytes": 100},
            "steady": {"process_rss_bytes": 90},
            "swap_delta_bytes": 0,
        },
        "scenarios": [
            {
                "scenario": "cold_prompt",
                "case_id": "synthetic",
                "answer": "ok",
                "categories": ["medication"],
                "score": {"passed": True},
                "stats": {
                    "ttft_ms": 10.0,
                    "prefill_ms": 5.0,
                    "total_latency_ms": 20.0,
                    "tps": 30.0,
                },
            }
        ],
    }
    base = {
        "schema_version": 1,
        "suite_id": manifest["suite_id"],
        "generated_at": "deterministic-ci",
        "artifact_verification_ms": 1.0,
        "build": {"git_commit": "ci", "git_dirty": False, "llama_cpp": {}},
        "model": {
            "filename": "synthetic.gguf",
            "artifact_sha256": "a",
            "quantization": "synthetic",
        },
        "runs": [synthetic_run],
        "summary": aggregate_suite([synthetic_run]),
    }
    same = json.loads(json.dumps(base))
    comparison = compare_suites(base, same, manifest["regression_thresholds"])
    if comparison["status"] != "pass":
        raise BenchmarkError("comparison self-test failed to accept identical suites")
    degraded = json.loads(json.dumps(base))
    degraded["runs"][0]["scenarios"][0]["score"]["passed"] = False
    comparison = compare_suites(base, degraded, manifest["regression_thresholds"])
    if comparison["status"] != "regression":
        raise BenchmarkError("comparison self-test failed to detect a quality regression")
    slower = json.loads(json.dumps(base))
    slower["runs"][0]["scenarios"][0]["stats"]["total_latency_ms"] = 40.0
    comparison = compare_suites(base, slower, manifest["regression_thresholds"])
    if not any(item["kind"] == "total_latency_ms" for item in comparison["regressions"]):
        raise BenchmarkError("comparison self-test failed to detect a latency regression")
    report = render_report(base, None)
    if "Context and memory" not in report or "Clinical categories" not in report:
        raise BenchmarkError("report self-test did not render required sections")
    with tempfile.TemporaryDirectory(prefix="ramdoc-benchmark-ci-") as directory:
        csv_path = Path(directory) / "results.csv"
        write_csv(csv_path, base)
        if len(csv_path.read_text(encoding="utf-8").splitlines()) != 2:
            raise BenchmarkError("CSV self-test did not emit one row per synthetic case")
    print("Deterministic benchmark subset passed.")
    return 0


def command_run(args: argparse.Namespace) -> int:
    if platform.system() != "Darwin":
        raise BenchmarkError("full local-inference benchmarks require macOS")
    if platform.machine() != "arm64":
        raise BenchmarkError(
            "full local-inference benchmarks must run natively on Apple Silicon; refusing a non-Metal comparison"
        )
    manifest = read_json(MANIFEST_PATH)
    cases = read_json(CASES_PATH)
    validate_files(manifest, cases)
    model = Path(args.model).expanduser().resolve()
    if not model.is_file():
        raise BenchmarkError(f"model does not exist: {model}")
    profiles = select_profiles(manifest, args.contexts, args.quick)
    baseline_context = int(manifest["baseline"]["configuration"]["context_tokens"])
    profiles.sort(
        key=lambda profile: (
            int(profile["context_tokens"]) != baseline_context,
            int(profile["context_tokens"]),
        )
    )
    repetitions = args.repetitions
    if repetitions is None:
        repetitions = 1 if args.quick else int(manifest["repetitions"])
    if repetitions < 1:
        raise BenchmarkError("repetitions must be positive")

    baseline_model = manifest["baseline"]["model"]
    if model.name == baseline_model["filename"]:
        quantization = args.quantization or baseline_model["quantization"]
    elif args.quantization:
        quantization = args.quantization
    else:
        raise BenchmarkError("--quantization is required for a model not captured in the baseline")

    print(f"Verifying {model.name} SHA-256…", flush=True)
    model_hash, verification_ms = sha256_file(model)
    model_size = model.stat().st_size
    if model.name == baseline_model["filename"]:
        if (
            model_hash.lower() != baseline_model["artifact_sha256"].lower()
            or model_size != baseline_model["artifact_size_bytes"]
            or quantization != baseline_model["quantization"]
        ):
            raise BenchmarkError("Qwen3-8B artifact does not match the captured baseline pin")

    if not args.no_build:
        build_worker()
    if not BINARY_PATH.is_file():
        raise BenchmarkError(f"benchmark worker is missing: {BINARY_PATH}")

    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    output_dir = (
        Path(args.output_dir).expanduser().resolve()
        if args.output_dir
        else REPO_ROOT / "benchmark-results" / timestamp
    )
    raw_dir = output_dir / "raw"
    raw_dir.mkdir(parents=True, exist_ok=True)
    runs = []
    sequence = 0
    for profile in profiles:
        for repetition in range(repetitions):
            if sequence == 0:
                load_state = "cold_process_after_artifact_verification"
            elif repetition == 0:
                load_state = "cold_process_shared_filesystem_cache"
            else:
                load_state = "repeat_process_warm_filesystem_cache"
            run_id = f"{timestamp}-{profile['label']}-r{repetition}"
            raw_path = raw_dir / f"{profile['label']}-r{repetition}.json"
            print(
                f"Running {profile['label']} repetition {repetition + 1}/{repetitions}…",
                flush=True,
            )
            runs.append(
                run_worker(
                    model,
                    model_hash,
                    quantization,
                    profile,
                    load_state,
                    repetition,
                    run_id,
                    raw_path,
                    args.quick,
                )
            )
            sequence += 1

    first = next((run for run in runs if run.get("model")), {})
    suite = {
        "schema_version": 1,
        "suite_id": manifest["suite_id"],
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "artifact_verification_ms": verification_ms,
        "artifact_verification_algorithm": "SHA-256",
        "quick": bool(args.quick),
        "requested_repetitions": repetitions,
        "manifest_sha256": hashlib.sha256(MANIFEST_PATH.read_bytes()).hexdigest(),
        "clinical_cases_sha256": hashlib.sha256(CASES_PATH.read_bytes()).hexdigest(),
        "manifest": manifest,
        "build": first.get("build", {}),
        "host": first.get("host", {}),
        "model": first.get(
            "model",
            {
                "filename": model.name,
                "artifact_sha256": model_hash,
                "artifact_size_bytes": model_size,
                "quantization": quantization,
            },
        ),
        "runs": runs,
        "summary": aggregate_suite(runs),
    }

    comparison = None
    if args.baseline:
        baseline = read_json(Path(args.baseline).expanduser().resolve())
        comparison = compare_suites(
            baseline, suite, manifest["regression_thresholds"]
        )
        write_json(output_dir / "comparison.json", comparison)
    write_json(output_dir / "results.json", suite)
    write_csv(output_dir / "results.csv", suite)
    (output_dir / "report.md").write_text(
        render_report(suite, comparison), encoding="utf-8"
    )
    print(f"Results: {output_dir / 'results.json'}")
    print(f"Report:  {output_dir / 'report.md'}")

    hard_failures = [
        run
        for run in runs
        if run.get("status") in {"failed", "quality_failed"}
        or (
            run.get("status") == "completed"
            and (
                run.get("memory", {}).get("swap_delta_bytes") is None
                or int(run["memory"]["swap_delta_bytes"])
                > int(manifest["regression_thresholds"]["max_swap_growth_bytes"])
            )
        )
    ]
    if comparison and comparison["status"] == "regression":
        return 1
    if not any(
        run.get("status") in {"completed", "quality_failed"} for run in runs
    ):
        return 1
    return 1 if hard_failures else 0


def command_compare(args: argparse.Namespace) -> int:
    manifest = read_json(MANIFEST_PATH)
    baseline_path = Path(args.baseline).expanduser().resolve()
    candidate_path = Path(args.candidate).expanduser().resolve()
    baseline = read_json(baseline_path)
    candidate = read_json(candidate_path)
    thresholds = candidate.get("manifest", manifest)["regression_thresholds"]
    comparison = compare_suites(baseline, candidate, thresholds)
    output_dir = (
        Path(args.output_dir).expanduser().resolve()
        if args.output_dir
        else candidate_path.parent / "comparison"
    )
    output_dir.mkdir(parents=True, exist_ok=True)
    write_json(output_dir / "comparison.json", comparison)
    (output_dir / "report.md").write_text(
        render_report(candidate, comparison), encoding="utf-8"
    )
    print(f"Comparison: {output_dir / 'comparison.json'}")
    print(f"Verdict: {comparison['status']}")
    return 1 if comparison["status"] == "regression" else 0


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    commands = root.add_subparsers(dest="command", required=True)

    ci = commands.add_parser("ci", help="run the deterministic no-model subset")
    ci.set_defaults(handler=command_ci)

    run = commands.add_parser("run", help="run the full macOS model benchmark")
    run.add_argument("--model", required=True, help="absolute or relative GGUF path")
    run.add_argument(
        "--quantization", help="artifact quantization (required outside captured Qwen baseline)"
    )
    run.add_argument(
        "--contexts", help="comma-separated profile labels or token counts (default: all)"
    )
    run.add_argument("--repetitions", type=int, help="override manifest repetitions")
    run.add_argument("--baseline", help="prior results.json to compare against")
    run.add_argument("--output-dir", help="artifact directory")
    run.add_argument(
        "--quick", action="store_true", help="run CI cases at 16K once for a hardware smoke test"
    )
    run.add_argument(
        "--no-build", action="store_true", help="reuse an existing release benchmark worker"
    )
    run.set_defaults(handler=command_run)

    compare = commands.add_parser("compare", help="compare two result suites")
    compare.add_argument("baseline", help="baseline results.json")
    compare.add_argument("candidate", help="candidate results.json")
    compare.add_argument("--output-dir", help="comparison artifact directory")
    compare.set_defaults(handler=command_compare)
    return root


def main() -> int:
    args = parser().parse_args()
    try:
        return int(args.handler(args))
    except BenchmarkError as error:
        print(f"benchmark-local-inference: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
