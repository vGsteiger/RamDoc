# Referral-letter quality benchmark

This benchmark compares prompt, reasoning-budget, and sampler changes for German referral
letters without storing patient data or generated letters in Git. It is a narrow regression
harness, not a clinical validation study.

## Safety boundary

- The checked-in fixture is `synthetic_deidentified` and uses only invented `Beispiel` patients.
- Never add real letters, PDFs, copied patient facts, model files, or raw generations to this
  repository.
- The runner requires an absolute output path outside the repository and refuses an in-repository
  destination.
- Aggregate counts and latency may be copied into a PR. Inspect raw text only locally and delete it
  when it is no longer needed.
- A clinician must review generated documents before use. Passing this suite does not establish
  medical correctness or fitness for clinical use.

## What is controlled

The suite pins:

- the exact GGUF filename, byte size, and SHA-256;
- the inference profile and prompt implementation from the checked-out Git revision;
- five synthetic cases, split before experimentation into three development cases and two frozen
  holdouts;
- required facts, required section headings, acceptable alternative wording, and strings that
  flag unsupported claims or leaked placeholders;
- temperature, top-k, top-p, min-p, repetition penalty, presence penalty, seed, thinking effort,
  and Qwen's `/no_think` switch per arm.

The default configuration is selected on the development split. The holdout split is run only for
the preselected finalists. Do not change a preset after examining holdout failures; add new cases
and begin a new versioned suite instead.

## Reproduce the sweep

Use the pinned model declared in
[`benchmarks/local-inference/referral-quality-sweep.json`](../benchmarks/local-inference/referral-quality-sweep.json).
The runner verifies its hash before loading it.

```bash
cd dokassist/src-tauri
cargo run --features benchmark-harness --bin referral-quality-sweep -- validate

cargo run --features benchmark-harness --bin referral-quality-sweep -- run \
  --model /absolute/path/to/Qwen3-8B-Q4_K_M.gguf \
  --output /private/tmp/ramdoc-referral-development.json \
  --split development \
  --repetitions 1
```

Choose finalists using development results only, then run multiple seeds on the frozen holdouts:

```bash
cargo run --features benchmark-harness --bin referral-quality-sweep -- run \
  --model /absolute/path/to/Qwen3-8B-Q4_K_M.gguf \
  --output /private/tmp/ramdoc-referral-holdout.json \
  --split holdout \
  --arms current-medium,current-low,conservative-low \
  --repetitions 3
```

The output records the model hash, configuration hash, Git revision, host architecture, physical
memory, whether the worktree was dirty, the effective context/KV configuration, every sampler
value, latency, token statistics, per-check results, and raw generations. That is why it must
remain outside the repository.

Run the real-GGUF sweep from a normal macOS terminal with Metal access. Sandboxed shells may expose
only a non-selectable `MTL` placeholder and then fail to create a command queue. Do not interpret
that as a quality result or enable a silent CPU fallback; rerun with GPU access and confirm that the
log names the expected Apple GPU.

## Reading the result

Use `arm_summaries` for the first pass:

- `checks_passed / checks_total` measures explicit fact and structure recall;
- `cases_passed / cases_total` requires every check in a case to pass;
- `unsupported_claim_flags` counts failed exclusions and is a safety-oriented veto, not merely
  another quality point;
- `mean_elapsed_ms` captures the latency tradeoff;
- `mean_visible_chars` helps spot truncation or runaway verbosity.

Substring checks are intentionally transparent, but they cannot judge prose quality, clinical
appropriateness, or subtle contradictions. Before changing the production default, add a blinded
clinician preference review of deidentified outputs and report inter-rater agreement alongside
these automated metrics.

## September 2026 development screen

Environment: Qwen3-8B Q4_K_M with the pinned hash, Apple M5 Pro with 24 GB unified memory, Metal,
32K F16 KV context, one run per development case. This screen selected finalists; it did not use
the holdout cases.

| Arm | Checks | Complete cases | Unsupported flags | Mean latency |
| --- | ---: | ---: | ---: | ---: |
| current sampler, medium effort | 55/55 | 3/3 | 0 | 65.7 s |
| current sampler, low effort | 54/55 | 2/3 | 0 | 30.5 s |
| conservative sampler, low effort | 54/55 | 2/3 | 0 | 29.5 s |
| greedy, low effort | 54/55 | 2/3 | 0 | 32.4 s |
| current + repeat penalty, low effort | 54/55 | 2/3 | 0 | 33.6 s |
| Qwen thinking sampler, medium effort | 54/55 | 2/3 | 1 | 62.9 s |
| Qwen non-thinking + presence penalty | 54/55 | 2/3 | 0 | 56.5 s |
| Qwen thinking sampler, low effort | 53/55 | 1/3 | 1 | 36.1 s |
| Qwen non-thinking sampler | 53/55 | 2/3 | 1 | 34.3 s |

The development result initially favoured the current `0.35 / 40 / 0.9 / 0.05` sampler with medium
reasoning. The frozen holdout was then used to challenge—not tune—that selection.

## Frozen holdout result

The three preselected finalists were run three times on each of two untouched cases (six outputs
per arm). A post-run rubric audit added `Keine Medikation` as an accepted exact synonym for
`Aktuell keine Medikation`; no prompt or generated output changed. After that transparent scoring
correction:

| Arm | Checks | Complete cases | Unsupported flags |
| --- | ---: | ---: | ---: |
| current sampler, low effort | 105/105 | 6/6 | 0 |
| conservative sampler, low effort | 105/105 | 6/6 | 0 |
| current sampler, medium effort | 103/105 | 4/6 | 2 |

Both low-effort finalists preserved all checked facts and constraints. Medium effort added
unsupported comorbidity or differential-diagnosis language in two uncertain-ADHD generations.
Accordingly, the safe recommendation for referral letters is low reasoning effort with the current
clinical sampler; the sampler remains user-tunable. Latency from this holdout run is deliberately
omitted because unrelated local compilation overlapped its first two arms and would make the
comparison misleading.

This remains a small synthetic sample. It supports a RamDoc default for this model and task, not a
broad claim about other models, other clinical documents, or real-world clinical correctness.

## Why these knobs

- The [Qwen3 model card](https://huggingface.co/Qwen/Qwen3-8B/blob/main/README.md#best-practices)
  recommends separate thinking and non-thinking sampler profiles, warns against greedy decoding,
  and notes that high presence penalties can reduce performance or mix languages. We tested those
  settings rather than assuming they transfer to clinical prose.
- [llama.cpp's CLI reference](https://github.com/ggml-org/llama.cpp/blob/master/tools/cli/README.md#common-options)
  documents the temperature, top-k, top-p, min-p, repetition, and presence controls available in
  the local runtime.
- Planning the required content and section order is supported by clinical report-generation
  research ([Nishino et al., EMNLP 2022](https://aclanthology.org/2022.emnlp-main.480/)). RamDoc's
  referral prompt therefore declares the document plan instead of relying on sampling alone.
- Independent verification passes can reduce hallucinations in general long-form tasks
  ([Dhuliawala et al., ACL 2024](https://aclanthology.org/2024.findings-acl.212/)). A second-pass
  verifier is a future experiment because it roughly doubles latency and needs its own failure-mode
  evaluation before production use.
- Relevant facts can be missed in the middle of long contexts
  ([Liu et al., 2023](https://arxiv.org/abs/2307.03172)). The benchmark therefore includes fact
  recall, while production context assembly prioritises concise, labelled clinical facts.

Grammar-constrained decoding is useful for intermediate JSON, but was not tested for the final
free-form letter because it can constrain wording without proving factuality. Dynamic temperature,
Mirostat, and large combinatorial sampler chains were excluded from this first sweep because the
model vendor does not recommend them and they add substantial overfitting surface. Add them as
named arms only when there is a specific, predeclared failure mode to address.
