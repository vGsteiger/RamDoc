pub mod agent;
#[cfg(any(test, feature = "benchmark-harness"))]
#[doc(hidden)]
pub mod benchmark_harness;
pub mod chunk;
pub mod context_cache;
pub mod download;
pub mod embed;
pub mod engine;
pub mod evidence;
mod extract;
pub mod harness;
pub mod inference;
pub mod memory_governor;
#[cfg(test)]
mod profile_benchmark;
mod prompts;
pub mod quantization;
mod report;
pub mod sanitize;
pub mod thinking;
pub mod tools;
pub mod utf8;

pub use engine::{EngineStatus, LlmEngine, ModelChoice};
pub use extract::{extract_metadata_with_prompt, FileMetadata};
pub use prompts::{LetterType, ReportType, SYSTEM_PROMPT_DE, SYSTEM_PROMPT_FR};
pub use report::{
    generate_evidence_answer_streaming, generate_letter_streaming_with_prompt,
    generate_report_streaming_with_prompt, generate_session_summary_streaming_with_prompt,
    improve_text_streaming_with_prompt,
};
pub use thinking::ThinkingEffort;
