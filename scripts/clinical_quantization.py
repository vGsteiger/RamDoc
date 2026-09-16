#!/usr/bin/env python3
"""Governed clinical calibration and mixed-bit quantization tooling.

This module intentionally uses only the Python standard library.  The expensive
model work remains in llama.cpp; this tool makes the data boundary, bit
allocation, held-out decision, and RamDoc promotion hand-off reproducible.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import statistics
import sys
import unicodedata
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence


SCHEMA_VERSION = 1
STUDY_KIND = "ramdoc-clinical-quantization-study"
SENSITIVITY_KIND = "ramdoc-quantization-sensitivity"
RESULTS_KIND = "ramdoc-held-out-evaluation"
PROMOTION_KIND = "ramdoc-clinical-quantization-promotion"
PROMOTION_EXIT_REJECTED = 3
MAX_FRONTIER_STATES = 50_000
MAX_ARTIFACT_BYTES = 60 * 1024**3
MAX_TENSOR_INVENTORY = 100_000

CALIBRATION_CATEGORIES = (
    "medication",
    "dose",
    "date",
    "negation",
    "uncertainty",
    "german_swiss",
    "report_generation",
    "tool_call",
)

EVALUATION_CATEGORIES = (
    "medication",
    "dose",
    "date",
    "negation",
    "uncertainty",
    "chronology",
    "unsupported_claim",
    "german_swiss",
    "report_generation",
    "tool_call",
    "general_instruction",
    "long_context",
)

SENSITIVITY_DOMAINS = (
    "attention",
    "mlp",
    "embedding_output",
    "clinical_sensitive",
)

_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
_COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
_ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$")
_QUANT_TYPE_RE = re.compile(r"^[A-Za-z0-9_]+$")


class QuantizationError(ValueError):
    """A user-actionable contract or gate failure."""


@dataclass(frozen=True)
class ValidatedStudy:
    path: Path
    document: dict[str, Any]
    manifest_sha256: str
    calibration: tuple[dict[str, Any], ...]
    evaluation: tuple[dict[str, Any], ...]
    split_sha256: Mapping[str, str]


@dataclass(frozen=True)
class AllocationState:
    size_bytes: int
    regressions: tuple[float, ...]
    assignments: tuple[tuple[str, str, str], ...]


def _mapping(value: Any, context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise QuantizationError(f"{context} must be a JSON object")
    return value


def _sequence(value: Any, context: str) -> list[Any]:
    if not isinstance(value, list):
        raise QuantizationError(f"{context} must be a JSON array")
    return value


def _required_string(obj: Mapping[str, Any], key: str, context: str) -> str:
    value = obj.get(key)
    if not isinstance(value, str) or not value.strip():
        raise QuantizationError(f"{context}.{key} must be a non-empty string")
    if value != value.strip():
        raise QuantizationError(
            f"{context}.{key} must not contain leading or trailing whitespace"
        )
    return value


def _required_bool(obj: Mapping[str, Any], key: str, context: str) -> bool:
    value = obj.get(key)
    if not isinstance(value, bool):
        raise QuantizationError(f"{context}.{key} must be a boolean")
    return value


def _required_int(obj: Mapping[str, Any], key: str, context: str) -> int:
    value = obj.get(key)
    if isinstance(value, bool) or not isinstance(value, int):
        raise QuantizationError(f"{context}.{key} must be an integer")
    return value


def _required_number(obj: Mapping[str, Any], key: str, context: str) -> float:
    value = obj.get(key)
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise QuantizationError(f"{context}.{key} must be a number")
    value = float(value)
    if not math.isfinite(value):
        raise QuantizationError(f"{context}.{key} must be finite")
    return value


def _validate_id(value: str, context: str) -> None:
    if not _ID_RE.fullmatch(value):
        raise QuantizationError(
            f"{context} must match {_ID_RE.pattern!r}; got {value!r}"
        )


def _validate_sha256(value: str, context: str) -> None:
    if not _SHA256_RE.fullmatch(value):
        raise QuantizationError(f"{context} must be a lowercase SHA-256 digest")


def _load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise QuantizationError(f"cannot read {path}: {exc}") from exc
    except json.JSONDecodeError as exc:
        raise QuantizationError(f"invalid JSON in {path}: {exc}") from exc
    return _mapping(value, str(path))


def _read_jsonl(path: Path) -> tuple[dict[str, Any], ...]:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        raise QuantizationError(f"cannot read {path}: {exc}") from exc

    records: list[dict[str, Any]] = []
    for line_number, line in enumerate(lines, start=1):
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as exc:
            raise QuantizationError(
                f"invalid JSON in {path}:{line_number}: {exc.msg}"
            ) from exc
        records.append(_mapping(value, f"{path}:{line_number}"))
    return tuple(records)


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as exc:
        raise QuantizationError(f"cannot hash {path}: {exc}") from exc
    return digest.hexdigest()


def _atomic_write(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp")
    try:
        temporary.write_bytes(data)
        temporary.replace(path)
    except OSError as exc:
        raise QuantizationError(f"cannot write {path}: {exc}") from exc


def _write_json(path: Path, value: Mapping[str, Any]) -> None:
    encoded = (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")
    _atomic_write(path, encoded)


def _normalised_text(text: str) -> str:
    return " ".join(unicodedata.normalize("NFKC", text).casefold().split())


def _record_fingerprint(record: Mapping[str, Any]) -> str:
    text = _required_string(record, "text", "corpus record")
    return hashlib.sha256(_normalised_text(text).encode("utf-8")).hexdigest()


def _validate_provenance(
    split: Mapping[str, Any],
    context: str,
    contains_real_clinical_text: bool,
) -> set[str]:
    sources = _sequence(split.get("sources"), f"{context}.sources")
    if not sources:
        raise QuantizationError(f"{context}.sources must not be empty")

    source_ids: set[str] = set()
    for index, raw_source in enumerate(sources):
        source_context = f"{context}.sources[{index}]"
        source = _mapping(raw_source, source_context)
        source_id = _required_string(source, "id", source_context)
        _validate_id(source_id, f"{source_context}.id")
        if source_id in source_ids:
            raise QuantizationError(f"duplicate source id {source_id!r} in {context}")
        source_ids.add(source_id)
        _required_string(source, "origin", source_context)
        _required_string(source, "license", source_context)
        _required_string(source, "consent_basis", source_context)
        _required_string(source, "preprocessing", source_context)
        synthetic = _required_bool(source, "synthetic", source_context)
        if not contains_real_clinical_text and not synthetic:
            raise QuantizationError(
                f"{source_context} is marked non-synthetic while governance declares "
                "that no real clinical text is present"
            )
    return source_ids


def _validate_split(
    manifest_path: Path,
    split_name: str,
    raw_split: Any,
    contains_real_clinical_text: bool,
) -> tuple[tuple[dict[str, Any], ...], str]:
    context = f"data.{split_name}"
    split = _mapping(raw_split, context)
    relative_path = _required_string(split, "path", context)
    split_path = Path(relative_path)
    if not split_path.is_absolute():
        split_path = manifest_path.parent / split_path
    split_path = split_path.resolve()

    expected_sha = _required_string(split, "sha256", context)
    _validate_sha256(expected_sha, f"{context}.sha256")
    actual_sha = _sha256_file(split_path)
    if actual_sha != expected_sha:
        raise QuantizationError(
            f"{context} hash mismatch: manifest={expected_sha}, actual={actual_sha}"
        )

    source_ids = _validate_provenance(split, context, contains_real_clinical_text)
    records = _read_jsonl(split_path)
    declared_count = _required_int(split, "record_count", context)
    if declared_count != len(records):
        raise QuantizationError(
            f"{context}.record_count is {declared_count}, but {split_path} has "
            f"{len(records)} records"
        )
    if not records:
        raise QuantizationError(f"{context} must contain at least one record")

    seen_cases: set[str] = set()
    categories: set[str] = set()
    for index, record in enumerate(records):
        record_context = f"{context}[{index}]"
        case_id = _required_string(record, "case_id", record_context)
        family_id = _required_string(record, "family_id", record_context)
        category = _required_string(record, "category", record_context)
        source_id = _required_string(record, "source_id", record_context)
        _required_string(record, "text", record_context)
        _validate_id(case_id, f"{record_context}.case_id")
        _validate_id(family_id, f"{record_context}.family_id")
        if case_id in seen_cases:
            raise QuantizationError(f"duplicate case_id {case_id!r} in {context}")
        if source_id not in source_ids:
            raise QuantizationError(
                f"{record_context}.source_id {source_id!r} is not declared in sources"
            )
        seen_cases.add(case_id)
        categories.add(category)
        if split_name == "evaluation":
            _mapping(record.get("expected"), f"{record_context}.expected")

    required = set(
        CALIBRATION_CATEGORIES
        if split_name == "calibration"
        else EVALUATION_CATEGORIES
    )
    missing = sorted(required - categories)
    if missing:
        raise QuantizationError(
            f"{context} is missing required categories: {', '.join(missing)}"
        )
    return records, actual_sha


def _validate_acceptance(document: Mapping[str, Any]) -> None:
    acceptance = _mapping(document.get("acceptance"), "acceptance")
    min_cases = _required_int(acceptance, "min_cases_per_category", "acceptance")
    if min_cases < 1:
        raise QuantizationError("acceptance.min_cases_per_category must be positive")
    confidence = _required_number(acceptance, "confidence_level", "acceptance")
    if not 0.5 < confidence < 1.0:
        raise QuantizationError("acceptance.confidence_level must be between 0.5 and 1")
    max_artifact = _required_int(acceptance, "max_artifact_bytes", "acceptance")
    max_peak_ram = _required_int(acceptance, "max_peak_ram_bytes", "acceptance")
    if not 0 < max_artifact <= MAX_ARTIFACT_BYTES:
        raise QuantizationError(
            f"acceptance.max_artifact_bytes must be in 1..{MAX_ARTIFACT_BYTES}"
        )
    if max_peak_ram <= 0:
        raise QuantizationError("acceptance.max_peak_ram_bytes must be positive")

    category_limits = _mapping(
        acceptance.get("category_regression_limits"),
        "acceptance.category_regression_limits",
    )
    for category in EVALUATION_CATEGORIES:
        limit = _required_number(
            category_limits,
            category,
            "acceptance.category_regression_limits",
        )
        if not 0 <= limit <= 1:
            raise QuantizationError(
                f"category regression limit for {category} must be in [0, 1]"
            )

    resource_limits = _mapping(
        acceptance.get("resource_regression_limits"),
        "acceptance.resource_regression_limits",
    )
    for metric in (
        "artifact_size_bytes",
        "peak_ram_bytes",
        "ttft_ms",
        "prompt_tokens_per_second",
        "decode_tokens_per_second",
    ):
        limit = _required_number(
            resource_limits,
            metric,
            "acceptance.resource_regression_limits",
        )
        if not 0 <= limit < 1:
            raise QuantizationError(
                f"resource regression limit for {metric} must be in [0, 1)"
            )
    improvement = _required_number(
        acceptance, "minimum_improvement_ratio", "acceptance"
    )
    if not 0 < improvement < 1:
        raise QuantizationError(
            "acceptance.minimum_improvement_ratio must be between 0 and 1"
        )
    required_baselines = _sequence(
        acceptance.get("required_baseline_artifacts"),
        "acceptance.required_baseline_artifacts",
    )
    if not required_baselines:
        raise QuantizationError(
            "acceptance.required_baseline_artifacts must not be empty"
        )
    seen_baselines: set[str] = set()
    for index, baseline in enumerate(required_baselines):
        if not isinstance(baseline, str) or not baseline.strip():
            raise QuantizationError(
                f"acceptance.required_baseline_artifacts[{index}] must be a string"
            )
        _validate_id(
            baseline,
            f"acceptance.required_baseline_artifacts[{index}]",
        )
        if baseline in seen_baselines:
            raise QuantizationError(
                f"duplicate required baseline artifact {baseline!r}"
            )
        seen_baselines.add(baseline)


def validate_manifest(path: Path) -> ValidatedStudy:
    """Validate governance, provenance, hashes, coverage, and split isolation."""

    path = path.resolve()
    document = _load_json(path)
    if document.get("schema_version") != SCHEMA_VERSION:
        raise QuantizationError(
            f"schema_version must be {SCHEMA_VERSION}; got {document.get('schema_version')!r}"
        )
    if document.get("kind") != STUDY_KIND:
        raise QuantizationError(f"kind must be {STUDY_KIND!r}")
    study_id = _required_string(document, "study_id", "study")
    _validate_id(study_id, "study.study_id")
    stage = _required_string(document, "study_stage", "study")
    if stage not in {"development", "promotion"}:
        raise QuantizationError("study.study_stage must be development or promotion")

    governance = _mapping(document.get("governance"), "governance")
    contains_real = _required_bool(
        governance, "contains_real_clinical_text", "governance"
    )
    review_status = _required_string(governance, "review_status", "governance")
    if contains_real:
        if review_status != "approved":
            raise QuantizationError(
                "real clinical text is blocked until governance.review_status is approved"
            )
        for key in ("review_id", "reviewer", "reviewed_at", "consent_basis"):
            _required_string(governance, key, "governance")
        deidentification = _mapping(
            governance.get("deidentification"), "governance.deidentification"
        )
        _required_string(deidentification, "method", "governance.deidentification")
        _required_string(deidentification, "verified_by", "governance.deidentification")
    elif review_status != "synthetic_only":
        raise QuantizationError(
            "a study without real clinical text must use review_status=synthetic_only"
        )

    model = _mapping(document.get("base_model"), "base_model")
    _required_string(model, "filename", "base_model")
    _required_string(model, "source", "base_model")
    _required_string(model, "license", "base_model")
    model_sha = _required_string(model, "sha256", "base_model")
    _validate_sha256(model_sha, "base_model.sha256")
    if stage == "promotion" and model_sha == "0" * 64:
        raise QuantizationError(
            "a promotion study cannot use the all-zero base_model.sha256 placeholder"
        )

    toolchain = _mapping(document.get("toolchain"), "toolchain")
    commit = _required_string(toolchain, "llama_cpp_commit", "toolchain")
    if not _COMMIT_RE.fullmatch(commit):
        raise QuantizationError("toolchain.llama_cpp_commit must be a lowercase 40-char SHA")
    commands = _mapping(toolchain.get("commands"), "toolchain.commands")
    command_values: list[str] = []
    for key in ("imatrix", "quantize", "evaluate"):
        command_values.append(_required_string(commands, key, "toolchain.commands"))
    if stage == "promotion" and any(
        "..." in command or "placeholder" in command.casefold()
        for command in command_values
    ):
        raise QuantizationError(
            "a promotion study must record complete commands without ellipses or placeholders"
        )

    _validate_acceptance(document)
    data = _mapping(document.get("data"), "data")
    calibration, calibration_sha = _validate_split(
        path, "calibration", data.get("calibration"), contains_real
    )
    evaluation, evaluation_sha = _validate_split(
        path, "evaluation", data.get("evaluation"), contains_real
    )

    calibration_case_ids = {str(record["case_id"]) for record in calibration}
    evaluation_case_ids = {str(record["case_id"]) for record in evaluation}
    overlapping_cases = sorted(calibration_case_ids & evaluation_case_ids)
    if overlapping_cases:
        raise QuantizationError(
            "calibration/evaluation case_id overlap: " + ", ".join(overlapping_cases)
        )

    calibration_families = {str(record["family_id"]) for record in calibration}
    evaluation_families = {str(record["family_id"]) for record in evaluation}
    overlapping_families = sorted(calibration_families & evaluation_families)
    if overlapping_families:
        raise QuantizationError(
            "calibration/evaluation family_id overlap (paraphrase leakage): "
            + ", ".join(overlapping_families)
        )

    calibration_fingerprints = {_record_fingerprint(record) for record in calibration}
    evaluation_fingerprints = {_record_fingerprint(record) for record in evaluation}
    if calibration_fingerprints & evaluation_fingerprints:
        raise QuantizationError(
            "calibration/evaluation contain identical normalised text"
        )

    return ValidatedStudy(
        path=path,
        document=document,
        manifest_sha256=_sha256_file(path),
        calibration=calibration,
        evaluation=evaluation,
        split_sha256={
            "calibration": calibration_sha,
            "evaluation": evaluation_sha,
        },
    )


def study_lock(study: ValidatedStudy) -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "kind": "ramdoc-clinical-quantization-study-lock",
        "study_id": study.document["study_id"],
        "manifest_sha256": study.manifest_sha256,
        "splits": {
            "calibration": {
                "sha256": study.split_sha256["calibration"],
                "record_count": len(study.calibration),
            },
            "evaluation": {
                "sha256": study.split_sha256["evaluation"],
                "record_count": len(study.evaluation),
            },
        },
        "llama_cpp_commit": study.document["toolchain"]["llama_cpp_commit"],
    }


def export_calibration(study: ValidatedStudy) -> bytes:
    """Materialise only the governed calibration split for llama-imatrix."""

    texts = [str(record["text"]).strip() for record in study.calibration]
    return ("\n\n###\n\n".join(texts) + "\n").encode("utf-8")


def _dominates(left: AllocationState, right: AllocationState) -> bool:
    no_larger = left.size_bytes <= right.size_bytes
    no_worse = all(a <= b + 1e-12 for a, b in zip(left.regressions, right.regressions))
    strictly_better = left.size_bytes < right.size_bytes or any(
        a < b - 1e-12 for a, b in zip(left.regressions, right.regressions)
    )
    return no_larger and no_worse and strictly_better


def _prune_frontier(states: Iterable[AllocationState]) -> list[AllocationState]:
    ordered = sorted(
        states,
        key=lambda state: (state.size_bytes, state.regressions, state.assignments),
    )
    frontier: list[AllocationState] = []
    for state in ordered:
        if any(_dominates(other, state) for other in frontier):
            continue
        frontier = [other for other in frontier if not _dominates(state, other)]
        if frontier and (
            frontier[-1].size_bytes == state.size_bytes
            and frontier[-1].regressions == state.regressions
        ):
            continue
        frontier.append(state)
    return frontier


def allocate_recipe(
    sensitivity_path: Path,
    study: ValidatedStudy,
) -> tuple[str, dict[str, Any]]:
    """Search the exact category-wise Pareto frontier and choose a minimax recipe."""

    sensitivity_path = sensitivity_path.resolve()
    document = _load_json(sensitivity_path)
    if document.get("schema_version") != SCHEMA_VERSION:
        raise QuantizationError(f"sensitivity.schema_version must be {SCHEMA_VERSION}")
    if document.get("kind") != SENSITIVITY_KIND:
        raise QuantizationError(f"sensitivity.kind must be {SENSITIVITY_KIND!r}")
    if document.get("study_manifest_sha256") != study.manifest_sha256:
        raise QuantizationError("sensitivity is not bound to this study manifest hash")
    evidence_status = _required_string(document, "evidence_status", "sensitivity")
    if evidence_status not in {"measured", "synthetic_smoke_test"}:
        raise QuantizationError(
            "sensitivity.evidence_status must be measured or synthetic_smoke_test"
        )
    default_type = _required_string(document, "default_type", "sensitivity")
    if not _QUANT_TYPE_RE.fullmatch(default_type):
        raise QuantizationError("sensitivity.default_type is not a valid quant type")

    categories = _sequence(document.get("categories"), "sensitivity.categories")
    if categories != list(EVALUATION_CATEGORIES):
        raise QuantizationError(
            "sensitivity.categories must exactly match the ordered RamDoc evaluation categories"
        )

    raw_inventory = _sequence(
        document.get("tensor_inventory"), "sensitivity.tensor_inventory"
    )
    if not raw_inventory or len(raw_inventory) > MAX_TENSOR_INVENTORY:
        raise QuantizationError(
            f"sensitivity.tensor_inventory must contain 1..{MAX_TENSOR_INVENTORY} names"
        )
    tensor_inventory: list[str] = []
    seen_tensors: set[str] = set()
    for tensor_index, raw_tensor in enumerate(raw_inventory):
        context = f"sensitivity.tensor_inventory[{tensor_index}]"
        if (
            not isinstance(raw_tensor, str)
            or not raw_tensor
            or len(raw_tensor) > 512
            or raw_tensor != raw_tensor.lower()
            or any(
                character.isspace() or unicodedata.category(character) == "Cc"
                for character in raw_tensor
            )
        ):
            raise QuantizationError(
                f"{context} must be a lowercase tensor name without whitespace or control characters"
            )
        if raw_tensor in seen_tensors:
            raise QuantizationError(f"duplicate tensor inventory name {raw_tensor!r}")
        seen_tensors.add(raw_tensor)
        tensor_inventory.append(raw_tensor)

    fixed_bytes = _required_int(document, "fixed_bytes", "sensitivity")
    acceptance = study.document["acceptance"]
    budget = int(acceptance["max_artifact_bytes"])
    if fixed_bytes < 0 or fixed_bytes > budget:
        raise QuantizationError("sensitivity.fixed_bytes is outside the artifact budget")
    limits_map = acceptance["category_regression_limits"]
    limits = tuple(float(limits_map[category]) for category in categories)

    raw_groups = _sequence(document.get("groups"), "sensitivity.groups")
    if not raw_groups:
        raise QuantizationError("sensitivity.groups must not be empty")
    groups: list[
        tuple[str, str, tuple[str, ...], list[tuple[str, int, tuple[float, ...]]]]
    ] = []
    seen_names: set[str] = set()
    seen_selectors: set[str] = set()
    covered_domains: set[str] = set()
    claimed_tensors: dict[str, str] = {}
    group_matches: dict[str, list[str]] = {}
    for group_index, raw_group in enumerate(raw_groups):
        context = f"sensitivity.groups[{group_index}]"
        group = _mapping(raw_group, context)
        name = _required_string(group, "name", context)
        selector = _required_string(group, "selector", context)
        _validate_id(name, f"{context}.name")
        if name in seen_names:
            raise QuantizationError(f"duplicate sensitivity group name {name!r}")
        if selector in seen_selectors:
            raise QuantizationError(f"duplicate tensor selector {selector!r}")
        if (
            "=" in selector
            or selector != selector.lower()
            or any(
                character.isspace() or unicodedata.category(character) == "Cc"
                for character in selector
            )
        ):
            raise QuantizationError(
                f"{context}.selector must be lowercase and cannot contain '=', whitespace, or control characters"
            )
        try:
            selector_pattern = re.compile(selector)
        except re.error as exc:
            raise QuantizationError(
                f"{context}.selector is not a valid tensor-name regex: {exc}"
            ) from exc
        matched_tensors = [
            tensor_name
            for tensor_name in tensor_inventory
            if selector_pattern.search(tensor_name)
        ]
        if not matched_tensors:
            raise QuantizationError(
                f"{context}.selector matches no tensor in sensitivity.tensor_inventory"
            )
        overlap = sorted(
            tensor_name for tensor_name in matched_tensors if tensor_name in claimed_tensors
        )
        if overlap:
            previous = claimed_tensors[overlap[0]]
            raise QuantizationError(
                f"{context}.selector overlaps group {previous!r} on tensor {overlap[0]!r}"
            )
        for tensor_name in matched_tensors:
            claimed_tensors[tensor_name] = name
        group_matches[name] = matched_tensors
        seen_names.add(name)
        seen_selectors.add(selector)

        raw_domains = _sequence(group.get("domains"), f"{context}.domains")
        domains: list[str] = []
        for domain_index, raw_domain in enumerate(raw_domains):
            if not isinstance(raw_domain, str) or raw_domain not in SENSITIVITY_DOMAINS:
                raise QuantizationError(
                    f"{context}.domains[{domain_index}] must be one of "
                    + ", ".join(SENSITIVITY_DOMAINS)
                )
            if raw_domain in domains:
                raise QuantizationError(
                    f"duplicate sensitivity domain {raw_domain!r} in group {name!r}"
                )
            domains.append(raw_domain)
        if not domains:
            raise QuantizationError(f"{context}.domains must not be empty")
        covered_domains.update(domains)

        options: list[tuple[str, int, tuple[float, ...]]] = []
        seen_types: set[str] = set()
        for option_index, raw_option in enumerate(
            _sequence(group.get("options"), f"{context}.options")
        ):
            option_context = f"{context}.options[{option_index}]"
            option = _mapping(raw_option, option_context)
            quant_type = _required_string(option, "type", option_context)
            if not _QUANT_TYPE_RE.fullmatch(quant_type):
                raise QuantizationError(f"{option_context}.type is invalid")
            if quant_type in seen_types:
                raise QuantizationError(
                    f"duplicate type {quant_type!r} in sensitivity group {name!r}"
                )
            seen_types.add(quant_type)
            option_bytes = _required_int(option, "estimated_bytes", option_context)
            if option_bytes <= 0:
                raise QuantizationError(f"{option_context}.estimated_bytes must be positive")
            regression_map = _mapping(
                option.get("regression_upper_95"),
                f"{option_context}.regression_upper_95",
            )
            regressions = tuple(
                _required_number(
                    regression_map,
                    category,
                    f"{option_context}.regression_upper_95",
                )
                for category in categories
            )
            if any(value < 0 or value > 1 for value in regressions):
                raise QuantizationError(
                    f"{option_context}.regression_upper_95 values must be in [0, 1]"
                )
            options.append((quant_type, option_bytes, regressions))
        if not options:
            raise QuantizationError(f"{context}.options must not be empty")
        groups.append((name, selector, tuple(domains), options))

    missing_domains = sorted(set(SENSITIVITY_DOMAINS) - covered_domains)
    if missing_domains:
        raise QuantizationError(
            "sensitivity groups are missing required tensor domains: "
            + ", ".join(missing_domains)
        )

    frontier = [AllocationState(fixed_bytes, (0.0,) * len(categories), ())]
    frontier_sizes: list[int] = []
    for name, selector, _domains, options in groups:
        candidates: list[AllocationState] = []
        for state in frontier:
            for quant_type, option_bytes, regressions in options:
                size = state.size_bytes + option_bytes
                if size > budget:
                    continue
                candidates.append(
                    AllocationState(
                        size_bytes=size,
                        regressions=tuple(
                            current + added
                            for current, added in zip(state.regressions, regressions)
                        ),
                        assignments=state.assignments + ((name, selector, quant_type),),
                    )
                )
        if not candidates:
            raise QuantizationError(
                f"no recipe fits the artifact budget after group {name!r}"
            )
        if len(candidates) > MAX_FRONTIER_STATES:
            raise QuantizationError(
                f"frontier expanded to {len(candidates)} states at group {name!r}; "
                "combine equivalent tensor groups or raise the implementation limit"
            )
        frontier = _prune_frontier(candidates)
        frontier_sizes.append(len(frontier))

    feasible = [
        state
        for state in frontier
        if all(value <= limit + 1e-12 for value, limit in zip(state.regressions, limits))
    ]
    if not feasible:
        best = min(
            frontier,
            key=lambda state: max(
                (
                    value / limit
                    if limit > 0
                    else (0.0 if value <= 0 else math.inf)
                )
                for value, limit in zip(state.regressions, limits)
            ),
        )
        observed = dict(zip(categories, best.regressions))
        raise QuantizationError(
            "no mixed-bit recipe satisfies every category regression limit; "
            f"best predicted upper regressions={observed}"
        )

    def objective(state: AllocationState) -> tuple[Any, ...]:
        normalised = tuple(
            value / limit if limit > 0 else (0.0 if value <= 0 else math.inf)
            for value, limit in zip(state.regressions, limits)
        )
        return (
            max(normalised),
            sum(max(0.0, value) for value in normalised),
            state.size_bytes,
            state.assignments,
        )

    selected = min(feasible, key=objective)
    recipe = "".join(
        f"{selector}={quant_type}\n"
        for _name, selector, quant_type in selected.assignments
    )
    recipe_sha = hashlib.sha256(recipe.encode("utf-8")).hexdigest()
    report: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "kind": "ramdoc-mixed-bit-allocation",
        "study_manifest_sha256": study.manifest_sha256,
        "sensitivity_sha256": _sha256_file(sensitivity_path),
        "evidence_status": evidence_status,
        "algorithm": "category-pareto-minimax-v1",
        "default_type": default_type,
        "budget_bytes": budget,
        "fixed_bytes": fixed_bytes,
        "tensor_inventory_sha256": hashlib.sha256(
            ("\n".join(tensor_inventory) + "\n").encode("utf-8")
        ).hexdigest(),
        "tensor_inventory_count": len(tensor_inventory),
        "matched_tensors_by_group": group_matches,
        "unmatched_tensor_count": len(tensor_inventory) - len(claimed_tensors),
        "estimated_artifact_bytes": selected.size_bytes,
        "recipe_sha256": recipe_sha,
        "predicted_regression_upper_95": dict(zip(categories, selected.regressions)),
        "category_regression_limits": dict(zip(categories, limits)),
        "tensor_domain_coverage": {
            domain: [
                selector
                for _name, selector, domains, _options in groups
                if domain in domains
            ]
            for domain in SENSITIVITY_DOMAINS
        },
        "assignments": [
            {"group": name, "selector": selector, "type": quant_type}
            for name, selector, quant_type in selected.assignments
        ],
        "frontier_states_after_each_group": frontier_sizes,
        "final_frontier_states": len(frontier),
        "feasible_frontier_states": len(feasible),
    }
    return recipe, report


def _validate_artifact_result(
    raw_artifact: Any,
    context: str,
    expected_cases: Mapping[str, str],
) -> dict[str, Any]:
    artifact = _mapping(raw_artifact, context)
    artifact_id = _required_string(artifact, "id", context)
    _validate_id(artifact_id, f"{context}.id")
    role = _required_string(artifact, "role", context)
    if role not in {"standard", "candidate"}:
        raise QuantizationError(f"{context}.role must be standard or candidate")
    _required_string(artifact, "display_name", context)
    _required_string(artifact, "quantization", context)
    filename = _required_string(artifact, "filename", context)
    if (
        not filename.endswith(".gguf")
        or Path(filename).name != filename
        or "/" in filename
        or "\\" in filename
        or ".." in filename
    ):
        raise QuantizationError(f"{context}.filename must be a plain .gguf filename")
    artifact_sha = _required_string(artifact, "sha256", context)
    base_sha = _required_string(artifact, "base_model_sha256", context)
    _validate_sha256(artifact_sha, f"{context}.sha256")
    _validate_sha256(base_sha, f"{context}.base_model_sha256")
    if artifact_sha == "0" * 64 or base_sha == "0" * 64:
        raise QuantizationError(f"{context} cannot use an all-zero SHA-256 placeholder")
    if role == "candidate":
        recipe_sha = _required_string(artifact, "recipe_sha256", context)
        _validate_sha256(recipe_sha, f"{context}.recipe_sha256")
        if recipe_sha == "0" * 64:
            raise QuantizationError(
                f"{context}.recipe_sha256 cannot be an all-zero placeholder"
            )
    size = _required_int(artifact, "artifact_size_bytes", context)
    if not 0 < size <= MAX_ARTIFACT_BYTES:
        raise QuantizationError(f"{context}.artifact_size_bytes is outside the safe range")
    for metric in (
        "peak_ram_bytes",
        "ttft_ms",
        "prompt_tokens_per_second",
        "decode_tokens_per_second",
    ):
        if _required_number(artifact, metric, context) <= 0:
            raise QuantizationError(f"{context}.{metric} must be positive")

    raw_cases = _sequence(artifact.get("cases"), f"{context}.cases")
    seen_cases: set[str] = set()
    for case_index, raw_case in enumerate(raw_cases):
        case_context = f"{context}.cases[{case_index}]"
        case = _mapping(raw_case, case_context)
        case_id = _required_string(case, "case_id", case_context)
        category = _required_string(case, "category", case_context)
        score = _required_number(case, "score", case_context)
        if case_id in seen_cases:
            raise QuantizationError(f"duplicate case_id {case_id!r} in {context}")
        if case_id not in expected_cases:
            raise QuantizationError(f"unknown held-out case_id {case_id!r} in {context}")
        if category != expected_cases[case_id]:
            raise QuantizationError(
                f"held-out category changed for {case_id!r}: expected "
                f"{expected_cases[case_id]!r}, got {category!r}"
            )
        if not 0 <= score <= 1:
            raise QuantizationError(f"{case_context}.score must be in [0, 1]")
        seen_cases.add(case_id)
    missing = sorted(set(expected_cases) - seen_cases)
    if missing:
        raise QuantizationError(
            f"{context} is missing held-out cases: {', '.join(missing[:10])}"
        )
    return artifact


def validate_results(path: Path, study: ValidatedStudy) -> dict[str, Any]:
    path = path.resolve()
    document = _load_json(path)
    if document.get("schema_version") != SCHEMA_VERSION:
        raise QuantizationError(f"results.schema_version must be {SCHEMA_VERSION}")
    if document.get("kind") != RESULTS_KIND:
        raise QuantizationError(f"results.kind must be {RESULTS_KIND!r}")
    if document.get("study_manifest_sha256") != study.manifest_sha256:
        raise QuantizationError("held-out results are not bound to this study manifest")
    if document.get("evidence_status") != "measured":
        raise QuantizationError("only measured held-out results can enter the promotion gate")
    if document.get("llama_cpp_commit") != study.document["toolchain"][
        "llama_cpp_commit"
    ]:
        raise QuantizationError("results llama.cpp commit differs from the study pin")
    _required_string(document, "completed_at", "results")

    expected_cases = {
        str(record["case_id"]): str(record["category"])
        for record in study.evaluation
    }
    artifacts = _sequence(document.get("artifacts"), "results.artifacts")
    if not artifacts:
        raise QuantizationError("results.artifacts must not be empty")
    validated: list[dict[str, Any]] = []
    seen_ids: set[str] = set()
    for index, artifact in enumerate(artifacts):
        validated_artifact = _validate_artifact_result(
            artifact,
            f"results.artifacts[{index}]",
            expected_cases,
        )
        artifact_id = str(validated_artifact["id"])
        if artifact_id in seen_ids:
            raise QuantizationError(f"duplicate artifact id {artifact_id!r}")
        if validated_artifact["base_model_sha256"] != study.document["base_model"][
            "sha256"
        ]:
            raise QuantizationError(
                f"artifact {artifact_id!r} was not built from the study's pinned base model"
            )
        seen_ids.add(artifact_id)
        validated.append(validated_artifact)
    if not any(artifact["role"] == "standard" for artifact in validated):
        raise QuantizationError("results must contain at least one standard artifact")
    standard_ids = {
        str(artifact["id"])
        for artifact in validated
        if artifact["role"] == "standard"
    }
    required_baselines = set(
        study.document["acceptance"]["required_baseline_artifacts"]
    )
    missing_baselines = sorted(required_baselines - standard_ids)
    if missing_baselines:
        raise QuantizationError(
            "results are missing required standard artifacts: "
            + ", ".join(missing_baselines)
        )
    document["artifacts"] = validated
    return document


def _case_scores(artifact: Mapping[str, Any]) -> dict[str, tuple[str, float]]:
    return {
        str(case["case_id"]): (str(case["category"]), float(case["score"]))
        for case in artifact["cases"]
    }


def _paired_category_comparison(
    candidate: Mapping[str, Any],
    baseline: Mapping[str, Any],
    category: str,
    confidence_level: float,
    regression_limit: float,
) -> dict[str, Any]:
    candidate_scores = _case_scores(candidate)
    baseline_scores = _case_scores(baseline)
    case_ids = sorted(
        case_id
        for case_id, (case_category, _score) in candidate_scores.items()
        if case_category == category
    )
    deltas = [
        candidate_scores[case_id][1] - baseline_scores[case_id][1]
        for case_id in case_ids
    ]
    if not deltas:
        raise QuantizationError(f"no held-out cases for required category {category}")
    mean_delta = statistics.fmean(deltas)
    standard_error = (
        statistics.stdev(deltas) / math.sqrt(len(deltas)) if len(deltas) > 1 else 0.0
    )
    z_score = statistics.NormalDist().inv_cdf(confidence_level)
    delta_lower = max(-1.0, mean_delta - z_score * standard_error)
    regression_upper = max(0.0, -delta_lower)
    candidate_mean = statistics.fmean(candidate_scores[case_id][1] for case_id in case_ids)
    baseline_mean = statistics.fmean(baseline_scores[case_id][1] for case_id in case_ids)
    return {
        "cases": len(case_ids),
        "candidate_score": candidate_mean,
        "baseline_score": baseline_mean,
        "mean_delta": mean_delta,
        "delta_lower_confidence": delta_lower,
        "regression_upper_confidence": regression_upper,
        "regression_limit": regression_limit,
        "noninferior": regression_upper <= regression_limit + 1e-12,
        "confident_improvement": delta_lower > 0,
    }


def _resource_comparison(
    candidate: Mapping[str, Any],
    baseline: Mapping[str, Any],
    limits: Mapping[str, Any],
    minimum_improvement: float,
) -> dict[str, Any]:
    result: dict[str, Any] = {}
    lower_is_better = {"artifact_size_bytes", "peak_ram_bytes", "ttft_ms"}
    for metric in (
        "artifact_size_bytes",
        "peak_ram_bytes",
        "ttft_ms",
        "prompt_tokens_per_second",
        "decode_tokens_per_second",
    ):
        candidate_value = float(candidate[metric])
        baseline_value = float(baseline[metric])
        limit = float(limits[metric])
        if metric in lower_is_better:
            noninferior = candidate_value <= baseline_value * (1 + limit)
            improved = candidate_value <= baseline_value * (1 - minimum_improvement)
            relative_change = candidate_value / baseline_value - 1
        else:
            noninferior = candidate_value >= baseline_value * (1 - limit)
            improved = candidate_value >= baseline_value * (1 + minimum_improvement)
            relative_change = candidate_value / baseline_value - 1
        result[metric] = {
            "candidate": candidate_value,
            "baseline": baseline_value,
            "relative_change": relative_change,
            "regression_limit": limit,
            "noninferior": noninferior,
            "improved": improved,
        }
    return result


def _compare_pair(
    candidate: Mapping[str, Any],
    baseline: Mapping[str, Any],
    study: ValidatedStudy,
) -> dict[str, Any]:
    acceptance = study.document["acceptance"]
    category_limits = acceptance["category_regression_limits"]
    category_results = {
        category: _paired_category_comparison(
            candidate,
            baseline,
            category,
            float(acceptance["confidence_level"]),
            float(category_limits[category]),
        )
        for category in EVALUATION_CATEGORIES
    }
    resources = _resource_comparison(
        candidate,
        baseline,
        acceptance["resource_regression_limits"],
        float(acceptance["minimum_improvement_ratio"]),
    )
    category_noninferior = all(
        result["noninferior"] for result in category_results.values()
    )
    resource_noninferior = all(result["noninferior"] for result in resources.values())
    improved = any(
        result["confident_improvement"] for result in category_results.values()
    ) or any(result["improved"] for result in resources.values())
    return {
        "baseline_id": baseline["id"],
        "categories": category_results,
        "resources": resources,
        "category_noninferior": category_noninferior,
        "resource_noninferior": resource_noninferior,
        "strict_improvement": improved,
        "dominates": category_noninferior and resource_noninferior and improved,
    }


def gate_candidate(
    results: Mapping[str, Any],
    candidate_id: str,
    study: ValidatedStudy,
) -> dict[str, Any]:
    """Apply per-category non-inferiority and fixed-memory Pareto gates."""

    artifacts = list(results["artifacts"])
    candidate = next(
        (artifact for artifact in artifacts if artifact["id"] == candidate_id), None
    )
    if candidate is None:
        raise QuantizationError(f"candidate artifact {candidate_id!r} is absent")
    if candidate["role"] != "candidate":
        raise QuantizationError(f"artifact {candidate_id!r} is not marked candidate")
    standards = [artifact for artifact in artifacts if artifact["role"] == "standard"]

    acceptance = study.document["acceptance"]
    min_cases = int(acceptance["min_cases_per_category"])
    category_counts = {
        category: sum(
            1 for case in candidate["cases"] if case["category"] == category
        )
        for category in EVALUATION_CATEGORIES
    }
    enough_cases = all(count >= min_cases for count in category_counts.values())
    comparisons = [_compare_pair(candidate, baseline, study) for baseline in standards]

    reverse_comparisons = [
        _compare_pair(baseline, candidate, study) for baseline in standards
    ]
    dominates = [
        comparison["baseline_id"]
        for comparison in comparisons
        if comparison["dominates"]
    ]
    dominated_by = [
        standard["id"]
        for standard, comparison in zip(standards, reverse_comparisons)
        if comparison["dominates"]
    ]
    all_categories_noninferior = all(
        comparison["category_noninferior"] for comparison in comparisons
    )
    within_memory = (
        int(candidate["artifact_size_bytes"])
        <= int(acceptance["max_artifact_bytes"])
        and float(candidate["peak_ram_bytes"])
        <= float(acceptance["max_peak_ram_bytes"])
    )
    promotion_stage = study.document["study_stage"] == "promotion"

    reasons: list[str] = []
    if not promotion_stage:
        reasons.append("study_stage is development, not promotion")
    if not enough_cases:
        reasons.append(
            "held-out category count is below acceptance.min_cases_per_category"
        )
    if not all_categories_noninferior:
        reasons.append("one or more clinical/general categories exceed their regression limit")
    if not within_memory:
        reasons.append("candidate exceeds the fixed artifact or peak-RAM envelope")
    if not dominates:
        reasons.append("candidate does not strictly dominate any standard artifact")
    if dominated_by:
        reasons.append("candidate remains dominated by: " + ", ".join(dominated_by))

    recommended = not reasons
    worst_regressions = {
        category: max(
            comparison["categories"][category]["regression_upper_confidence"]
            for comparison in comparisons
        )
        for category in EVALUATION_CATEGORIES
    }
    return {
        "schema_version": SCHEMA_VERSION,
        "kind": "ramdoc-quantization-gate-report",
        "study_id": study.document["study_id"],
        "study_manifest_sha256": study.manifest_sha256,
        "candidate_id": candidate_id,
        "recommended": recommended,
        "pareto_frontier_expanded": bool(dominates) and not dominated_by,
        "dominates": dominates,
        "dominated_by": dominated_by,
        "within_fixed_memory_envelope": within_memory,
        "all_categories_noninferior": all_categories_noninferior,
        "minimum_cases_satisfied": enough_cases,
        "category_case_counts": category_counts,
        "category_regression_upper_confidence": worst_regressions,
        "category_regression_limits": dict(
            acceptance["category_regression_limits"]
        ),
        "comparisons": comparisons,
        "reasons": reasons,
    }


def _verify_gguf(path: Path) -> tuple[str, int]:
    try:
        size = path.stat().st_size
        with path.open("rb") as handle:
            magic = handle.read(4)
    except OSError as exc:
        raise QuantizationError(f"cannot inspect candidate artifact {path}: {exc}") from exc
    if magic != b"GGUF":
        raise QuantizationError(f"candidate artifact {path} does not have GGUF magic")
    if not 0 < size <= MAX_ARTIFACT_BYTES:
        raise QuantizationError("candidate GGUF size is outside the safe range")
    return _sha256_file(path), size


def create_promotion(
    results_path: Path,
    results: Mapping[str, Any],
    candidate_id: str,
    candidate_artifact_path: Path,
    recipe_path: Path,
    gate_report: Mapping[str, Any],
    study: ValidatedStudy,
) -> dict[str, Any]:
    if not gate_report["recommended"]:
        raise QuantizationError("a rejected candidate cannot produce a promotion record")
    candidate = next(
        artifact for artifact in results["artifacts"] if artifact["id"] == candidate_id
    )
    candidate_artifact_path = candidate_artifact_path.resolve()
    recipe_path = recipe_path.resolve()
    artifact_sha, artifact_size = _verify_gguf(candidate_artifact_path)
    if candidate_artifact_path.name != candidate.get("filename"):
        raise QuantizationError(
            "candidate artifact filename differs from results metadata"
        )
    if artifact_sha != candidate["sha256"]:
        raise QuantizationError("candidate GGUF SHA-256 differs from results metadata")
    if artifact_size != int(candidate["artifact_size_bytes"]):
        raise QuantizationError("candidate GGUF size differs from results metadata")
    recipe_sha = _sha256_file(recipe_path)
    if recipe_sha != candidate.get("recipe_sha256"):
        raise QuantizationError("recipe SHA-256 differs from candidate results metadata")
    if candidate["base_model_sha256"] != study.document["base_model"]["sha256"]:
        raise QuantizationError("candidate base-model SHA differs from study base model")

    return {
        "schema_version": SCHEMA_VERSION,
        "kind": PROMOTION_KIND,
        "study_id": study.document["study_id"],
        "display_name": candidate["display_name"],
        "created_at": results["completed_at"],
        "artifact": {
            "filename": candidate_artifact_path.name,
            "sha256": artifact_sha,
            "size_bytes": artifact_size,
            "base_model_sha256": candidate["base_model_sha256"],
            "quantization": candidate["quantization"],
            "recipe_sha256": recipe_sha,
        },
        "evidence": {
            "study_manifest_sha256": study.manifest_sha256,
            "held_out_results_sha256": _sha256_file(results_path.resolve()),
            "llama_cpp_commit": study.document["toolchain"]["llama_cpp_commit"],
            "categories": list(EVALUATION_CATEGORIES),
            "baseline_artifacts": [
                artifact["id"]
                for artifact in results["artifacts"]
                if artifact["role"] == "standard"
            ],
        },
        "decision": {
            "recommended": True,
            "pareto_frontier_expanded": True,
            "dominates": list(gate_report["dominates"]),
            "category_regression_upper_confidence": dict(
                gate_report["category_regression_upper_confidence"]
            ),
            "category_regression_limits": dict(
                gate_report["category_regression_limits"]
            ),
        },
    }


def _command_validate(args: argparse.Namespace) -> int:
    study = validate_manifest(args.manifest)
    if args.lock:
        _write_json(args.lock, study_lock(study))
    print(
        json.dumps(
            {
                "study_id": study.document["study_id"],
                "manifest_sha256": study.manifest_sha256,
                "calibration_records": len(study.calibration),
                "evaluation_records": len(study.evaluation),
                "governance": study.document["governance"]["review_status"],
                "valid": True,
            },
            sort_keys=True,
        )
    )
    return 0


def _command_export(args: argparse.Namespace) -> int:
    study = validate_manifest(args.manifest)
    payload = export_calibration(study)
    _atomic_write(args.output, payload)
    print(
        json.dumps(
            {
                "output": str(args.output),
                "records": len(study.calibration),
                "sha256": hashlib.sha256(payload).hexdigest(),
            },
            sort_keys=True,
        )
    )
    return 0


def _command_allocate(args: argparse.Namespace) -> int:
    study = validate_manifest(args.manifest)
    recipe, report = allocate_recipe(args.sensitivity, study)
    _atomic_write(args.recipe, recipe.encode("utf-8"))
    _write_json(args.report, report)
    print(
        json.dumps(
            {
                "recipe": str(args.recipe),
                "report": str(args.report),
                "recipe_sha256": report["recipe_sha256"],
                "estimated_artifact_bytes": report["estimated_artifact_bytes"],
            },
            sort_keys=True,
        )
    )
    return 0


def _command_gate(args: argparse.Namespace) -> int:
    if args.promotion.exists() or args.promotion.is_symlink():
        raise QuantizationError(
            f"promotion output {args.promotion} already exists; archive or remove it before a new gate run"
        )
    study = validate_manifest(args.manifest)
    results = validate_results(args.results, study)
    report = gate_candidate(results, args.candidate, study)
    _write_json(args.report, report)
    if not report["recommended"]:
        print(json.dumps({"recommended": False, "reasons": report["reasons"]}))
        return PROMOTION_EXIT_REJECTED

    promotion = create_promotion(
        args.results,
        results,
        args.candidate,
        args.artifact,
        args.recipe,
        report,
        study,
    )
    _write_json(args.promotion, promotion)
    print(
        json.dumps(
            {
                "recommended": True,
                "promotion": str(args.promotion),
                "dominates": report["dominates"],
            },
            sort_keys=True,
        )
    )
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate = subparsers.add_parser(
        "validate-manifest", help="validate governance, provenance, hashes, and splits"
    )
    validate.add_argument("manifest", type=Path)
    validate.add_argument("--lock", type=Path)
    validate.set_defaults(handler=_command_validate)

    export = subparsers.add_parser(
        "export-calibration", help="materialise only the governed calibration split"
    )
    export.add_argument("manifest", type=Path)
    export.add_argument("--output", type=Path, required=True)
    export.set_defaults(handler=_command_export)

    allocate = subparsers.add_parser(
        "allocate", help="solve the category-wise mixed-bit Pareto allocation"
    )
    allocate.add_argument("manifest", type=Path)
    allocate.add_argument("sensitivity", type=Path)
    allocate.add_argument("--recipe", type=Path, required=True)
    allocate.add_argument("--report", type=Path, required=True)
    allocate.set_defaults(handler=_command_allocate)

    gate = subparsers.add_parser(
        "gate", help="gate a measured candidate and emit an app promotion record"
    )
    gate.add_argument("manifest", type=Path)
    gate.add_argument("results", type=Path)
    gate.add_argument("--candidate", required=True)
    gate.add_argument("--artifact", type=Path, required=True)
    gate.add_argument("--recipe", type=Path, required=True)
    gate.add_argument("--report", type=Path, required=True)
    gate.add_argument("--promotion", type=Path, required=True)
    gate.set_defaults(handler=_command_gate)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return int(args.handler(args))
    except QuantizationError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
