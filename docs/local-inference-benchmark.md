# Local-inference benchmark

Issue #398 adds a versioned benchmark for deciding whether an inference change
improves RamDoc on a 16 GiB Mac. It treats clinical exactness and memory pressure
as release constraints; token speed alone cannot make a result pass.

## Run the full benchmark

On Apple Silicon macOS (running natively, not under Rosetta), with the target
GGUF already downloaded, one command verifies the artifact, builds the Metal
worker, runs the matrix in fresh processes, and writes JSON, CSV, and Markdown:

```sh
./scripts/benchmark-local-inference.py run \
  --model /absolute/path/to/Qwen3-8B-Q4_K_M.gguf
```

The default performs two repetitions at each declared context tier, with the
captured 16K control first so its cold/warm load pair is directly available. A
model is only run at tiers supported by its GGUF native-context metadata;
unsupported tiers are recorded as `skipped`, never silently capped. On the
captured Qwen3-8B baseline, 64K and 128K are therefore explicit skips. A profile
the memory governor considers unsafe is also recorded and skipped before model
tensors are loaded.

For a fast hardware smoke test, use the deterministic subset at 16K once:

```sh
./scripts/benchmark-local-inference.py run \
  --model /absolute/path/to/Qwen3-8B-Q4_K_M.gguf \
  --quick
```

For another model, declare the artifact quantization so the result is not
ambiguous:

```sh
./scripts/benchmark-local-inference.py run \
  --model /absolute/path/to/candidate.gguf \
  --quantization Q4_K_M
```

By default, artifacts land in `benchmark-results/<UTC timestamp>/` (ignored by
Git):

- `results.json`: complete, versioned records for every process and case
- `results.csv`: one flat row per case/repetition, including skipped tiers
- `report.md`: concise context, memory, quality, and performance tables
- `raw/*.json`: the direct output from each fresh benchmark process
- `comparison.json`: regression details when a baseline was supplied

## Compare two builds

Run a candidate against an earlier result and make regressions fail the command:

```sh
./scripts/benchmark-local-inference.py run \
  --model /absolute/path/to/Qwen3-8B-Q4_K_M.gguf \
  --baseline /absolute/path/to/baseline/results.json
```

Already-produced suites can be compared without loading a model:

```sh
./scripts/benchmark-local-inference.py compare \
  /absolute/path/to/baseline/results.json \
  /absolute/path/to/candidate/results.json
```

The checked-in thresholds reject a deterministic clinical pass-rate loss,
TTFT/prefill/total-latency increases above 15%, decode-throughput loss above
10%, whole-process peak-RSS growth above 256 MiB, any swap growth, unreadable
swap telemetry, and a context tier that the baseline ran but the candidate
cannot. Answer changes at temperature zero are retained as hashes for review
even when all exact clinical checks still pass.

## Captured baseline and matrix

The manifest at
[`benchmarks/local-inference/manifest.json`](../benchmarks/local-inference/manifest.json)
is the single source of truth. Its current control is:

| Field | Captured value |
| --- | --- |
| Model | `Qwen3-8B-Q4_K_M.gguf` |
| Artifact SHA-256 | `120307ba529eb2439d6c430d94104dabd578497bc7bfe7e322b5d9933b449bd4` |
| Artifact quantization | Q4_K_M |
| Shipped requested profile | `governed` |
| Effective profile on 16 GiB | `governed-f16` |
| Context / KV | 16,384 / F16 |
| Batch / micro-batch | 2,048 / 512 |
| Completion headroom | 4,096 |
| Flash Attention | enabled |
| llama.cpp bindings | `llama-cpp-2` and `llama-cpp-sys-2` 0.1.146 |

The sweep declares 2K, 8K, 16K, 32K, 64K, and 128K context tiers. The long
middle-retrieval case fills 75% of the prompt budget and records the tokenizer's
actual prompt-token count. The 32K tier uses Q8 KV and the 64K/128K research
tiers use Q4 KV so the matrix tests useful fixed-memory configurations rather
than knowingly unsafe F16 allocations. Every effective context, K/V type,
logical batch, micro-batch, Flash-Attention mode, fallback, and governor estimate
is recorded in the result.

## Workloads and scoring

The synthetic, deidentified cases live in
[`benchmarks/local-inference/clinical-cases.json`](../benchmarks/local-inference/clinical-cases.json).
They cover:

- medication and exact dose
- dates, negation, and chronology
- middle-of-context retrieval and multi-hop evidence
- valid agent tool name and exact required JSON arguments
- German and Swiss orthography
- unsupported claims
- prompt injection in untrusted clinical text

Four execution paths are measured: cold prompt, shared prefix, continued
session, and multi-turn agent/tool call. Warm paths must report a real context
cache hit and at least one reused token. Scoring uses declared tokens,
exclusions, exact values, and JSON argument subsets—there is no nondeterministic
model grader and no aggregate score that can hide a harmful category failure.
Generation uses the engine's fixed seed 0 at temperature 0.

## Recorded performance and provenance

Each generation records prompt processing, TTFT, prefill duration, total
latency, decode throughput, evaluated/reused/completion tokens, and cache status.
Each process also records:

- model filename, artifact size, SHA-256, quantization, and native context
- RamDoc commit and dirty state
- executable and `Cargo.lock` SHA-256 values
- Rust version, pinned llama.cpp crate versions/checksum, and backend system info
- macOS version, CPU/hardware model, architecture, and physical memory
- requested and effective KV/context/batch parameters and governor plan

The sampler reads RSS from the benchmark process that owns the full RamDoc Rust
runtime, model, KV cache, contexts, and allocator state. It reports baseline,
peak, and retained RSS after a declared 250 ms quiescent interval; this is
deliberately not a model-weight-only figure.
Wired and compressed memory and swap are system-wide macOS pressure signals and
are labelled that way in the schema and report.

Artifact verification is timed separately because RamDoc must read the entire
GGUF to establish its hash. Every measured load starts in a new application
process. The first load follows that verification pass; later repetitions also
benefit from the macOS filesystem cache and are reported as warm-filesystem
loads. The harness does not run `purge` or claim to evict macOS caches. For
release numbers, reboot the target Mac, close unrelated applications, connect
power, keep thermal conditions stable, and record any deviation alongside the
result artifacts.

## CI and release use

The small deterministic slice needs no GGUF and is the CI command:

```sh
./scripts/benchmark-local-inference.py ci
```

It validates the manifest, fixture classification and coverage, captured
Qwen/llama.cpp pins, scorer failure modes, memory-summary semantics, and the
comparison regression path. The Rust CI workflow calls it explicitly. Full
model runs are not implicit PR jobs: they require the approved multi-gigabyte
artifact and target hardware, so a release engineer runs the full command on
the reference Mac and attaches `results.json`, `results.csv`, and `report.md`.
