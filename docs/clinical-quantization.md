# Clinical calibration and mixed-bit quantization

Issue #405 is implemented as an offline-to-app pipeline. It does **not** ship a
claimed clinical result without measurements. It ships the governed data and
result contracts, an exact mixed-bit allocator, a held-out promotion gate, and
a guarded RamDoc import path. The checked-in corpus is synthetic and the sample
study is deliberately marked `development`, so it cannot produce a promotion.

The workflow uses llama.cpp's importance-matrix and tensor override interfaces,
pinned to the commit recorded in each study manifest. The upstream interfaces
are documented in the [imatrix tool](https://github.com/ggml-org/llama.cpp/blob/9f0d017efb4a388bd5c60a27a575c90f20868e51/tools/imatrix/README.md)
and the pinned [`llama-quantize` source](https://github.com/ggml-org/llama.cpp/blob/9f0d017efb4a388bd5c60a27a575c90f20868e51/tools/quantize/quantize.cpp).

## The improvement: category-Pareto minimax allocation

Ordinary mixed-precision searches usually collapse quality into one number. In
clinical text, that can trade a dose or negation failure for several harmless
instruction-following gains and still report an improved average. RamDoc keeps
quality as a vector instead:

```text
state = (serialized bytes,
         medication regression,
         dose regression,
         date regression,
         negation regression,
         ...,
         long-context regression)
```

For every tensor group and candidate quantization type, the sensitivity input
records the upper 95% bound of its measured regression in all twelve categories.
The allocator composes groups and removes a state only when another state is no
larger **and** no worse in every category. This computes the exact surviving
Pareto frontier rather than a weighted approximation. From the states inside
the fixed artifact budget and every category's loss budget, it chooses the
minimax-normalised regret:

```text
minimise max(category regression / category limit)
```

Ties minimise total normalised regret and then bytes. The output is a plain
`selector=ggml_type` file accepted directly by llama.cpp's
`--tensor-type-file` option.

Sensitivity effects are modelled additively during allocation. That is only a
search heuristic: the complete quantized artifact must then pass the paired
held-out gate. Cross-tensor interactions therefore cannot earn a production
promotion merely because the allocator predicted that they were safe.

## Files

| Path | Purpose |
| --- | --- |
| `scripts/clinical_quantization.py` | Manifest validator, calibration exporter, allocator, held-out gate, and promotion-record writer |
| `research/clinical-quantization/study.synthetic.json` | Valid, synthetic, non-promotable study example |
| `research/clinical-quantization/data/*.jsonl` | Disjoint German/Swiss calibration and held-out smoke fixtures |
| `tests/test_clinical_quantization.py` | Contract, leakage, allocator, gate, and artifact-binding tests |
| `dokassist/src-tauri/src/llm/quantization.rs` | App-side promotion validation and streaming GGUF import |

Generated calibration text, importance matrices, GGUFs, raw model responses,
and evaluation results are study artifacts. Do not commit real clinical text or
large model artifacts to this repository.

## 1. Govern and lock the data

Start from the synthetic manifest, but create a new study ID and replace every
placeholder. A promotion study must pin:

- the exact F16/BF16 source-model digest and license;
- one full llama.cpp commit SHA;
- exact imatrix, quantization, and evaluation commands;
- the calibration and evaluation file hashes and record counts;
- source-level provenance, license, consent basis, and whether each source is
  synthetic, plus its exact preprocessing description;
- category and resource regression limits before evaluating candidates.
- required standard artifact IDs (normally Q3, Q4, Q5, and an imatrix control),
  so an inconvenient baseline cannot be omitted after seeing results.

If any real clinical text is present, `contains_real_clinical_text` must be
`true` and the manifest must record an approved governance review, reviewer,
review time, consent/legal basis, and independently verified deidentification
method. The validator blocks the data before export otherwise.

Each JSONL record has a `case_id`, `family_id`, `category`, `source_id`, and
`text`. Evaluation records also carry an `expected` specification. Isolation is
checked at three levels:

- no repeated case IDs;
- no family IDs crossing splits, which prevents minimal-pair or paraphrase
  leakage;
- no identical NFKC-normalised text crossing splits.

Validate the manifest and write a small content lock for the experiment log:

```sh
python3 scripts/clinical_quantization.py validate-manifest \
  research/clinical-quantization/study.synthetic.json \
  --lock /absolute/study-output/study.lock.json
```

The synthetic example covers medication identity, dose, dates, negation,
uncertainty, chronology, unsupported claims, Swiss German usage, report
structure, exact tool calls, general instructions, and long-context retrieval.
It has one smoke case per category, whereas its declared promotion minimum is
30. This mismatch is intentional: the fixture tests plumbing, not model quality.

## 2. Export calibration without exposing the held-out split

```sh
python3 scripts/clinical_quantization.py export-calibration \
  /absolute/study.json \
  --output /absolute/study-output/calibration.txt
```

The exporter revalidates governance and every digest, then reads only the
calibration path. The held-out text is never present in the imatrix input.

Run the exact pinned command recorded in the study, for example:

```sh
/absolute/llama.cpp/build/bin/llama-imatrix \
  -m /absolute/Qwen3-8B-F16.gguf \
  -f /absolute/study-output/calibration.txt \
  -o /absolute/study-output/clinical-imatrix.gguf \
  -ngl 99
```

Record the generated imatrix digest alongside the study lock. Never requantize
an existing low-bit model for a result intended for promotion.

## 3. Measure tensor-group sensitivity

Create one controlled artifact per tensor-group/type perturbation from the same
source model and imatrix. Evaluate it on calibration probes only. The resulting
JSON contract is:

```json
{
  "schema_version": 1,
  "kind": "ramdoc-quantization-sensitivity",
  "study_manifest_sha256": "<sha256>",
  "evidence_status": "measured",
  "default_type": "q4_k_m",
  "fixed_bytes": 600000000,
  "categories": ["medication", "dose", "date", "negation", "uncertainty", "chronology", "unsupported_claim", "german_swiss", "report_generation", "tool_call", "general_instruction", "long_context"],
  "tensor_inventory": [
    "token_embd.weight",
    "blk.0.attn_v.weight",
    "blk.0.ffn_down.weight",
    "output.weight"
  ],
  "groups": [
    {
      "name": "attention_value",
      "selector": "attn_v",
      "domains": ["attention", "clinical_sensitive"],
      "options": [
        {
          "type": "q4_k",
          "estimated_bytes": 700000000,
          "regression_upper_95": {
            "medication": 0.002,
            "dose": 0.0,
            "date": 0.001,
            "negation": 0.0,
            "uncertainty": 0.002,
            "chronology": 0.001,
            "unsupported_claim": 0.0,
            "german_swiss": 0.001,
            "report_generation": 0.002,
            "tool_call": 0.0,
            "general_instruction": 0.002,
            "long_context": 0.003
          }
        }
      ]
    }
  ]
}
```

Every group declares one or more `domains`. Their union must cover `attention`,
`mlp`, `embedding_output`, and `clinical_sensitive`; the last is an explicit
cross-cutting tag for groups selected from the clinical sensitivity study. The
allocation report retains the selectors contributing to each domain, so tensor
coverage is reviewable instead of being inferred from naming conventions.

`tensor_inventory` is the lowercase tensor-name list from the pinned source
GGUF. Selectors use llama.cpp's regex-search semantics. The allocator resolves
every selector against that inventory, rejects empty or overlapping groups, and
records the exact matches plus an inventory digest. This prevents an apparently
valid byte allocation from silently becoming a different recipe because a
broad regex captured another group's tensors first.

`fixed_bytes` covers tensors not represented by selectable groups. Group byte
estimates must be mutually exclusive so they sum to the predicted artifact
size. Use upper confidence bounds, not point estimates, in
`regression_upper_95`. An illustrative or generated input must use
`synthetic_smoke_test`; its allocation report can be inspected, but the final
gate accepts only measured held-out results.

## 4. Allocate and build the mixed-bit candidate

```sh
python3 scripts/clinical_quantization.py allocate \
  /absolute/study.json \
  /absolute/sensitivity.json \
  --recipe /absolute/study-output/mixed-bit.recipe \
  --report /absolute/study-output/allocation.json

/absolute/llama.cpp/build/bin/llama-quantize \
  --imatrix /absolute/study-output/clinical-imatrix.gguf \
  --tensor-type-file /absolute/study-output/mixed-bit.recipe \
  /absolute/Qwen3-8B-F16.gguf \
  /absolute/study-output/Qwen3-8B-RamDoc-Mix.gguf \
  Q4_K_M
```

The allocation report records the sensitivity and recipe hashes, selected type
per group, predicted per-category upper regressions, exact estimated bytes, and
the number of Pareto states surviving each group. A run fails rather than
silently falling back to a weighted average if no recipe satisfies every
category limit.

Build standard Q3, Q4, and Q5/imatrix controls from the same source commit and
source weights. Do not compare an F16-derived candidate against a requantized
control.

## 5. Evaluate the complete artifacts on held-out cases

Run every artifact at temperature zero on exactly the evaluation case IDs and
record a score in `[0, 1]` per case. The result document must use kind
`ramdoc-held-out-evaluation`, `evidence_status: measured`, the study-manifest
digest and pinned llama.cpp commit. Every artifact records:

- role (`standard` or `candidate`), display name, filename, quantization label,
  source-model SHA-256 and GGUF SHA-256;
- artifact bytes, peak total RamDoc memory, TTFT, prompt speed, and decode
  speed;
- every held-out `case_id`, its unchanged category, and its score;
- `recipe_sha256` for candidates.

The validator requires the exact same held-out case set and category assignment
for every artifact. It reports every category independently. The gate uses a
paired one-sided confidence bound on candidate-minus-baseline scores, so a dose
failure cannot be hidden by unrelated wins.

## 6. Gate and promote

Put the candidate GGUF and desired promotion JSON in the same directory. Then:

```sh
python3 scripts/clinical_quantization.py gate \
  /absolute/study.json \
  /absolute/held-out-results.json \
  --candidate clinical-mix \
  --artifact /absolute/study-output/Qwen3-8B-RamDoc-Mix.gguf \
  --recipe /absolute/study-output/mixed-bit.recipe \
  --report /absolute/study-output/gate-report.json \
  --promotion /absolute/study-output/Qwen3-8B-RamDoc-Mix.promotion.json
```

The report is written for positive and negative results. A rejected candidate
returns exit status `3` and does not write a promotion record. The promotion
output path must be new, so a stale positive record cannot survive unnoticed
across a later gate run. Recommendation requires all of the following:

1. `study_stage` is `promotion` and each category has its predeclared minimum
   held-out count.
2. Artifact size and measured peak RamDoc memory stay inside the fixed envelope.
3. The upper confidence bound of regression stays inside its own limit for
   every clinical and general category against every standard artifact.
4. The candidate strictly dominates at least one standard artifact in quality
   or resources, stays within every declared resource tolerance, and is not
   dominated by another standard artifact.
5. The actual GGUF magic, full GGUF SHA-256, GGUF size, recipe SHA-256, source
   model SHA-256, results SHA-256, manifest SHA-256, and llama.cpp commit all
   agree.

This records negative findings and per-category regressions without producing a
misleading combined score.

## 7. Import into RamDoc

In **Settings → Model Management**, choose **Import verified model**. The dialog
asks for the promotion JSON first and previews the display name, size, and
quantization label. If the GGUF named by the record sits next to the JSON,
RamDoc offers it immediately; otherwise you can choose the GGUF from another
folder. Import then streams a progress bar while the file is hashed and copied.

RamDoc:

- parses a bounded, strict schema;
- rechecks that the Pareto decision is positive and every category is within
  its recorded limit;
- streams the whole GGUF through SHA-256 while copying it into the app-owned
  model directory;
- repeats the full digest check immediately before every load, rejecting any
  post-import mutation;
- refuses traversal, non-GGUF content, oversized input, mismatched hashes or
  sizes, and replacement of a different installed model;
- stores the normalised promotion record beside the model and registers the
  model only after verification.

The installed-model card shows a **Verified recipe** badge. Expanding
**Verification details** reveals the quantization label, baselines, study ID,
worst per-category regression upper bound, and abbreviated held-out evidence
hash. Deleting the model also deletes its promotion sidecar.

A promotion record is content-addressed provenance, not a remote signature or a
claim that the model is clinically validated. It proves that the locally chosen
artifact matches the record; the reported study hashes and gate decision still
rely on the provenance of that locally supplied record. Distribution through an
official model channel would still need the repository's existing
release/signature and hardcoded-whitelist review.

## Verification

The Python suite covers governance rejection, hash drift, family leakage,
category-Pareto allocation, infeasible clinical budgets, dose-regression veto,
development-stage veto, GGUF tampering, and evidence-hash binding. Rust tests
cover strict promotion parsing, category-limit enforcement, traversal, streamed
copy/hash verification, sidecar persistence, and tampered-source cleanup.

```sh
python3 -m unittest tests/test_clinical_quantization.py -v

cd dokassist/src-tauri
cargo test --lib llm::quantization
```
