use futures_util::StreamExt;
use reqwest::header::{CONTENT_RANGE, RANGE};
use ring::digest::{Context as DigestContext, SHA256};
use std::path::Path;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use crate::error::AppError;

/// CRIT-4: Hard maximum download size — 60 GiB.
/// Aborts the stream if the server sends more bytes than this.
const MAX_DOWNLOAD_BYTES: u64 = 60 * 1024 * 1024 * 1024;

/// Maximum size we'll accept for an LFS pointer body (they're ~130 bytes).
const MAX_POINTER_BYTES: usize = 4096;

/// Single source of truth for all whitelisted models.
/// Both the download URL and the LFS pointer URL are co-located so they
/// cannot diverge — adding a model in one place without the other is a
/// compile error (missing struct field).
///
/// Every entry carries a compile-time SHA-256 pin. The runtime LFS pointer must
/// match that pin before a large download starts, and the completed GGUF must
/// match the same digest. This prevents a mutable `main` branch or compromised
/// pointer response from silently changing an approved model.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ModelEntry {
    pub name: &'static str,
    pub filename: &'static str,
    /// CRIT-4: HuggingFace blob URL (resolve/main) — used for the actual download.
    pub download_url: &'static str,
    /// CRIT-3: HuggingFace raw-git URL (raw/main) — returns the LFS pointer text
    /// (~130 bytes) containing the authoritative SHA-256 of the blob.
    pub lfs_pointer_url: &'static str,
    /// CRIT-3: SHA-256 of the GGUF blob, pinned at compile time.
    /// Must match the "oid sha256:" value in the LFS pointer exactly.
    pub pinned_sha256: &'static str,
    pub size_bytes: u64,
    pub min_ram_gb: u64,
    pub context_window_tokens: u64,
    pub parameters: &'static str,
    pub license: &'static str,
    pub description: &'static str,
    pub disclaimer: Option<&'static str>,
    /// Filename of the entry that replaces this one. Superseded entries stay
    /// whitelisted so existing installs keep loading, routing and verifying,
    /// but the picker only lists them once they are installed.
    pub superseded_by: Option<&'static str>,
}

pub(crate) const MODELS: &[ModelEntry] = &[
    // Apache-2.0; 30.7B dense; Google QAT Q4_0; minimum 32 GB unified memory.
    ModelEntry {
        name: "Gemma 4 31B QAT Q4_0",
        filename: "gemma-4-31B_q4_0-it.gguf",
        download_url: "https://huggingface.co/google/gemma-4-31B-it-qat-q4_0-gguf/resolve/main/gemma-4-31B_q4_0-it.gguf",
        lfs_pointer_url: "https://huggingface.co/google/gemma-4-31B-it-qat-q4_0-gguf/raw/main/gemma-4-31B_q4_0-it.gguf",
        pinned_sha256: "179cfb99212709597eae5929112cfca677e1bbf566178b479ae1da0c4772874b",
        size_bytes: 17_651_001_568,
        min_ram_gb: 32,
        context_window_tokens: 262_144,
        parameters: "30.7B dense",
        license: "Apache-2.0",
        description: "Largest dense Gemma 4 with Google's quantization-aware Q4_0 weights; strong multilingual writing with 256K context.",
        disclaimer: None,
        superseded_by: None,
    },
    // Apache-2.0; 29.3B dense; IBM vendor GGUF; minimum 32 GB unified memory.
    ModelEntry {
        name: "Granite 4.2 30B Q4_K_M",
        filename: "granite-4.2-30b-Q4_K_M.gguf",
        download_url: "https://huggingface.co/ibm-granite/granite-4.2-30b-GGUF/resolve/main/granite-4.2-30b-Q4_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/ibm-granite/granite-4.2-30b-GGUF/raw/main/granite-4.2-30b-Q4_K_M.gguf",
        pinned_sha256: "f299dace85d77ee0e24ca0b6720bd211b1f4cd1f65b9c0a2111e79692e29c9be",
        size_bytes: 17_721_455_328,
        min_ram_gb: 32,
        context_window_tokens: 131_072,
        parameters: "29.3B dense",
        license: "Apache-2.0",
        description: "IBM's enterprise model with officially evaluated German and French, reasoning mode and 128K context.",
        disclaimer: None,
        superseded_by: None,
    },
    // OpenMDW-1.1; 30B total / 3B active hybrid Mamba-2 MoE; minimum 32 GB unified memory.
    ModelEntry {
        name: "Nemotron 3.5 Lightning 30B-A3B Q4_0",
        filename: "NVIDIA-Nemotron-3.5-Lightning-30B-A3B-Q4_0.gguf",
        download_url: "https://huggingface.co/ggml-org/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-GGUF/resolve/main/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-Q4_0.gguf",
        lfs_pointer_url: "https://huggingface.co/ggml-org/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-GGUF/raw/main/NVIDIA-Nemotron-3.5-Lightning-30B-A3B-Q4_0.gguf",
        pinned_sha256: "61f87e75974e4b535dcdf9aad056541a9514f1dfa4538b463b081d19b7a00e3c",
        size_bytes: 18_898_091_584,
        min_ram_gb: 32,
        context_window_tokens: 1_048_576,
        parameters: "30B total / 3B active",
        license: "OpenMDW-1.1",
        description: "Fast hybrid Mamba-MoE with very long context for large records; supports German and French.",
        disclaimer: None,
        superseded_by: None,
    },
    // Apache-2.0; 27.8B dense; minimum 24 GB unified memory.
    ModelEntry {
        name: "Qwen 3.8 27B Dense UD-Q4_K_M",
        filename: "Qwen3.8-27B-UD-Q4_K_M.gguf",
        download_url: "https://huggingface.co/unsloth/Qwen3.8-27B-GGUF/resolve/main/Qwen3.8-27B-UD-Q4_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/unsloth/Qwen3.8-27B-GGUF/raw/main/Qwen3.8-27B-UD-Q4_K_M.gguf",
        pinned_sha256: "1bce504dfecd84153666928a41d8a66706f0158d3a317bc5196b72819d7dc529",
        size_bytes: 16_564_259_744,
        min_ram_gb: 24,
        context_window_tokens: 262_144,
        parameters: "27.8B dense",
        license: "Apache-2.0",
        description: "Successor to Qwen 3.6 27B: stronger multilingual reasoning and tool use with 256K context.",
        disclaimer: None,
        superseded_by: None,
    },
    // Apache-2.0; 25.2B total / 3.8B active; Google QAT Q4_0; minimum 24 GB unified memory.
    ModelEntry {
        name: "Gemma 4 26B-A4B MoE QAT Q4_0",
        filename: "gemma-4-26B_q4_0-it.gguf",
        download_url: "https://huggingface.co/google/gemma-4-26B-A4B-it-qat-q4_0-gguf/resolve/main/gemma-4-26B_q4_0-it.gguf",
        lfs_pointer_url: "https://huggingface.co/google/gemma-4-26B-A4B-it-qat-q4_0-gguf/raw/main/gemma-4-26B_q4_0-it.gguf",
        pinned_sha256: "3eca3b8f6d7baf218a7dd6bba5fb59a56ee25fe2d567b6f5f589b4f697eca51d",
        size_bytes: 14_439_363_584,
        min_ram_gb: 24,
        context_window_tokens: 262_144,
        parameters: "25.2B total / 3.8B active",
        license: "Apache-2.0",
        description: "Google's quantization-aware Q4_0 release of the Gemma 4 MoE; better quality than post-training Q4_0 at the same size.",
        disclaimer: None,
        superseded_by: None,
    },
    // Apache-2.0; 13.5B dense; minimum 16 GB unified memory.
    ModelEntry {
        name: "Ministral 3 14B Q4_K_M",
        filename: "Ministral-3-14B-Instruct-2512-Q4_K_M.gguf",
        download_url: "https://huggingface.co/unsloth/Ministral-3-14B-Instruct-2512-GGUF/resolve/main/Ministral-3-14B-Instruct-2512-Q4_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/unsloth/Ministral-3-14B-Instruct-2512-GGUF/raw/main/Ministral-3-14B-Instruct-2512-Q4_K_M.gguf",
        pinned_sha256: "76ce697c065f2e40f1e8e958118b02cab38e2c10a6015f7d7908036a292dc8c8",
        size_bytes: 8_239_067_840,
        min_ram_gb: 16,
        context_window_tokens: 262_144,
        parameters: "13.5B dense",
        license: "Apache-2.0",
        description: "Mistral's European mid-size model with particularly strong French and 256K context.",
        disclaimer: None,
        superseded_by: None,
    },
    // Apache-2.0; 11.95B dense; Google QAT Q4_0; minimum 16 GB unified memory.
    ModelEntry {
        name: "Gemma 4 12B QAT Q4_0",
        filename: "gemma-4-12b-it-qat-q4_0.gguf",
        download_url: "https://huggingface.co/google/gemma-4-12B-it-qat-q4_0-gguf/resolve/main/gemma-4-12b-it-qat-q4_0.gguf",
        lfs_pointer_url: "https://huggingface.co/google/gemma-4-12B-it-qat-q4_0-gguf/raw/main/gemma-4-12b-it-qat-q4_0.gguf",
        pinned_sha256: "93567e57a8fe10b23569b9d9ec38cd005deedf71e29477c421a4b83f418a538b",
        size_bytes: 6_975_879_296,
        min_ram_gb: 16,
        context_window_tokens: 262_144,
        parameters: "12B dense",
        license: "Apache-2.0",
        description: "Mid-size dense Gemma 4 with quantization-aware weights and 256K context for 16 GB Macs.",
        disclaimer: None,
        superseded_by: None,
    },
    // Apache-2.0; ~9B dense; minimum 16 GB unified memory.
    ModelEntry {
        name: "Qwen 3.5 9B Q4_K_M",
        filename: "Qwen3.5-9B-Q4_K_M.gguf",
        download_url: "https://huggingface.co/unsloth/Qwen3.5-9B-GGUF/resolve/main/Qwen3.5-9B-Q4_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/unsloth/Qwen3.5-9B-GGUF/raw/main/Qwen3.5-9B-Q4_K_M.gguf",
        pinned_sha256: "03b74727a860a56338e042c4420bb3f04b2fec5734175f4cb9fa853daf52b7e8",
        size_bytes: 5_680_522_464,
        min_ram_gb: 16,
        context_window_tokens: 262_144,
        parameters: "9B dense",
        license: "Apache-2.0",
        description: "Replaces Qwen3 8B: same footprint class with 256K instead of 32K context.",
        disclaimer: None,
        superseded_by: None,
    },
    // Apache-2.0; 8.8B dense; IBM vendor GGUF; minimum 16 GB unified memory.
    ModelEntry {
        name: "Granite 4.2 8B Q4_K_M",
        filename: "granite-4.2-8b-Q4_K_M.gguf",
        download_url: "https://huggingface.co/ibm-granite/granite-4.2-8b-GGUF/resolve/main/granite-4.2-8b-Q4_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/ibm-granite/granite-4.2-8b-GGUF/raw/main/granite-4.2-8b-Q4_K_M.gguf",
        pinned_sha256: "16a9369d0805f80b7377d25d87f937a90c05dc04ad79173a52001e42c9aab311",
        size_bytes: 5_347_917_952,
        min_ram_gb: 16,
        context_window_tokens: 131_072,
        parameters: "8.8B dense",
        license: "Apache-2.0",
        description: "Compact IBM model with evaluated German and French and a low-effort reasoning mode.",
        disclaimer: None,
        superseded_by: None,
    },
    // Apache-2.0; 8B stored / 4.5B effective; Google QAT Q4_0; minimum 12 GB unified memory.
    ModelEntry {
        name: "Gemma 4 E4B QAT Q4_0",
        filename: "gemma-4-E4B_q4_0-it.gguf",
        download_url: "https://huggingface.co/google/gemma-4-E4B-it-qat-q4_0-gguf/resolve/main/gemma-4-E4B_q4_0-it.gguf",
        lfs_pointer_url: "https://huggingface.co/google/gemma-4-E4B-it-qat-q4_0-gguf/raw/main/gemma-4-E4B_q4_0-it.gguf",
        pinned_sha256: "676c35070db6dbe52f93e9c864ee0fba4eddea94b9c875d9cb10daff453fbaee",
        size_bytes: 5_154_941_280,
        min_ram_gb: 12,
        context_window_tokens: 131_072,
        parameters: "8B stored / 4.5B effective",
        license: "Apache-2.0",
        description: "Quantization-aware release of the on-device Gemma 4 tier with 128K context.",
        disclaimer: None,
        superseded_by: None,
    },
    // OpenMDW-1.1; Gemma 4 E4B fine-tune; minimum 12 GB unified memory.
    ModelEntry {
        name: "Nemotron 3.5 Bolt E4B UD-Q4_K_XL",
        filename: "NVIDIA-Nemotron-3.5-Bolt-E4B-UD-Q4_K_XL.gguf",
        download_url: "https://huggingface.co/unsloth/NVIDIA-Nemotron-3.5-Bolt-E4B-GGUF/resolve/main/NVIDIA-Nemotron-3.5-Bolt-E4B-UD-Q4_K_XL.gguf",
        lfs_pointer_url: "https://huggingface.co/unsloth/NVIDIA-Nemotron-3.5-Bolt-E4B-GGUF/raw/main/NVIDIA-Nemotron-3.5-Bolt-E4B-UD-Q4_K_XL.gguf",
        pinned_sha256: "84ff34f5834b929043f5ae74799e8d562d4056998f6a5923ceb44831e8ad84d6",
        size_bytes: 5_060_482_240,
        min_ram_gb: 12,
        context_window_tokens: 65_536,
        parameters: "8B stored / 4.5B effective",
        license: "OpenMDW-1.1",
        description: "NVIDIA's reasoning-tuned Gemma 4 E4B for small Macs; 64K context.",
        disclaimer: None,
        superseded_by: None,
    },
    // Apache-2.0; 5.1B stored / 2.3B effective; Google QAT Q4_0; minimum 8 GB unified memory.
    ModelEntry {
        name: "Gemma 4 E2B QAT Q4_0",
        filename: "gemma-4-E2B_q4_0-it.gguf",
        download_url: "https://huggingface.co/google/gemma-4-E2B-it-qat-q4_0-gguf/resolve/main/gemma-4-E2B_q4_0-it.gguf",
        lfs_pointer_url: "https://huggingface.co/google/gemma-4-E2B-it-qat-q4_0-gguf/raw/main/gemma-4-E2B_q4_0-it.gguf",
        pinned_sha256: "fa401b55b07ee70a54c6dae3903c783a6e65064312529ea57175cb5f8dec6634",
        size_bytes: 3_349_516_256,
        min_ram_gb: 8,
        context_window_tokens: 131_072,
        parameters: "5.1B stored / 2.3B effective",
        license: "Apache-2.0",
        description: "Quantization-aware release of the smallest Gemma 4 for memory-constrained Macs; 128K context.",
        disclaimer: None,
        superseded_by: None,
    },
    // Apache-2.0; 35B total / 3B active; minimum 32 GB unified memory.
    ModelEntry {
        name: "Qwen 3.6 35B-A3B MoE UD-Q4_K_M",
        filename: "Qwen3.6-35B-A3B-UD-Q4_K_M.gguf",
        download_url: "https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF/resolve/main/Qwen3.6-35B-A3B-UD-Q4_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/unsloth/Qwen3.6-35B-A3B-GGUF/raw/main/Qwen3.6-35B-A3B-UD-Q4_K_M.gguf",
        pinned_sha256: "ac0e2c1189e055faa36eff361580e79c5bd6f8e76bffb4ce547f167d53e31a61",
        size_bytes: 22_134_528_992,
        min_ram_gb: 32,
        context_window_tokens: 262_144,
        parameters: "35B total / 3B active",
        license: "Apache-2.0",
        description: "Flagship long-context MoE for report generation, German clinical text, RAG, and tool use.",
        disclaimer: None,
        superseded_by: None,
    },
    // Apache-2.0; 27B dense; minimum 24 GB unified memory.
    ModelEntry {
        name: "Qwen 3.6 27B Dense Q4_K_M",
        filename: "Qwen3.6-27B-Q4_K_M.gguf",
        download_url: "https://huggingface.co/unsloth/Qwen3.6-27B-GGUF/resolve/main/Qwen3.6-27B-Q4_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/unsloth/Qwen3.6-27B-GGUF/raw/main/Qwen3.6-27B-Q4_K_M.gguf",
        pinned_sha256: "5ed60d0af4650a854b1755bd392f9aef4872643dc25a254bc68043fa638392a0",
        size_bytes: 16_817_244_384,
        min_ram_gb: 24,
        context_window_tokens: 262_144,
        parameters: "27B dense",
        license: "Apache-2.0",
        description: "Dense Qwen 3.6 quality tier with strong multilingual reasoning and 256K-class context.",
        disclaimer: None,
        superseded_by: Some("Qwen3.8-27B-UD-Q4_K_M.gguf"),
    },
    // Apache-2.0; 25.2B total / 3.8B active; minimum 24 GB unified memory.
    ModelEntry {
        name: "Gemma 4 26B-A4B MoE Q4_0",
        filename: "gemma-4-26B-A4B-it-Q4_0.gguf",
        download_url: "https://huggingface.co/ggml-org/gemma-4-26B-A4B-it-GGUF/resolve/main/gemma-4-26B-A4B-it-Q4_0.gguf",
        lfs_pointer_url: "https://huggingface.co/ggml-org/gemma-4-26B-A4B-it-GGUF/raw/main/gemma-4-26B-A4B-it-Q4_0.gguf",
        pinned_sha256: "d208665ab1cd3a69f7a9a4bc59430e8448c8093d9b06334f566ac59d6d504a03",
        size_bytes: 14_618_145_824,
        min_ram_gb: 24,
        context_window_tokens: 262_144,
        parameters: "25.2B total / 3.8B active",
        license: "Apache-2.0",
        description: "Memory-efficient multilingual MoE alternative with 256K context and over 140 languages.",
        disclaimer: None,
        superseded_by: Some("gemma-4-26B_q4_0-it.gguf"),
    },
    // Apache-2.0; 21B total / 3.6B active; minimum 18 GB unified memory.
    ModelEntry {
        name: "gpt-oss-20b MXFP4",
        filename: "gpt-oss-20b-MXFP4.gguf",
        download_url: "https://huggingface.co/ggml-org/gpt-oss-20b-GGUF/resolve/main/gpt-oss-20b-MXFP4.gguf",
        lfs_pointer_url: "https://huggingface.co/ggml-org/gpt-oss-20b-GGUF/raw/main/gpt-oss-20b-MXFP4.gguf",
        pinned_sha256: "27cd6c432c7672cb812a92f611cf3ba7bbc35928262bb1e1253ff4ee6ae35901",
        size_bytes: 12_109_566_624,
        min_ram_gb: 18,
        context_window_tokens: 131_072,
        parameters: "21B total / 3.6B active",
        license: "Apache-2.0",
        description: "Compact reasoning and tool-use MoE; the official MXFP4 weights are designed for local use.",
        disclaimer: None,
        superseded_by: None,
    },
    // Apache-2.0; 8B stored / 4.5B effective; minimum 12 GB unified memory.
    ModelEntry {
        name: "Gemma 4 E4B Q4_0",
        filename: "gemma-4-E4B-it-Q4_0.gguf",
        download_url: "https://huggingface.co/ggml-org/gemma-4-E4B-it-GGUF/resolve/main/gemma-4-E4B-it-Q4_0.gguf",
        lfs_pointer_url: "https://huggingface.co/ggml-org/gemma-4-E4B-it-GGUF/raw/main/gemma-4-E4B-it-Q4_0.gguf",
        pinned_sha256: "a555b900214b477d8880e7832e0b8925e139b0159640036b09fe472b6f2097f2",
        size_bytes: 4_590_807_392,
        min_ram_gb: 12,
        context_window_tokens: 131_072,
        parameters: "8B stored / 4.5B effective",
        license: "Apache-2.0",
        description: "New on-device Gemma tier with 128K context and a low 4.3 GiB model footprint.",
        disclaimer: None,
        superseded_by: Some("gemma-4-E4B_q4_0-it.gguf"),
    },
    // Apache-2.0; 5.1B stored / 2.3B effective; minimum 8 GB unified memory.
    ModelEntry {
        name: "Gemma 4 E2B Q4_0",
        filename: "gemma-4-E2B-it-Q4_0.gguf",
        download_url: "https://huggingface.co/ggml-org/gemma-4-E2B-it-GGUF/resolve/main/gemma-4-E2B-it-Q4_0.gguf",
        lfs_pointer_url: "https://huggingface.co/ggml-org/gemma-4-E2B-it-GGUF/raw/main/gemma-4-E2B-it-Q4_0.gguf",
        pinned_sha256: "8e30dff3ac4c8434c49a7036fa15564bdbb6044e42bf04550bf1a096ad7e6a52",
        size_bytes: 2_841_481_184,
        min_ram_gb: 8,
        context_window_tokens: 131_072,
        parameters: "5.1B stored / 2.3B effective",
        license: "Apache-2.0",
        description: "Newest ultra-small multilingual option with 128K context for memory-constrained Macs.",
        disclaimer: None,
        superseded_by: Some("gemma-4-E2B_q4_0-it.gguf"),
    },
    // Apache-2.0; 8B dense; minimum 16 GB unified memory.
    ModelEntry {
        name: "Qwen3 8B Q4_K_M",
        filename: "Qwen3-8B-Q4_K_M.gguf",
        download_url: "https://huggingface.co/unsloth/Qwen3-8B-GGUF/resolve/main/Qwen3-8B-Q4_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/unsloth/Qwen3-8B-GGUF/raw/main/Qwen3-8B-Q4_K_M.gguf",
        pinned_sha256: "120307ba529eb2439d6c430d94104dabd578497bc7bfe7e322b5d9933b449bd4",
        size_bytes: 5_027_784_512,
        min_ram_gb: 16,
        context_window_tokens: 32_768,
        parameters: "8B dense",
        license: "Apache-2.0",
        description: "Established multilingual dense fallback with good quality at a 4.7 GiB footprint.",
        disclaimer: None,
        superseded_by: Some("Qwen3.5-9B-Q4_K_M.gguf"),
    },
    // MIT; 3.8B dense; minimum 8 GB unified memory.
    ModelEntry {
        name: "Phi-4 Mini Q4_K_M",
        filename: "Phi-4-mini-instruct-Q4_K_M.gguf",
        download_url: "https://huggingface.co/unsloth/Phi-4-mini-instruct-GGUF/resolve/main/Phi-4-mini-instruct-Q4_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/unsloth/Phi-4-mini-instruct-GGUF/raw/main/Phi-4-mini-instruct-Q4_K_M.gguf",
        pinned_sha256: "88c00229914083cd112853aab84ed51b87bdf6b9ce42f532d8c85c7c63b1730a",
        size_bytes: 2_491_874_272,
        min_ram_gb: 8,
        context_window_tokens: 131_072,
        parameters: "3.8B dense",
        license: "MIT",
        description: "Fast compact alternative for English-leaning workloads and limited memory.",
        disclaimer: None,
        superseded_by: None,
    },
    // Google Health AI terms; 4B dense; minimum 8 GB unified memory.
    ModelEntry {
        name: "MedGemma 1.5 4B IT Q4_K_M",
        filename: "medgemma-1.5-4b-it-Q4_K_M.gguf",
        download_url: "https://huggingface.co/unsloth/medgemma-1.5-4b-it-GGUF/resolve/main/medgemma-1.5-4b-it-Q4_K_M.gguf",
        lfs_pointer_url: "https://huggingface.co/unsloth/medgemma-1.5-4b-it-GGUF/raw/main/medgemma-1.5-4b-it-Q4_K_M.gguf",
        pinned_sha256: "b31becdf4f39561800505514cce67681604fe449d04dd35c8c92fd7848c6d7bd",
        size_bytes: 2_489_894_976,
        min_ram_gb: 8,
        context_window_tokens: 131_072,
        parameters: "4B dense",
        license: "Health AI Developer Foundations terms",
        description: "Medical-domain instruction model for clinician-reviewed on-device drafting.",
        disclaimer: Some("Use for clinician-reviewed drafting only. This model is not validated for diagnosis or treatment decisions."),
        superseded_by: None,
    },
];

pub(crate) fn find_model(filename: &str) -> Option<&'static ModelEntry> {
    MODELS.iter().find(|m| m.filename == filename)
}

/// CRIT-3: Fetch the expected SHA-256 digest from a HuggingFace LFS pointer file.
///
/// Git LFS pointer format:
/// ```
/// version https://git-lfs.github.com/spec/v1
/// oid sha256:<64-char-hex>
/// size <bytes>
/// ```
async fn fetch_lfs_sha256(client: &reqwest::Client, pointer_url: &str) -> Result<String, AppError> {
    let response = client
        .get(pointer_url)
        .send()
        .await
        .map_err(|e| AppError::Llm(format!("Failed to fetch LFS pointer: {e}")))?;

    // Reject unexpectedly large responses before reading the body.
    if let Some(len) = response.content_length() {
        if len > MAX_POINTER_BYTES as u64 {
            return Err(AppError::Validation(format!(
                "LFS pointer response too large ({len} bytes); expected ~130 bytes"
            )));
        }
    }

    let text = response
        .text()
        .await
        .map_err(|e| AppError::Llm(format!("Failed to read LFS pointer body: {e}")))?;

    // Guard against no Content-Length but still oversized body.
    parse_lfs_pointer_text(&text)
}

/// Parse an LFS pointer text body and return the lowercase hex SHA-256 digest.
/// Extracted for unit testability — the HTTP fetching stays in `fetch_lfs_sha256`.
fn parse_lfs_pointer_text(text: &str) -> Result<String, AppError> {
    if text.len() > MAX_POINTER_BYTES {
        return Err(AppError::Validation(
            "LFS pointer body too large".to_string(),
        ));
    }

    // Parse the "oid sha256:<hex>" line.
    for line in text.lines() {
        if let Some(hex) = line.strip_prefix("oid sha256:") {
            let hex = hex.trim();
            if hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Ok(hex.to_lowercase());
            }
            return Err(AppError::Validation(format!(
                "LFS pointer contained malformed sha256 oid: '{hex}'"
            )));
        }
    }

    Err(AppError::Validation(
        "LFS pointer did not contain an 'oid sha256:' line".to_string(),
    ))
}

/// CRIT-4: Map a GGUF filename to its HuggingFace (Unsloth mirror) download URL.
/// Only explicitly whitelisted filenames are allowed — the fallback arm has been
/// removed to prevent SSRF and arbitrary URL construction.
pub fn model_url(filename: &str) -> Result<String, AppError> {
    find_model(filename)
        .map(|m| m.download_url.to_string())
        .ok_or_else(|| {
            AppError::Validation(format!(
                "Unknown model filename '{}'. Only explicitly whitelisted models may be downloaded.",
                filename
            ))
        })
}

/// Download a model file, resuming from where it left off if a partial file exists.
/// Emits `"model-download-progress"` (f64 0.0–1.0) and `"model-download-done"` Tauri events.
///
/// Returns the verified SHA-256 hex digest of the downloaded file.
///
/// CRIT-3: Fetches the expected SHA-256 from HuggingFace's LFS pointer before downloading,
///         then verifies the completed file against it.
///         The runtime-fetched LFS pointer digest is asserted against the compile-time
///         pin before the download begins.
/// HIGH-2: Aborts download if total bytes exceed MAX_DOWNLOAD_BYTES.
pub async fn download_model_with_progress(
    app: &AppHandle,
    url: &str,
    dest_path: &Path,
    filename: &str,
) -> Result<String, AppError> {
    // CRIT-3: Every filename that passes model_url() is guaranteed to have an
    // lfs_pointer_url entry in MODELS, so the error branch below is
    // unreachable in normal operation.
    let model = find_model(filename).ok_or_else(|| {
        AppError::Validation(format!(
            "No model entry for '{}' — integrity check cannot proceed",
            filename
        ))
    })?;
    download_pinned_file(
        app,
        PinnedDownload {
            url,
            lfs_pointer_url: model.lfs_pointer_url,
            pinned_sha256: model.pinned_sha256,
            filename,
            progress_event: "model-download-progress",
            done_event: "model-download-done",
        },
        dest_path,
    )
    .await
}

/// A whitelisted file with a compile-time SHA-256 pin.
pub(crate) struct PinnedDownload<'a> {
    pub url: &'a str,
    pub lfs_pointer_url: &'a str,
    pub pinned_sha256: &'a str,
    pub filename: &'a str,
    /// Tauri event receiving progress as f64 0.0–1.0.
    pub progress_event: &'a str,
    /// Tauri event emitted once the file is verified.
    pub done_event: &'a str,
}

/// Download a pinned file with resume, size cap and SHA-256 verification
/// (CRIT-3 / HIGH-2). Shared by the LLM and dictation model catalogs.
pub(crate) async fn download_pinned_file(
    app: &AppHandle,
    spec: PinnedDownload<'_>,
    dest_path: &Path,
) -> Result<String, AppError> {
    let PinnedDownload {
        url,
        lfs_pointer_url,
        pinned_sha256,
        filename,
        progress_event,
        done_event,
    } = spec;
    let client = reqwest::Client::new();

    // CRIT-3: Fetch expected SHA-256 from HuggingFace LFS pointer before the download begins.
    // This fails fast (before any large transfer) if the pointer is unavailable or malformed.
    log::info!("Fetching LFS pointer for '{}'…", filename);
    let expected_hex = fetch_lfs_sha256(&client, lfs_pointer_url).await?;

    // CRIT-3: Assert that the mutable LFS pointer still identifies the approved blob.
    if expected_hex != pinned_sha256 {
        return Err(AppError::Validation(format!(
            "LFS pointer SHA-256 for '{}' does not match pinned value: \
             expected pinned={}, fetched={}. \
             Possible supply-chain tampering — download aborted.",
            filename, pinned_sha256, expected_hex
        )));
    }
    log::info!("Pinned SHA-256 verified for '{}'.", filename);

    // Check for an existing partial download.
    let existing_size = if dest_path.exists() {
        tokio::fs::metadata(dest_path).await?.len()
    } else {
        0
    };

    let mut request = client.get(url);
    if existing_size > 0 {
        request = request.header(RANGE, format!("bytes={}-", existing_size));
    }

    let response = request
        .send()
        .await
        .map_err(|e| AppError::Llm(format!("Download request failed: {e}")))?;

    // Total size: from Content-Range when resuming, from Content-Length otherwise.
    let total_size = if existing_size > 0 {
        response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split('/').next_back())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    } else {
        response.content_length().unwrap_or(0)
    };

    // HIGH-2: Reject if the declared size already exceeds our cap
    if total_size > MAX_DOWNLOAD_BYTES {
        return Err(AppError::Validation(format!(
            "Declared content size {} bytes exceeds maximum allowed {} bytes",
            total_size, MAX_DOWNLOAD_BYTES
        )));
    }

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(existing_size > 0)
        .write(true)
        .open(dest_path)
        .await?;

    let mut downloaded = existing_size;
    let mut stream = response.bytes_stream();

    // CRIT-3: Hash context for the *entire* file (existing bytes + new bytes).
    // When resuming, re-hash already-downloaded bytes from disk first.
    let mut sha256 = DigestContext::new(&SHA256);
    if existing_size > 0 {
        let existing_bytes = tokio::fs::read(dest_path).await.map_err(|e| {
            AppError::Llm(format!("Failed to read partial download for hashing: {e}"))
        })?;
        sha256.update(&existing_bytes);
    }

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Llm(format!("Stream error: {e}")))?;

        // HIGH-2: Hard cap — abort if we're receiving more data than expected
        downloaded += chunk.len() as u64;
        if downloaded > MAX_DOWNLOAD_BYTES {
            let _ = tokio::fs::remove_file(dest_path).await;
            return Err(AppError::Validation(format!(
                "Download exceeded maximum size cap of {} bytes — aborting",
                MAX_DOWNLOAD_BYTES
            )));
        }

        // CRIT-3: Feed chunk into the running hash before writing
        sha256.update(&chunk);

        file.write_all(&chunk).await?;

        if total_size > 0 {
            let progress = downloaded as f64 / total_size as f64;
            let _ = app.emit(progress_event, progress);
        }
    }

    // Flush and close the file before verifying
    file.flush().await?;
    drop(file);

    // CRIT-3: Verify SHA-256 digest against the value fetched from the LFS pointer
    let digest = sha256.finish();
    let computed_hex = hex::encode(digest.as_ref());

    if computed_hex != expected_hex {
        let _ = tokio::fs::remove_file(dest_path).await;
        return Err(AppError::Validation(format!(
            "SHA-256 mismatch for '{}': expected {}, got {}. \
             File removed — possible MITM or corrupted download.",
            filename, expected_hex, computed_hex
        )));
    }
    log::info!("SHA-256 verified for '{}': {}", filename, computed_hex);

    let _ = app.emit(done_event, ());
    Ok(computed_hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- model_url whitelist ----

    #[test]
    fn test_model_url_known_filenames() {
        for entry in MODELS {
            let url = model_url(entry.filename).unwrap();
            assert!(
                url.ends_with(entry.filename),
                "URL for '{}' should end with the filename, got '{}'",
                entry.filename,
                url
            );
            assert!(
                url.starts_with("https://huggingface.co/"),
                "URL should be HF: {}",
                url
            );
        }
    }

    #[test]
    fn test_model_url_unknown_filename() {
        let result = model_url("evil-model.gguf");
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    /// Ensure superseded or placeholder entries cannot be downloaded.
    #[test]
    fn test_placeholder_models_are_not_whitelisted() {
        for filename in [
            "Qwen3-30B-A3B-Q4_K_M.gguf",
            "Qwen3.6-30B-A3B-Q4_K_M.gguf",
            "gemma-4-26B-A4B-it-Q4_K_M.gguf",
            "gpt-oss-20B-Q4_K_M.gguf",
        ] {
            assert!(matches!(model_url(filename), Err(AppError::Validation(_))));
        }
    }

    /// `find_model` must return an entry for every new 2026-refresh filename.
    #[test]
    fn test_find_model_new_entries() {
        for filename in [
            "Qwen3.6-35B-A3B-UD-Q4_K_M.gguf",
            "Qwen3.6-27B-Q4_K_M.gguf",
            "gemma-4-26B-A4B-it-Q4_0.gguf",
            "gemma-4-E4B-it-Q4_0.gguf",
            "gemma-4-E2B-it-Q4_0.gguf",
            "gpt-oss-20b-MXFP4.gguf",
        ] {
            assert!(find_model(filename).is_some(), "missing model: {filename}");
        }
    }

    /// Every 2026-11 refresh entry is whitelisted, current, and not superseded.
    #[test]
    fn test_find_model_2026_11_entries() {
        for filename in [
            "gemma-4-31B_q4_0-it.gguf",
            "granite-4.2-30b-Q4_K_M.gguf",
            "NVIDIA-Nemotron-3.5-Lightning-30B-A3B-Q4_0.gguf",
            "Qwen3.8-27B-UD-Q4_K_M.gguf",
            "gemma-4-26B_q4_0-it.gguf",
            "Ministral-3-14B-Instruct-2512-Q4_K_M.gguf",
            "gemma-4-12b-it-qat-q4_0.gguf",
            "Qwen3.5-9B-Q4_K_M.gguf",
            "granite-4.2-8b-Q4_K_M.gguf",
            "gemma-4-E4B_q4_0-it.gguf",
            "NVIDIA-Nemotron-3.5-Bolt-E4B-UD-Q4_K_XL.gguf",
            "gemma-4-E2B_q4_0-it.gguf",
        ] {
            let entry = find_model(filename).unwrap_or_else(|| panic!("missing model: {filename}"));
            assert!(entry.superseded_by.is_none(), "{filename} is superseded");
        }
    }

    /// A superseded entry must point at a current entry, never a chain or a gap.
    #[test]
    fn test_superseded_entries_point_at_current_models() {
        for entry in MODELS.iter().filter(|m| m.superseded_by.is_some()) {
            let successor = find_model(entry.superseded_by.unwrap())
                .unwrap_or_else(|| panic!("{} points at an unknown model", entry.filename));
            assert!(successor.superseded_by.is_none());
            assert!(successor.min_ram_gb <= entry.min_ram_gb.max(16));
        }
    }

    #[test]
    fn test_catalog_has_no_duplicate_filenames_or_hashes() {
        let mut names: Vec<_> = MODELS.iter().map(|m| m.filename).collect();
        let mut hashes: Vec<_> = MODELS.iter().map(|m| m.pinned_sha256).collect();
        names.sort_unstable();
        hashes.sort_unstable();
        let total = MODELS.len();
        names.dedup();
        hashes.dedup();
        assert_eq!(names.len(), total);
        assert_eq!(hashes.len(), total);
    }

    /// Both URL fields for every model must point at the same filename blob
    /// (resolve/main vs raw/main, not a different quant or repo).
    #[test]
    fn test_model_url_pairs_are_consistent() {
        for entry in MODELS {
            let fname = entry.filename;
            assert!(
                entry.download_url.contains(fname),
                "download_url for '{}' does not contain the filename",
                fname
            );
            assert!(
                entry.lfs_pointer_url.contains(fname),
                "lfs_pointer_url for '{}' does not contain the filename",
                fname
            );
            assert!(
                entry.download_url.contains("resolve/main"),
                "download_url for '{}' should use resolve/main, got '{}'",
                fname,
                entry.download_url
            );
            assert!(
                entry.lfs_pointer_url.contains("raw/main"),
                "lfs_pointer_url for '{}' should use raw/main, got '{}'",
                fname,
                entry.lfs_pointer_url
            );
        }
    }

    /// All models must have a pinned SHA-256 (CRIT-3), and it must be exactly 64 lowercase hex characters.
    #[test]
    fn test_all_models_have_valid_pinned_sha256() {
        for entry in MODELS {
            let pinned = entry.pinned_sha256;

            assert_eq!(
                pinned.len(),
                64,
                "pinned_sha256 for '{}' must be 64 hex chars",
                entry.filename
            );
            assert!(
                pinned.bytes().all(|b| b.is_ascii_hexdigit()),
                "pinned_sha256 for '{}' contains non-hex characters",
                entry.filename
            );
            assert_eq!(
                pinned,
                pinned.to_lowercase(),
                "pinned_sha256 for '{}' must be lowercase",
                entry.filename
            );
        }
    }

    #[test]
    fn test_all_models_have_complete_picker_metadata() {
        for entry in MODELS {
            assert!(entry.size_bytes > 0, "missing size for {}", entry.filename);
            assert!(
                entry.min_ram_gb >= 8,
                "invalid RAM tier for {}",
                entry.filename
            );
            assert!(
                entry.context_window_tokens >= 32_768,
                "invalid context window for {}",
                entry.filename
            );
            assert!(!entry.parameters.is_empty());
            assert!(!entry.license.is_empty());
        }
    }

    // ---- parse_lfs_pointer_text ----

    fn valid_hex() -> String {
        "a".repeat(64)
    }

    #[test]
    fn test_parse_lfs_pointer_valid() {
        let hex = valid_hex();
        let text = format!(
            "version https://git-lfs.github.com/spec/v1\noid sha256:{}\nsize 1234567890\n",
            hex
        );
        let result = parse_lfs_pointer_text(&text).unwrap();
        assert_eq!(result, hex);
    }

    #[test]
    fn test_parse_lfs_pointer_extra_whitespace() {
        let hex = valid_hex();
        // Simulate Windows line endings — trailing \r on the hex line
        let text = format!(
            "version https://git-lfs.github.com/spec/v1\r\noid sha256:{}\r\nsize 1234\r\n",
            hex
        );
        let result = parse_lfs_pointer_text(&text).unwrap();
        assert_eq!(result, hex);
    }

    #[test]
    fn test_parse_lfs_pointer_missing_oid_line() {
        let text = "version https://git-lfs.github.com/spec/v1\nsize 1234\n";
        let result = parse_lfs_pointer_text(text);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn test_parse_lfs_pointer_malformed_hex_short() {
        // 63 chars instead of 64
        let hex = "a".repeat(63);
        let text = format!("oid sha256:{}\nsize 1234\n", hex);
        let result = parse_lfs_pointer_text(&text);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn test_parse_lfs_pointer_non_hex_chars() {
        // 64 chars but contains non-hex character 'Z'
        let hex = format!("{}Z{}", "a".repeat(32), "a".repeat(31));
        let text = format!("oid sha256:{}\nsize 1234\n", hex);
        let result = parse_lfs_pointer_text(&text);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn test_parse_lfs_pointer_body_too_large() {
        // Build a body that exceeds MAX_POINTER_BYTES (4096)
        let large_text = "x".repeat(MAX_POINTER_BYTES + 1);
        let result = parse_lfs_pointer_text(&large_text);
        assert!(matches!(result, Err(AppError::Validation(_))));
    }

    #[test]
    fn test_parse_lfs_pointer_hex_returned_lowercase() {
        // Even if the hex were uppercase (non-standard), trim/to_lowercase is applied
        let hex = "A".repeat(64);
        let text = format!("oid sha256:{}\nsize 1234\n", hex);
        let result = parse_lfs_pointer_text(&text).unwrap();
        assert_eq!(result, "a".repeat(64));
    }
}
