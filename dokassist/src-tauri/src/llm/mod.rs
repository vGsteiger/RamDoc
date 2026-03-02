mod engine;
mod prompts;
mod extract;
mod report;
pub mod download;

pub use engine::{LlmEngine, ModelChoice, EngineStatus};
pub use extract::{extract_metadata, extract_metadata_with_prompt, FileMetadata};
pub use prompts::{ReportType, SYSTEM_PROMPT_DE};
pub use report::{generate_report_streaming, generate_report_streaming_with_prompt};
