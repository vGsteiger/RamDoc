//! Document chunking for RAG (Retrieval-Augmented Generation)
//!
//! Splits documents into overlapping chunks of approximately 200 words each
//! to enable fine-grained semantic search and context retrieval while staying
//! within model context limits.

use crate::error::AppError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Target words per chunk (~200-250 tokens for most models)
/// Reduced from 500 to prevent context overflow when multiple chunks are retrieved
const TARGET_WORDS_PER_CHUNK: usize = 200;

/// Overlap between consecutive chunks (words) - 10% of chunk size
const CHUNK_OVERLAP_WORDS: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub id: String,
    pub file_id: Option<String>,
    pub literature_id: Option<String>,
    pub chunk_index: i32,
    pub content: String,
    pub word_count: i32,
    pub created_at: String,
}

/// Configuration for chunking strategy
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    pub target_words: usize,
    pub overlap_words: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            target_words: TARGET_WORDS_PER_CHUNK,
            overlap_words: CHUNK_OVERLAP_WORDS,
        }
    }
}

/// Split text into overlapping chunks by word boundaries
pub fn chunk_text(text: &str, config: &ChunkConfig) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.len() <= config.target_words {
        // Document is small enough to be a single chunk
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut start_idx = 0;

    while start_idx < words.len() {
        let end_idx = (start_idx + config.target_words).min(words.len());
        let chunk_words = &words[start_idx..end_idx];
        let chunk_text = chunk_words.join(" ");

        chunks.push(chunk_text);

        // Move forward by (target_words - overlap_words) to create overlap
        start_idx += config.target_words.saturating_sub(config.overlap_words);

        // Prevent infinite loop on last small chunk
        if end_idx >= words.len() {
            break;
        }
    }

    chunks
}

/// Create document chunks for a patient file
pub fn create_file_chunks(
    conn: &Connection,
    file_id: &str,
    text: &str,
    config: &ChunkConfig,
) -> Result<Vec<DocumentChunk>, AppError> {
    // First, delete any existing chunks for this file
    conn.execute("DELETE FROM document_chunks WHERE file_id = ?1", [file_id])?;

    let chunks_text = chunk_text(text, config);
    let mut created_chunks = Vec::with_capacity(chunks_text.len());

    for (idx, chunk_content) in chunks_text.iter().enumerate() {
        let chunk_id = uuid::Uuid::now_v7().to_string();
        let word_count = chunk_content.split_whitespace().count() as i32;
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO document_chunks (id, file_id, literature_id, chunk_index, content, word_count, created_at)
            VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)
            "#,
            rusqlite::params![chunk_id, file_id, idx as i32, chunk_content, word_count, now],
        )?;

        created_chunks.push(DocumentChunk {
            id: chunk_id,
            file_id: Some(file_id.to_string()),
            literature_id: None,
            chunk_index: idx as i32,
            content: chunk_content.clone(),
            word_count,
            created_at: now,
        });
    }

    Ok(created_chunks)
}

/// Create document chunks for a literature document
pub fn create_literature_chunks(
    conn: &Connection,
    literature_id: &str,
    text: &str,
    config: &ChunkConfig,
) -> Result<Vec<DocumentChunk>, AppError> {
    // First, delete any existing chunks for this literature
    conn.execute(
        "DELETE FROM document_chunks WHERE literature_id = ?1",
        [literature_id],
    )?;

    let chunks_text = chunk_text(text, config);
    let mut created_chunks = Vec::with_capacity(chunks_text.len());

    for (idx, chunk_content) in chunks_text.iter().enumerate() {
        let chunk_id = uuid::Uuid::now_v7().to_string();
        let word_count = chunk_content.split_whitespace().count() as i32;
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO document_chunks (id, file_id, literature_id, chunk_index, content, word_count, created_at)
            VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6)
            "#,
            rusqlite::params![
                chunk_id,
                literature_id,
                idx as i32,
                chunk_content,
                word_count,
                now
            ],
        )?;

        created_chunks.push(DocumentChunk {
            id: chunk_id,
            file_id: None,
            literature_id: Some(literature_id.to_string()),
            chunk_index: idx as i32,
            content: chunk_content.clone(),
            word_count,
            created_at: now,
        });
    }

    Ok(created_chunks)
}

/// Get all chunks for a file
pub fn get_file_chunks(conn: &Connection, file_id: &str) -> Result<Vec<DocumentChunk>, AppError> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, file_id, literature_id, chunk_index, content, word_count, created_at
        FROM document_chunks
        WHERE file_id = ?1
        ORDER BY chunk_index ASC
        "#,
    )?;

    let chunks = stmt
        .query_map([file_id], |row| {
            Ok(DocumentChunk {
                id: row.get(0)?,
                file_id: row.get(1)?,
                literature_id: row.get(2)?,
                chunk_index: row.get(3)?,
                content: row.get(4)?,
                word_count: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(chunks)
}

/// Get all chunks for a literature document
pub fn get_literature_chunks(
    conn: &Connection,
    literature_id: &str,
) -> Result<Vec<DocumentChunk>, AppError> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, file_id, literature_id, chunk_index, content, word_count, created_at
        FROM document_chunks
        WHERE literature_id = ?1
        ORDER BY chunk_index ASC
        "#,
    )?;

    let chunks = stmt
        .query_map([literature_id], |row| {
            Ok(DocumentChunk {
                id: row.get(0)?,
                file_id: row.get(1)?,
                literature_id: row.get(2)?,
                chunk_index: row.get(3)?,
                content: row.get(4)?,
                word_count: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;
    use crate::models::literature::{create_literature, CreateLiterature};

    fn setup_test_db() -> (tempfile::TempDir, crate::database::DbPool) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let key = [42u8; 32];
        let pool = database::init_db(&db_path, &key).unwrap();
        (temp_dir, pool)
    }

    #[test]
    fn test_chunk_text_small_document() {
        let config = ChunkConfig::default();
        let text = "This is a small document with only a few words.";
        let chunks = chunk_text(text, &config);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn test_chunk_text_large_document() {
        let config = ChunkConfig {
            target_words: 10,
            overlap_words: 2,
        };

        // Create a text with 25 words
        let words: Vec<String> = (0..25).map(|i| format!("word{}", i)).collect();
        let text = words.join(" ");

        let chunks = chunk_text(&text, &config);

        // With 25 words, target 10, overlap 2:
        // Chunk 0: words 0-9 (10 words)
        // Chunk 1: words 8-17 (10 words, starts at 10-2=8)
        // Chunk 2: words 16-24 (9 words, starts at 18-2=16)
        assert_eq!(chunks.len(), 3);

        // Verify overlap
        assert!(chunks[0].contains("word8") && chunks[0].contains("word9"));
        assert!(chunks[1].contains("word8") && chunks[1].contains("word9"));
    }

    #[test]
    fn test_create_and_get_literature_chunks() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        // Create a literature document
        let lit = create_literature(
            &conn,
            CreateLiterature {
                filename: "test.pdf".to_string(),
                vault_path: "vault/test.enc".to_string(),
                mime_type: "application/pdf".to_string(),
                size_bytes: 1000,
                description: None,
            },
        )
        .unwrap();

        // Create text with enough words to generate multiple chunks
        let words: Vec<String> = (0..600).map(|i| format!("word{}", i)).collect();
        let text = words.join(" ");

        let config = ChunkConfig::default();
        let chunks = create_literature_chunks(&conn, &lit.id, &text, &config).unwrap();

        // Should create 4 chunks (600 words with 200-word target and 20-word overlap)
        // Chunk 0: words 0-199 (200 words)
        // Chunk 1: words 180-379 (200 words, starts at 200-20=180)
        // Chunk 2: words 360-559 (200 words, starts at 380-20=360)
        // Chunk 3: words 540-599 (60 words, starts at 560-20=540)
        assert!(chunks.len() >= 4);

        // Verify chunks are stored in database
        let fetched_chunks = get_literature_chunks(&conn, &lit.id).unwrap();
        assert_eq!(fetched_chunks.len(), chunks.len());

        // Verify ordering
        for (idx, chunk) in fetched_chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, idx as i32);
            assert_eq!(chunk.literature_id, Some(lit.id.clone()));
            assert_eq!(chunk.file_id, None);
        }
    }

    #[test]
    fn test_recreate_chunks_deletes_old_ones() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        let lit = create_literature(
            &conn,
            CreateLiterature {
                filename: "test.pdf".to_string(),
                vault_path: "vault/test.enc".to_string(),
                mime_type: "application/pdf".to_string(),
                size_bytes: 1000,
                description: None,
            },
        )
        .unwrap();

        let config = ChunkConfig::default();

        // Create initial chunks
        let text1 = "First version of the document with some content.";
        let chunks1 = create_literature_chunks(&conn, &lit.id, text1, &config).unwrap();
        assert_eq!(chunks1.len(), 1);

        // Create new chunks (should replace old ones)
        let words: Vec<String> = (0..600).map(|i| format!("word{}", i)).collect();
        let text2 = words.join(" ");
        let chunks2 = create_literature_chunks(&conn, &lit.id, &text2, &config).unwrap();
        assert!(chunks2.len() > 1);

        // Verify old chunks are gone
        let all_chunks = get_literature_chunks(&conn, &lit.id).unwrap();
        assert_eq!(all_chunks.len(), chunks2.len());
    }

    #[test]
    fn test_chunk_config_default() {
        let config = ChunkConfig::default();
        assert_eq!(config.target_words, 200);
        assert_eq!(config.overlap_words, 20);
    }
}
