//! Tauri commands for literature document management

use crate::crypto::CryptoEngine;
use crate::database::DbPool;
use crate::error::AppError;
use crate::filesystem::FileSystem;
use crate::llm::chunk::{create_literature_chunks, get_literature_chunks, ChunkConfig};
use crate::llm::embed::EmbedEngine;
use crate::models::literature::{
    create_literature, delete_literature, get_literature, list_literature, update_literature,
    CreateLiterature, Literature, UpdateLiterature,
};
use crate::search::{save_chunk_embedding, search_literature_chunks, LiteratureChunkResult};
use std::sync::Arc;
use tauri::Emitter;

/// Upload a literature document
#[tauri::command]
pub async fn upload_literature(
    db: tauri::State<'_, DbPool>,
    fs: tauri::State<'_, Arc<FileSystem>>,
    filename: String,
    data: Vec<u8>,
    mime_type: String,
    description: Option<String>,
) -> Result<Literature, AppError> {
    if data.len() > 500 * 1024 * 1024 {
        return Err(AppError::Validation(
            "File size exceeds 500 MiB limit".to_string(),
        ));
    }

    // Generate vault path
    let lit_id = uuid::Uuid::now_v7().to_string();
    let vault_path = format!("literature/{}.enc", lit_id);

    // Write encrypted file to vault
    fs.write_encrypted(&vault_path, &data)?;

    // Create database record
    let conn = db.conn()?;
    let literature = create_literature(
        &conn,
        CreateLiterature {
            filename,
            vault_path,
            mime_type,
            size_bytes: data.len() as i64,
            description,
        },
    )?;

    Ok(literature)
}

/// Get a literature document by ID
#[tauri::command]
pub async fn get_literature_by_id(
    db: tauri::State<'_, DbPool>,
    id: String,
) -> Result<Literature, AppError> {
    let conn = db.conn()?;
    get_literature(&conn, &id)
}

/// List all literature documents
#[tauri::command]
pub async fn list_all_literature(
    db: tauri::State<'_, DbPool>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Literature>, AppError> {
    let conn = db.conn()?;
    list_literature(&conn, limit.unwrap_or(100), offset.unwrap_or(0))
}

/// Update literature metadata
#[tauri::command]
pub async fn update_literature_metadata(
    db: tauri::State<'_, DbPool>,
    id: String,
    description: Option<String>,
) -> Result<Literature, AppError> {
    let conn = db.conn()?;
    update_literature(&conn, &id, UpdateLiterature { description })
}

/// Delete a literature document
#[tauri::command]
pub async fn delete_literature_document(
    db: tauri::State<'_, DbPool>,
    fs: tauri::State<'_, Arc<FileSystem>>,
    id: String,
) -> Result<(), AppError> {
    let conn = db.conn()?;

    // Get literature to find vault path
    let literature = get_literature(&conn, &id)?;

    // Delete from database (cascades to chunks and embeddings)
    delete_literature(&conn, &id)?;

    // Delete encrypted file from vault
    fs.delete_file(&literature.vault_path)?;

    Ok(())
}

/// Download a literature document
#[tauri::command]
pub async fn download_literature(
    db: tauri::State<'_, DbPool>,
    fs: tauri::State<'_, Arc<FileSystem>>,
    id: String,
) -> Result<Vec<u8>, AppError> {
    let conn = db.conn()?;
    let literature = get_literature(&conn, &id)?;
    let decrypted = fs.read_decrypted(&literature.vault_path)?;
    Ok(decrypted)
}

/// Process a literature document (extract text, chunk, and embed)
#[tauri::command]
pub async fn process_literature(
    db: tauri::State<'_, DbPool>,
    fs: tauri::State<'_, Arc<FileSystem>>,
    crypto: tauri::State<'_, Arc<CryptoEngine>>,
    embed: tauri::State<'_, Arc<EmbedEngine>>,
    app: tauri::AppHandle,
    id: String,
) -> Result<(), AppError> {
    let conn = db.conn()?;
    let literature = get_literature(&conn, &id)?;

    // Read and decrypt file
    let decrypted = fs.read_decrypted(&literature.vault_path)?;

    // Extract text based on mime type
    let extracted_text = match literature.mime_type.as_str() {
        "application/pdf" => {
            let text = pdf_extract::extract_text_from_mem(&decrypted)
                .map_err(|e| AppError::Validation(format!("PDF extraction failed: {}", e)))?;
            text
        }
        "text/plain" => String::from_utf8(decrypted)
            .map_err(|e| AppError::Validation(format!("UTF-8 decode failed: {}", e)))?,
        _ => {
            return Err(AppError::Validation(format!(
                "Unsupported file type for processing: {}",
                literature.mime_type
            )));
        }
    };

    if extracted_text.trim().is_empty() {
        return Err(AppError::Validation(
            "No text content could be extracted from file".to_string(),
        ));
    }

    // Chunk the text
    let config = ChunkConfig::default();
    let chunks = create_literature_chunks(&conn, &id, &extracted_text, &config)?;

    // Wait for embed engine to be ready
    if !embed.is_ready().await {
        return Err(AppError::Validation(
            "Embedding engine not ready".to_string(),
        ));
    }

    // Generate and save embeddings for each chunk
    for chunk in chunks {
        let embedding = embed.embed(&chunk.content).await?;
        save_chunk_embedding(&conn, &chunk.id, &embedding)?;
    }

    // Emit event that processing is complete
    app.emit("literature-processed", &id)
        .map_err(|e| AppError::Internal(format!("Failed to emit event: {}", e)))?;

    Ok(())
}

/// Search literature chunks using semantic similarity
#[tauri::command]
pub async fn search_literature(
    db: tauri::State<'_, DbPool>,
    embed: tauri::State<'_, Arc<EmbedEngine>>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<LiteratureChunkResult>, AppError> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    // Wait for embed engine to be ready
    if !embed.is_ready().await {
        return Err(AppError::Validation(
            "Embedding engine not ready".to_string(),
        ));
    }

    // Embed the query
    let query_vec = embed.embed(&query).await?;

    // Search literature chunks
    let conn = db.conn()?;
    search_literature_chunks(&conn, &query_vec, limit.unwrap_or(5))
}

/// Get all chunks for a literature document (for debugging/inspection)
#[tauri::command]
pub async fn get_literature_document_chunks(
    db: tauri::State<'_, DbPool>,
    id: String,
) -> Result<Vec<crate::llm::chunk::DocumentChunk>, AppError> {
    let conn = db.conn()?;
    get_literature_chunks(&conn, &id)
}
