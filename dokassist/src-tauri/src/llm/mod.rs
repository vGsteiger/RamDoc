pub mod agent;
pub mod chunk;
pub mod download;
pub mod embed;
pub mod engine;
mod extract;
mod prompts;
mod report;
pub mod sanitize;
pub mod tools;

pub use engine::{EngineStatus, LlmEngine, ModelChoice};
pub use extract::{extract_metadata_with_prompt, FileMetadata};
pub use prompts::{LetterType, ReportType, SYSTEM_PROMPT_DE, SYSTEM_PROMPT_FR, letter_generation_prompt};
pub use report::{
    generate_letter_streaming_with_prompt, generate_report_streaming_with_prompt,
    generate_session_summary_streaming_with_prompt, improve_text_streaming_with_prompt,
};
