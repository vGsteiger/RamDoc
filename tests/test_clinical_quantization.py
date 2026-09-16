from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from scripts import clinical_quantization as cq


ROOT = Path(__file__).resolve().parents[1]
SYNTHETIC_STUDY = (
    ROOT / "research" / "clinical-quantization" / "study.synthetic.json"
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class StudyFixture:
    def __init__(
        self,
        root: Path,
        *,
        stage: str = "promotion",
        min_cases: int = 2,
        max_artifact_bytes: int = 1_000,
        category_limit: float = 0.05,
    ) -> None:
        self.root = root
        self.manifest = root / "study.json"
        self.calibration = root / "calibration.jsonl"
        self.evaluation = root / "evaluation.jsonl"

        calibration_records = [
            {
                "case_id": f"cal-{category}",
                "family_id": f"cal-family-{category}",
                "category": category,
                "source_id": "synthetic",
                "text": f"Synthetic calibration text for {category}.",
            }
            for category in cq.CALIBRATION_CATEGORIES
        ]
        evaluation_records = [
            {
                "case_id": f"eval-{category}-{index}",
                "family_id": f"eval-family-{category}-{index}",
                "category": category,
                "source_id": "synthetic",
                "text": f"Synthetic held-out text for {category}, variant {index}.",
                "expected": {"kind": "test"},
            }
            for category in cq.EVALUATION_CATEGORIES
            for index in range(min_cases)
        ]
        self._write_jsonl(self.calibration, calibration_records)
        self._write_jsonl(self.evaluation, evaluation_records)
        self.document = {
            "schema_version": 1,
            "kind": cq.STUDY_KIND,
            "study_id": "unit-study-v1",
            "study_stage": stage,
            "governance": {
                "contains_real_clinical_text": False,
                "review_status": "synthetic_only",
            },
            "base_model": {
                "filename": "base-f16.gguf",
                "source": "unit test",
                "license": "Apache-2.0",
                "sha256": "a" * 64,
            },
            "toolchain": {
                "llama_cpp_commit": "b" * 40,
                "commands": {
                    "imatrix": "llama-imatrix -m base-f16.gguf -f calibration.txt -o matrix.gguf",
                    "quantize": "llama-quantize --imatrix matrix.gguf base-f16.gguf candidate.gguf q4_k_m",
                    "evaluate": "python evaluator.py --model candidate.gguf --out results.json",
                },
            },
            "data": {
                "calibration": self._split("calibration.jsonl", calibration_records),
                "evaluation": self._split("evaluation.jsonl", evaluation_records),
            },
            "acceptance": {
                "category_regression_limits": {
                    category: category_limit
                    for category in cq.EVALUATION_CATEGORIES
                },
                "confidence_level": 0.95,
                "max_artifact_bytes": max_artifact_bytes,
                "max_peak_ram_bytes": 1_000,
                "min_cases_per_category": min_cases,
                "minimum_improvement_ratio": 0.05,
                "required_baseline_artifacts": ["q4-standard"],
                "resource_regression_limits": {
                    "artifact_size_bytes": 0.01,
                    "peak_ram_bytes": 0.01,
                    "ttft_ms": 0.05,
                    "prompt_tokens_per_second": 0.05,
                    "decode_tokens_per_second": 0.05,
                },
            },
        }
        self.write_manifest()

    @staticmethod
    def _write_jsonl(path: Path, records: list[dict]) -> None:
        path.write_text(
            "".join(json.dumps(record) + "\n" for record in records),
            encoding="utf-8",
        )

    def _split(self, filename: str, records: list[dict]) -> dict:
        return {
            "path": filename,
            "sha256": sha256(self.root / filename),
            "record_count": len(records),
            "sources": [
                {
                    "id": "synthetic",
                    "origin": "unit test",
                    "license": "CC0-1.0",
                    "consent_basis": "not applicable",
                    "preprocessing": "unit-test JSONL generation",
                    "synthetic": True,
                }
            ],
        }

    def write_manifest(self) -> None:
        self.manifest.write_text(json.dumps(self.document), encoding="utf-8")

    def validated(self) -> cq.ValidatedStudy:
        return cq.validate_manifest(self.manifest)

    def results(
        self,
        *,
        candidate_scores: dict[str, float] | None = None,
        candidate_sha: str = "c" * 64,
        candidate_size: int = 90,
        recipe_sha: str = "d" * 64,
    ) -> dict:
        study = self.validated()
        candidate_scores = candidate_scores or {}

        def cases(overrides: dict[str, float]) -> list[dict]:
            return [
                {
                    "case_id": record["case_id"],
                    "category": record["category"],
                    "score": overrides.get(record["category"], 1.0),
                }
                for record in study.evaluation
            ]

        return {
            "schema_version": 1,
            "kind": cq.RESULTS_KIND,
            "study_manifest_sha256": study.manifest_sha256,
            "evidence_status": "measured",
            "llama_cpp_commit": "b" * 40,
            "completed_at": "2026-08-17T12:00:00Z",
            "artifacts": [
                {
                    "id": "q4-standard",
                    "role": "standard",
                    "display_name": "Q4 standard",
                    "filename": "standard.gguf",
                    "quantization": "Q4_K_M",
                    "sha256": "e" * 64,
                    "base_model_sha256": "a" * 64,
                    "artifact_size_bytes": 100,
                    "peak_ram_bytes": 100,
                    "ttft_ms": 100,
                    "prompt_tokens_per_second": 100,
                    "decode_tokens_per_second": 100,
                    "cases": cases({}),
                },
                {
                    "id": "clinical-mix",
                    "role": "candidate",
                    "display_name": "RamDoc clinical mix",
                    "filename": "clinical-mix.gguf",
                    "quantization": "RamDoc-Mix-v1",
                    "sha256": candidate_sha,
                    "base_model_sha256": "a" * 64,
                    "recipe_sha256": recipe_sha,
                    "artifact_size_bytes": candidate_size,
                    "peak_ram_bytes": 90,
                    "ttft_ms": 100,
                    "prompt_tokens_per_second": 100,
                    "decode_tokens_per_second": 100,
                    "cases": cases(candidate_scores),
                },
            ],
        }


class ManifestTests(unittest.TestCase):
    def test_checked_in_synthetic_study_is_valid_and_strictly_split(self) -> None:
        study = cq.validate_manifest(SYNTHETIC_STUDY)
        calibration = cq.export_calibration(study).decode("utf-8")
        self.assertEqual(len(study.calibration), 8)
        self.assertEqual(len(study.evaluation), 12)
        self.assertIn("Zoloft", calibration)
        self.assertNotIn("ALPEN-47", calibration)

    def test_hash_mismatch_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = StudyFixture(Path(directory))
            fixture.document["data"]["evaluation"]["sha256"] = "0" * 64
            fixture.write_manifest()
            with self.assertRaisesRegex(cq.QuantizationError, "hash mismatch"):
                fixture.validated()

    def test_real_clinical_text_requires_completed_governance(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = StudyFixture(Path(directory))
            fixture.document["governance"] = {
                "contains_real_clinical_text": True,
                "review_status": "pending",
            }
            fixture.write_manifest()
            with self.assertRaisesRegex(cq.QuantizationError, "blocked"):
                fixture.validated()

    def test_paraphrase_family_leakage_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = StudyFixture(Path(directory))
            records = [json.loads(line) for line in fixture.evaluation.read_text().splitlines()]
            records[0]["family_id"] = "cal-family-medication"
            fixture._write_jsonl(fixture.evaluation, records)
            fixture.document["data"]["evaluation"]["sha256"] = sha256(fixture.evaluation)
            fixture.write_manifest()
            with self.assertRaisesRegex(cq.QuantizationError, "paraphrase leakage"):
                fixture.validated()

    def test_promotion_study_rejects_a_placeholder_source_digest(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = StudyFixture(Path(directory), stage="promotion")
            fixture.document["base_model"]["sha256"] = "0" * 64
            fixture.write_manifest()
            with self.assertRaisesRegex(cq.QuantizationError, "all-zero"):
                fixture.validated()


class AllocationTests(unittest.TestCase):
    @staticmethod
    def _zero_regressions() -> dict[str, float]:
        return {category: 0.0 for category in cq.EVALUATION_CATEGORIES}

    def test_allocator_uses_category_pareto_frontier_not_average_loss(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = StudyFixture(
                root, max_artifact_bytes=8, category_limit=0.3, min_cases=1
            )
            study = fixture.validated()
            a_low = self._zero_regressions()
            a_low["dose"] = 0.25
            b_low = self._zero_regressions()
            b_low["medication"] = 0.20
            sensitivity = {
                "schema_version": 1,
                "kind": cq.SENSITIVITY_KIND,
                "study_manifest_sha256": study.manifest_sha256,
                "evidence_status": "measured",
                "default_type": "q3_k_m",
                "fixed_bytes": 0,
                "categories": list(cq.EVALUATION_CATEGORIES),
                "tensor_inventory": [
                    "blk.0.attn_q.weight",
                    "blk.0.ffn_down.weight",
                ],
                "groups": [
                    {
                        "name": "attention_q",
                        "selector": "attn_q",
                        "domains": ["attention", "clinical_sensitive"],
                        "options": [
                            {
                                "type": "q3_k",
                                "estimated_bytes": 2,
                                "regression_upper_95": a_low,
                            },
                            {
                                "type": "q8_0",
                                "estimated_bytes": 6,
                                "regression_upper_95": self._zero_regressions(),
                            },
                        ],
                    },
                    {
                        "name": "ffn_down",
                        "selector": "ffn_down",
                        "domains": ["mlp", "embedding_output"],
                        "options": [
                            {
                                "type": "q3_k",
                                "estimated_bytes": 2,
                                "regression_upper_95": b_low,
                            },
                            {
                                "type": "q8_0",
                                "estimated_bytes": 6,
                                "regression_upper_95": self._zero_regressions(),
                            },
                        ],
                    },
                ],
            }
            sensitivity_path = root / "sensitivity.json"
            sensitivity_path.write_text(json.dumps(sensitivity), encoding="utf-8")
            recipe, report = cq.allocate_recipe(sensitivity_path, study)
            self.assertEqual(recipe, "attn_q=q8_0\nffn_down=q3_k\n")
            self.assertEqual(report["estimated_artifact_bytes"], 8)
            self.assertEqual(
                report["predicted_regression_upper_95"]["medication"], 0.20
            )
            self.assertEqual(report["predicted_regression_upper_95"]["dose"], 0.0)
            self.assertEqual(report["tensor_inventory_count"], 2)
            self.assertEqual(
                report["matched_tensors_by_group"]["attention_q"],
                ["blk.0.attn_q.weight"],
            )

    def test_allocator_rejects_a_hidden_critical_category_regression(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = StudyFixture(
                root, max_artifact_bytes=2, category_limit=0.3, min_cases=1
            )
            study = fixture.validated()
            unsafe = self._zero_regressions()
            unsafe["dose"] = 0.31
            sensitivity = {
                "schema_version": 1,
                "kind": cq.SENSITIVITY_KIND,
                "study_manifest_sha256": study.manifest_sha256,
                "evidence_status": "measured",
                "default_type": "q3_k_m",
                "fixed_bytes": 0,
                "categories": list(cq.EVALUATION_CATEGORIES),
                "tensor_inventory": ["output.weight"],
                "groups": [
                    {
                        "name": "output",
                        "selector": "output.weight",
                        "domains": list(cq.SENSITIVITY_DOMAINS),
                        "options": [
                            {
                                "type": "q3_k",
                                "estimated_bytes": 2,
                                "regression_upper_95": unsafe,
                            }
                        ],
                    }
                ],
            }
            path = root / "sensitivity.json"
            path.write_text(json.dumps(sensitivity), encoding="utf-8")
            with self.assertRaisesRegex(cq.QuantizationError, "no mixed-bit recipe"):
                cq.allocate_recipe(path, study)

    def test_allocator_does_not_allow_negative_regressions_to_cancel_loss(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = StudyFixture(root, max_artifact_bytes=2, min_cases=1)
            study = fixture.validated()
            invalid = self._zero_regressions()
            invalid["dose"] = -0.1
            sensitivity = {
                "schema_version": 1,
                "kind": cq.SENSITIVITY_KIND,
                "study_manifest_sha256": study.manifest_sha256,
                "evidence_status": "measured",
                "default_type": "q3_k_m",
                "fixed_bytes": 0,
                "categories": list(cq.EVALUATION_CATEGORIES),
                "tensor_inventory": ["output.weight"],
                "groups": [
                    {
                        "name": "output",
                        "selector": "output.weight",
                        "domains": list(cq.SENSITIVITY_DOMAINS),
                        "options": [
                            {
                                "type": "q3_k",
                                "estimated_bytes": 2,
                                "regression_upper_95": invalid,
                            }
                        ],
                    }
                ],
            }
            path = root / "sensitivity.json"
            path.write_text(json.dumps(sensitivity), encoding="utf-8")
            with self.assertRaisesRegex(cq.QuantizationError, r"must be in \[0, 1\]"):
                cq.allocate_recipe(path, study)

    def test_allocator_requires_all_issue_405_tensor_domains(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = StudyFixture(root, max_artifact_bytes=2, min_cases=1)
            study = fixture.validated()
            sensitivity = {
                "schema_version": 1,
                "kind": cq.SENSITIVITY_KIND,
                "study_manifest_sha256": study.manifest_sha256,
                "evidence_status": "measured",
                "default_type": "q4_k_m",
                "fixed_bytes": 0,
                "categories": list(cq.EVALUATION_CATEGORIES),
                "tensor_inventory": ["blk.0.attn_q.weight"],
                "groups": [
                    {
                        "name": "attention_only",
                        "selector": "attn_q",
                        "domains": ["attention"],
                        "options": [
                            {
                                "type": "q4_k",
                                "estimated_bytes": 2,
                                "regression_upper_95": self._zero_regressions(),
                            }
                        ],
                    }
                ],
            }
            path = root / "sensitivity.json"
            path.write_text(json.dumps(sensitivity), encoding="utf-8")
            with self.assertRaisesRegex(cq.QuantizationError, "tensor domains"):
                cq.allocate_recipe(path, study)

    def test_allocator_rejects_overlapping_llama_cpp_tensor_regexes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = StudyFixture(root, max_artifact_bytes=4, min_cases=1)
            study = fixture.validated()
            option = {
                "type": "q4_k",
                "estimated_bytes": 2,
                "regression_upper_95": self._zero_regressions(),
            }
            sensitivity = {
                "schema_version": 1,
                "kind": cq.SENSITIVITY_KIND,
                "study_manifest_sha256": study.manifest_sha256,
                "evidence_status": "measured",
                "default_type": "q4_k_m",
                "fixed_bytes": 0,
                "categories": list(cq.EVALUATION_CATEGORIES),
                "tensor_inventory": ["blk.0.attn_q.weight"],
                "groups": [
                    {
                        "name": "broad_attention",
                        "selector": "attn",
                        "domains": ["attention", "clinical_sensitive"],
                        "options": [option],
                    },
                    {
                        "name": "specific_attention",
                        "selector": "attn_q",
                        "domains": ["mlp", "embedding_output"],
                        "options": [option],
                    },
                ],
            }
            path = root / "sensitivity.json"
            path.write_text(json.dumps(sensitivity), encoding="utf-8")
            with self.assertRaisesRegex(cq.QuantizationError, "overlaps group"):
                cq.allocate_recipe(path, study)


class GateTests(unittest.TestCase):
    def test_smaller_equal_quality_candidate_expands_the_frontier(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = StudyFixture(Path(directory))
            study = fixture.validated()
            report = cq.gate_candidate(fixture.results(), "clinical-mix", study)
            self.assertTrue(report["recommended"])
            self.assertEqual(report["dominates"], ["q4-standard"])
            self.assertEqual(report["dominated_by"], [])

    def test_dose_regression_cannot_hide_behind_resource_savings(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = StudyFixture(Path(directory), category_limit=0.05)
            study = fixture.validated()
            report = cq.gate_candidate(
                fixture.results(candidate_scores={"dose": 0.0}),
                "clinical-mix",
                study,
            )
            self.assertFalse(report["recommended"])
            self.assertFalse(report["all_categories_noninferior"])
            self.assertGreater(
                report["category_regression_upper_confidence"]["dose"], 0.05
            )

    def test_development_study_cannot_promote(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            fixture = StudyFixture(Path(directory), stage="development")
            report = cq.gate_candidate(
                fixture.results(), "clinical-mix", fixture.validated()
            )
            self.assertFalse(report["recommended"])
            self.assertIn("study_stage", report["reasons"][0])

    def test_results_cannot_omit_a_predeclared_standard_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = StudyFixture(root)
            fixture.document["acceptance"]["required_baseline_artifacts"] = [
                "q3-standard",
                "q4-standard",
            ]
            fixture.write_manifest()
            results_path = root / "results.json"
            results_path.write_text(json.dumps(fixture.results()), encoding="utf-8")
            with self.assertRaisesRegex(cq.QuantizationError, "q3-standard"):
                cq.validate_results(results_path, fixture.validated())

    def test_results_cannot_compare_a_baseline_from_different_source_weights(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = StudyFixture(root)
            results = fixture.results()
            results["artifacts"][0]["base_model_sha256"] = "f" * 64
            results_path = root / "results.json"
            results_path.write_text(json.dumps(results), encoding="utf-8")
            with self.assertRaisesRegex(cq.QuantizationError, "pinned base model"):
                cq.validate_results(results_path, fixture.validated())

    def test_promotion_is_bound_to_actual_gguf_recipe_and_evidence_hashes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = StudyFixture(root)
            artifact = root / "clinical-mix.gguf"
            artifact.write_bytes(b"GGUFunit-test-model")
            recipe = root / "mixed-bit.recipe"
            recipe.write_text("attn_q=q8_0\n", encoding="utf-8")
            results = fixture.results(
                candidate_sha=sha256(artifact),
                candidate_size=artifact.stat().st_size,
                recipe_sha=sha256(recipe),
            )
            results_path = root / "results.json"
            results_path.write_text(json.dumps(results), encoding="utf-8")
            study = fixture.validated()
            validated_results = cq.validate_results(results_path, study)
            report = cq.gate_candidate(validated_results, "clinical-mix", study)
            promotion = cq.create_promotion(
                results_path,
                validated_results,
                "clinical-mix",
                artifact,
                recipe,
                report,
                study,
            )
            self.assertEqual(promotion["kind"], cq.PROMOTION_KIND)
            self.assertEqual(promotion["artifact"]["sha256"], sha256(artifact))
            self.assertEqual(
                promotion["evidence"]["held_out_results_sha256"], sha256(results_path)
            )

            artifact.write_bytes(b"GGUFtampered")
            with self.assertRaisesRegex(cq.QuantizationError, "SHA-256"):
                cq.create_promotion(
                    results_path,
                    validated_results,
                    "clinical-mix",
                    artifact,
                    recipe,
                    report,
                    study,
                )


if __name__ == "__main__":
    unittest.main()
