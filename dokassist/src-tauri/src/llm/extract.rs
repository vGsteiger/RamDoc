use serde::{Deserialize, Serialize};
use crate::error::AppError;
use super::{engine::LlmEngine, prompts};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub document_type: Option<String>,
    pub date: Option<String>,
    pub author: Option<String>,
    pub diagnoses: Vec<String>,
    pub medications: Vec<String>,
    pub summary: Option<String>,
}

/// Extract structured metadata using the built-in system prompt.
pub fn extract_metadata(engine: &LlmEngine, document_text: &str) -> Result<FileMetadata, AppError> {
    extract_metadata_with_prompt(engine, document_text, prompts::SYSTEM_PROMPT_DE)
}

/// Extract structured metadata using a caller-supplied system prompt.
pub fn extract_metadata_with_prompt(
    engine: &LlmEngine,
    document_text: &str,
    system_prompt: &str,
) -> Result<FileMetadata, AppError> {
    let user_message = prompts::metadata_extraction_prompt(document_text);
    let response = engine.generate(system_prompt, &user_message, 512, 0.1)?;
    let json_str = strip_markdown_fences(&response);
    serde_json::from_str(json_str)
        .map_err(|e| AppError::Llm(format!("Failed to parse metadata JSON: {e}")))
}

fn strip_markdown_fences(s: &str) -> &str {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix("```json") {
        if let Some(inner) = inner.strip_suffix("```") {
            return inner.trim();
        }
    }
    if let Some(inner) = s.strip_prefix("```") {
        if let Some(inner) = inner.strip_suffix("```") {
            return inner.trim();
        }
    }
    s
}
