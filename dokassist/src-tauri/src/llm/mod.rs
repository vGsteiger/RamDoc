pub mod download;
pub mod embed;
mod engine;
mod extract;
mod prompts;
mod report;
pub mod sanitize;

pub use engine::{EngineStatus, LlmEngine, ModelChoice};
pub use extract::{extract_metadata_with_prompt, FileMetadata};
pub use prompts::{ReportType, SYSTEM_PROMPT_DE};
pub use report::generate_report_streaming_with_prompt;
