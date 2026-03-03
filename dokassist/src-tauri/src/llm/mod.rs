pub mod download;
mod engine;
mod extract;
pub mod prompts;
mod report;
pub mod chunk;

pub use engine::{EngineStatus, LlmEngine, ModelChoice};
pub use extract::{extract_metadata_with_prompt, FileMetadata};
pub use prompts::{ReportType, SYSTEM_PROMPT_DE};
pub use report::generate_report_streaming_with_prompt;
pub use chunk::{chunk_document, ChunkConfig, TextChunk};
