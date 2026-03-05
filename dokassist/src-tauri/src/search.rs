use crate::error::AppError;
use crate::llm::embed::{blob_to_vec, cosine_similarity, vec_to_blob};
use crate::models::diagnosis::Diagnosis;
use crate::models::medication::Medication;
use crate::models::patient::Patient;
use crate::models::session::Session;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub result_type: String,
    pub entity_id: String,
    pub patient_id: String,
    pub patient_name: String,
    pub title: String,
    pub snippet: String,
    pub date: Option<String>,
    pub rank: f64,
}

/// Sanitize a query string into one or more FTS5 phrase-query terms.
///
/// MED-1: FTS5 treats operators like `*`, `OR`, `AND`, `NOT`, `"`, and `^`
/// as metacharacters even when the value is bound via a `?` parameter.
///
/// Each whitespace-separated token is wrapped in double-quotes (phrase
/// mode) which neutralises all metacharacters.  The *last* token also
/// gets a trailing `*` so that partially-typed words match their
/// completions (e.g. `"Vi"*` matches the token "Viktor").  Completed
/// tokens use exact phrase matching.
///
/// Internal double-quotes are escaped as `""` per the FTS5 spec.
fn sanitize_fts5_query(input: &str) -> String {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    if tokens.is_empty() {
        return String::new();
    }
    let last_idx = tokens.len() - 1;
    tokens
        .iter()
        .enumerate()
        .map(|(i, token)| {
            // Escape internal double-quotes (FTS5 phrase-literal escaping)
            let escaped = token.replace('"', "\"\"");
            if i == last_idx {
                // Last token: prefix match so partial input finds completions
                format!("\"{}\"*", escaped)
            } else {
                // Completed tokens: exact token match
                format!("\"{}\"", escaped)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Full-text search across all indexed content
pub fn search(conn: &Connection, query: &str, limit: u32) -> Result<Vec<SearchResult>, AppError> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    // Normalize AHV numbers in query (remove dots for searching)
    let normalized_query = normalize_ahv_for_search(query);
    // MED-1: Escape FTS5 metacharacters to prevent operator injection / DoS
    let normalized_query = sanitize_fts5_query(&normalized_query);

    // FTS5 search with ranking and snippet generation
    let mut stmt = conn.prepare(
        r#"
        SELECT
            entity_type,
            entity_id,
            patient_id,
            patient_name,
            title,
            snippet(search_index, 5, '<mark>', '</mark>', '...', 64) as snippet,
            date,
            bm25(search_index) as rank
        FROM search_index
        WHERE search_index MATCH ?1
        ORDER BY rank
        LIMIT ?2
        "#,
    )?;

    let results = stmt
        .query_map([&normalized_query, &limit.to_string()], |row| {
            Ok(SearchResult {
                result_type: row.get(0)?,
                entity_id: row.get(1)?,
                patient_id: row.get(2)?,
                patient_name: row.get(3)?,
                title: row.get(4)?,
                snippet: row.get(5)?,
                date: row.get(6)?,
                rank: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

/// Index or re-index a patient's searchable fields
pub fn index_patient(conn: &Connection, patient: &Patient) -> Result<(), AppError> {
    // Remove existing patient index entry
    remove_from_index(conn, "patient", &patient.id)?;

    let patient_name = format!("{} {}", patient.first_name, patient.last_name);

    // Prepare content for indexing - include all searchable fields
    let mut content_parts = vec![
        patient.first_name.clone(),
        patient.last_name.clone(),
        patient.ahv_number.clone(),
        patient.ahv_number.replace(".", ""), // Also index plain format
    ];

    // Index both dotted and plain AHV formats
    content_parts.push(patient.ahv_number.clone());
    content_parts.push(patient.ahv_number.replace(".", ""));

    if let Some(ref email) = patient.email {
        content_parts.push(email.clone());
    }

    if let Some(ref phone) = patient.phone {
        content_parts.push(phone.clone());
    }

    let content = content_parts.join(" ");

    conn.execute(
        r#"
        INSERT INTO search_index (entity_type, entity_id, patient_id, patient_name, title, content, date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        (
            "patient",
            &patient.id,
            &patient.id,
            &patient_name,
            &patient_name,
            &content,
            &patient.date_of_birth,
        ),
    )?;

    Ok(())
}

/// Index file content (called after LLM metadata extraction)
#[allow(clippy::too_many_arguments)]
pub fn index_file(
    conn: &Connection,
    file_id: &str,
    patient_id: &str,
    patient_name: &str,
    filename: &str,
    extracted_text: &str,
    document_type: Option<&str>,
    date: Option<&str>,
) -> Result<(), AppError> {
    // Remove existing file index entry
    remove_from_index(conn, "file", file_id)?;

    let title = document_type
        .map(|dt| format!("{} - {}", dt, filename))
        .unwrap_or_else(|| filename.to_string());

    conn.execute(
        r#"
        INSERT INTO search_index (entity_type, entity_id, patient_id, patient_name, title, content, date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        (
            "file",
            file_id,
            patient_id,
            patient_name,
            &title,
            extracted_text,
            date,
        ),
    )?;

    Ok(())
}

/// Index session notes
pub fn index_session(
    conn: &Connection,
    session_id: &str,
    patient_id: &str,
    patient_name: &str,
    session_type: &str,
    notes: &str,
    session_date: &str,
) -> Result<(), AppError> {
    // Remove existing session index entry
    remove_from_index(conn, "session", session_id)?;

    let title = format!("{} - {}", session_type, session_date);

    conn.execute(
        r#"
        INSERT INTO search_index (entity_type, entity_id, patient_id, patient_name, title, content, date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        (
            "session",
            session_id,
            patient_id,
            patient_name,
            &title,
            notes,
            session_date,
        ),
    )?;

    Ok(())
}

/// Index finalized report content
#[allow(clippy::too_many_arguments)]
pub fn index_report(
    conn: &Connection,
    report_id: &str,
    patient_id: &str,
    patient_name: &str,
    report_type: &str,
    title: &str,
    content: &str,
    generated_at: &str,
) -> Result<(), AppError> {
    // Remove existing report index entry
    remove_from_index(conn, "report", report_id)?;

    let full_title = format!("{} - {}", report_type, title);

    conn.execute(
        r#"
        INSERT INTO search_index (entity_type, entity_id, patient_id, patient_name, title, content, date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        (
            "report",
            report_id,
            patient_id,
            patient_name,
            &full_title,
            content,
            generated_at,
        ),
    )?;

    Ok(())
}

/// Remove all index entries for an entity
pub fn remove_from_index(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM search_index WHERE entity_type = ?1 AND entity_id = ?2",
        (entity_type, entity_id),
    )?;

    Ok(())
}

// Wrapper functions that take model structs directly

pub fn index_session_from_model(conn: &Connection, session: &Session) -> Result<(), AppError> {
    // Get patient name from database
    let patient_name: String = conn
        .query_row(
            "SELECT first_name || ' ' || last_name FROM patients WHERE id = ?",
            [&session.patient_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "Unknown Patient".to_string());

    index_session(
        conn,
        &session.id,
        &session.patient_id,
        &patient_name,
        &session.session_type,
        session.notes.as_deref().unwrap_or(""),
        &session.session_date,
    )
}

pub fn index_diagnosis_from_model(
    conn: &Connection,
    diagnosis: &Diagnosis,
) -> Result<(), AppError> {
    // Get patient name from database
    let patient_name: String = conn
        .query_row(
            "SELECT first_name || ' ' || last_name FROM patients WHERE id = ?",
            [&diagnosis.patient_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "Unknown Patient".to_string());

    // Remove existing diagnosis index entry
    remove_from_index(conn, "diagnosis", &diagnosis.id)?;

    let title = format!("{} - {}", diagnosis.icd10_code, diagnosis.description);
    let content = format!(
        "{} {} {}",
        diagnosis.description,
        diagnosis.icd10_code,
        diagnosis.notes.as_deref().unwrap_or("")
    );

    conn.execute(
        r#"
        INSERT INTO search_index (entity_type, entity_id, patient_id, patient_name, title, content, date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        (
            "diagnosis",
            &diagnosis.id,
            &diagnosis.patient_id,
            &patient_name,
            &title,
            &content,
            &diagnosis.diagnosed_date,
        ),
    )?;

    Ok(())
}

pub fn index_medication_from_model(
    conn: &Connection,
    medication: &Medication,
) -> Result<(), AppError> {
    // Get patient name from database
    let patient_name: String = conn
        .query_row(
            "SELECT first_name || ' ' || last_name FROM patients WHERE id = ?",
            [&medication.patient_id],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "Unknown Patient".to_string());

    // Remove existing medication index entry
    remove_from_index(conn, "medication", &medication.id)?;

    let title = format!("{} - {}", medication.substance, medication.dosage);
    let content = format!(
        "{} {} {} {}",
        medication.substance,
        medication.dosage,
        medication.frequency,
        medication.notes.as_deref().unwrap_or("")
    );

    conn.execute(
        r#"
        INSERT INTO search_index (entity_type, entity_id, patient_id, patient_name, title, content, date)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        (
            "medication",
            &medication.id,
            &medication.patient_id,
            &patient_name,
            &title,
            &content,
            &medication.start_date,
        ),
    )?;

    Ok(())
}

// ── Embedding persistence ────────────────────────────────────────────────────

/// Persist a document embedding as a little-endian f32 BLOB.
/// Replaces any existing embedding for the same file_id (UPSERT).
pub fn save_embedding(conn: &Connection, file_id: &str, vector: &[f32]) -> Result<(), AppError> {
    let blob = vec_to_blob(vector);
    conn.execute(
        r#"
        INSERT INTO document_embeddings (file_id, vector)
        VALUES (?1, ?2)
        ON CONFLICT(file_id) DO UPDATE SET vector = excluded.vector,
                                           created_at = datetime('now')
        "#,
        rusqlite::params![file_id, blob],
    )?;
    Ok(())
}

/// Persist a chunk embedding as a little-endian f32 BLOB.
/// Replaces any existing embedding for the same chunk_id (UPSERT).
pub fn save_chunk_embedding(
    conn: &Connection,
    chunk_id: &str,
    vector: &[f32],
) -> Result<(), AppError> {
    let blob = vec_to_blob(vector);
    conn.execute(
        r#"
        INSERT INTO chunk_embeddings (chunk_id, vector)
        VALUES (?1, ?2)
        ON CONFLICT(chunk_id) DO UPDATE SET vector = excluded.vector,
                                           created_at = datetime('now')
        "#,
        rusqlite::params![chunk_id, blob],
    )?;
    Ok(())
}

/// Load all chunk embeddings.  Returns `(chunk_id, vector)` pairs.
pub fn load_chunk_embeddings(conn: &Connection) -> Result<Vec<(String, Vec<f32>)>, AppError> {
    let mut stmt = conn.prepare("SELECT chunk_id, vector FROM chunk_embeddings")?;
    let rows = stmt
        .query_map([], |row| {
            let chunk_id: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((chunk_id, blob))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .map(|(id, blob)| (id, blob_to_vec(&blob)))
        .collect())
}

/// Load all stored embeddings.  Returns `(file_id, vector)` pairs.
pub fn load_embeddings(conn: &Connection) -> Result<Vec<(String, Vec<f32>)>, AppError> {
    let mut stmt = conn.prepare("SELECT file_id, vector FROM document_embeddings")?;
    let rows = stmt
        .query_map([], |row| {
            let file_id: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((file_id, blob))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .map(|(id, blob)| (id, blob_to_vec(&blob)))
        .collect())
}

// ── Semantic search ──────────────────────────────────────────────────────────

/// In-memory cosine similarity search over document embeddings.
///
/// Loads all stored embeddings, ranks them against `query_vec`, then joins
/// the top results with the `files` and `patients` tables to build
/// `SearchResult` values.  Suitable for < 10 000 documents.
pub fn semantic_search(
    conn: &Connection,
    query_vec: &[f32],
    limit: usize,
) -> Result<Vec<SearchResult>, AppError> {
    let embeddings = load_embeddings(conn)?;
    if embeddings.is_empty() {
        return Ok(vec![]);
    }

    // Score and rank
    let mut scored: Vec<(f32, String)> = embeddings
        .into_iter()
        .map(|(file_id, vec)| (cosine_similarity(query_vec, &vec), file_id))
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    // Fetch metadata for each top result
    let mut results = Vec::with_capacity(scored.len());
    for (sim, file_id) in scored {
        let result = conn.query_row(
            r#"
            SELECT f.id, f.patient_id, f.filename, f.mime_type, f.created_at,
                   p.first_name || ' ' || p.last_name AS patient_name
            FROM files f
            JOIN patients p ON f.patient_id = p.id
            WHERE f.id = ?1
            "#,
            [&file_id],
            |row| {
                let filename: String = row.get(2)?;
                let mime_type: String = row.get(3)?;
                let created_at: String = row.get(4)?;
                Ok(SearchResult {
                    result_type: "file".to_string(),
                    entity_id: row.get(0)?,
                    patient_id: row.get(1)?,
                    patient_name: row.get(5)?,
                    title: filename,
                    snippet: format!("[semantic match — {}]", mime_type),
                    date: Some(created_at),
                    rank: sim as f64,
                })
            },
        );

        match result {
            Ok(r) => results.push(r),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // File deleted after embedding was stored — skip silently
            }
            Err(e) => return Err(AppError::Database(e)),
        }
    }

    Ok(results)
}

// ── Hybrid search (FTS5 + semantic, merged via RRF) ──────────────────────────

/// Merge FTS5 keyword results with optional semantic results using
/// Reciprocal Rank Fusion (k = 60).
///
/// If `query_vec` is `None` (embed engine not yet loaded) the function falls
/// back to pure FTS5 results.
pub fn hybrid_search(
    conn: &Connection,
    query: &str,
    query_vec: Option<&[f32]>,
    limit: u32,
) -> Result<Vec<SearchResult>, AppError> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    let fetch_limit = (limit * 2).max(50);

    // FTS5 results
    let fts_results = search(conn, query, fetch_limit)?;

    // Semantic results (empty when engine not loaded or no embeddings exist)
    let sem_results = match query_vec {
        Some(qv) => semantic_search(conn, qv, fetch_limit as usize)?,
        None => vec![],
    };

    // Pure FTS5 fallback if semantic is unavailable
    if sem_results.is_empty() {
        return Ok(fts_results.into_iter().take(limit as usize).collect());
    }

    // Build RRF score map  key = (result_type, entity_id)
    let mut rrf: HashMap<(String, String), f64> = HashMap::new();
    for (rank, r) in fts_results.iter().enumerate() {
        *rrf.entry((r.result_type.clone(), r.entity_id.clone()))
            .or_insert(0.0) += 1.0 / (60.0 + (rank + 1) as f64);
    }
    for (rank, r) in sem_results.iter().enumerate() {
        *rrf.entry((r.result_type.clone(), r.entity_id.clone()))
            .or_insert(0.0) += 1.0 / (60.0 + (rank + 1) as f64);
    }

    // Merge unique results, FTS5 preferred (has snippets)
    let mut result_map: HashMap<(String, String), SearchResult> = HashMap::new();
    for r in fts_results.into_iter().chain(sem_results.into_iter()) {
        let key = (r.result_type.clone(), r.entity_id.clone());
        result_map.entry(key).or_insert(r);
    }

    // Sort by RRF score and apply RRF score to rank field
    let mut merged: Vec<SearchResult> = result_map.into_values().collect();
    merged.sort_by(|a, b| {
        let sa = rrf
            .get(&(a.result_type.clone(), a.entity_id.clone()))
            .copied()
            .unwrap_or(0.0);
        let sb = rrf
            .get(&(b.result_type.clone(), b.entity_id.clone()))
            .copied()
            .unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
    for r in merged.iter_mut() {
        r.rank = rrf
            .get(&(r.result_type.clone(), r.entity_id.clone()))
            .copied()
            .unwrap_or(0.0);
    }

    merged.truncate(limit as usize);
    Ok(merged)
}

// ── Literature chunk search ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct LiteratureChunkResult {
    pub chunk_id: String,
    pub literature_id: String,
    pub filename: String,
    pub chunk_index: i32,
    pub content: String,
    pub similarity: f64,
}

/// Semantic search over literature document chunks.
///
/// Returns the top-k most similar chunks from literature documents.
/// Used for RAG retrieval in chat and report generation.
pub fn search_literature_chunks(
    conn: &Connection,
    query_vec: &[f32],
    limit: usize,
) -> Result<Vec<LiteratureChunkResult>, AppError> {
    let embeddings = load_chunk_embeddings(conn)?;
    if embeddings.is_empty() {
        return Ok(vec![]);
    }

    // Score and rank all chunks
    let mut scored: Vec<(f32, String)> = embeddings
        .into_iter()
        .map(|(chunk_id, vec)| (cosine_similarity(query_vec, &vec), chunk_id))
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    // Fetch chunk metadata and filter for literature chunks only
    let mut results = Vec::with_capacity(scored.len());
    for (sim, chunk_id) in scored {
        let result = conn.query_row(
            r#"
            SELECT dc.id, dc.literature_id, dc.chunk_index, dc.content, l.filename
            FROM document_chunks dc
            JOIN literature l ON dc.literature_id = l.id
            WHERE dc.id = ?1
            "#,
            [&chunk_id],
            |row| {
                Ok(LiteratureChunkResult {
                    chunk_id: row.get(0)?,
                    literature_id: row.get(1)?,
                    chunk_index: row.get(2)?,
                    content: row.get(3)?,
                    filename: row.get(4)?,
                    similarity: sim as f64,
                })
            },
        );

        match result {
            Ok(r) => results.push(r),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // Chunk might be from a patient file, or deleted — skip
            }
            Err(e) => return Err(AppError::Database(e)),
        }
    }

    Ok(results)
}

/// Normalize AHV queries: "7561234567897" and "756.1234.5678.97" both match
fn normalize_ahv_for_search(query: &str) -> String {
    // Check if query looks like an AHV number
    let digits_only: String = query.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits_only.len() == 13 && digits_only.starts_with("756") {
        // Search using plain digits only — FTS5 treats dots as separators,
        // making "756.1234.5678.97" a syntax error as an unquoted query term.
        // The content is indexed with both dotted and plain formats, so
        // matching the plain digits is sufficient.
        digits_only
    } else {
        query.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{self, DbPool};
    use tempfile::TempDir;

    fn setup_test_db() -> (TempDir, DbPool) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let key = [42u8; 32];
        let pool = database::init_db(&db_path, &key).unwrap();
        (temp_dir, pool)
    }

    #[test]
    fn test_sanitize_fts5_query() {
        // Single word becomes a prefix query (last token always gets `*`)
        assert_eq!(sanitize_fts5_query("Viktor"), "\"Viktor\"*");

        // Multi-word: all but the last are exact; the last is a prefix
        assert_eq!(sanitize_fts5_query("John Doe"), "\"John\" \"Doe\"*");

        // FTS5 wildcard inside a word is neutralised (quoted, not a real wildcard)
        assert_eq!(sanitize_fts5_query("*"), "\"*\"*");

        // Boolean operators are neutralised — each word is individually quoted
        assert_eq!(
            sanitize_fts5_query("a OR * OR b"),
            "\"a\" \"OR\" \"*\" \"OR\" \"b\"*"
        );

        // Internal double-quotes are escaped
        assert_eq!(
            sanitize_fts5_query("say \"hello\""),
            "\"say\" \"\"\"hello\"\"\"*"
        );
    }

    #[test]
    fn test_normalize_ahv() {
        let query1 = "756.1234.5678.97";
        let result1 = normalize_ahv_for_search(query1);
        assert!(result1.contains("7561234567897"));

        let query2 = "7561234567897";
        let result2 = normalize_ahv_for_search(query2);
        assert!(result2.contains("7561234567897"));

        let query3 = "John Doe";
        let result3 = normalize_ahv_for_search(query3);
        assert_eq!(result3, "John Doe");
    }

    #[test]
    fn test_index_and_search_patient() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        let patient = Patient {
            id: "patient-123".to_string(),
            first_name: "Max".to_string(),
            last_name: "Müller".to_string(),
            date_of_birth: "1980-01-01".to_string(),
            gender: Some("male".to_string()),
            ahv_number: "756.1234.5678.97".to_string(),
            email: None,
            phone: None,
            address: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        index_patient(&conn, &patient).unwrap();

        // Search by name
        let results = search(&conn, "Müller", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].result_type, "patient");

        // Search by AHV (dotted format)
        let results = search(&conn, "756.1234.5678.97", 10).unwrap();
        assert_eq!(results.len(), 1);

        // Search by AHV (plain format)
        let results = search(&conn, "7561234567897", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_prefix_search() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        let patient = Patient {
            id: "patient-prefix".to_string(),
            first_name: "Viktor".to_string(),
            last_name: "Steiger".to_string(),
            date_of_birth: "1990-06-15".to_string(),
            gender: None,
            ahv_number: "756.9999.0000.12".to_string(),
            email: None,
            phone: None,
            address: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        index_patient(&conn, &patient).unwrap();

        // Two-letter prefix should find "Viktor"
        let results = search(&conn, "Vi", 10).unwrap();
        assert_eq!(results.len(), 1, "prefix 'Vi' should match 'Viktor'");

        // Partial last name
        let results = search(&conn, "Stei", 10).unwrap();
        assert_eq!(results.len(), 1, "prefix 'Stei' should match 'Steiger'");

        // Multi-word prefix: first word exact, last word prefix
        let results = search(&conn, "Viktor Stei", 10).unwrap();
        assert_eq!(results.len(), 1, "multi-word prefix should match");
    }

    #[test]
    fn test_search_empty_query() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();
        let results = search(&conn, "", 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_save_and_load_embeddings() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        // Insert a patient + file so the FK is satisfied
        let patient_id = uuid::Uuid::now_v7().to_string();
        conn.execute(
            "INSERT INTO patients (id, first_name, last_name, date_of_birth, ahv_number)
             VALUES (?1, 'Test', 'User', '1990-01-01', '7560000000000')",
            [&patient_id],
        )
        .unwrap();
        let file_id = uuid::Uuid::now_v7().to_string();
        conn.execute(
            "INSERT INTO files (id, patient_id, filename, vault_path, mime_type, size_bytes)
             VALUES (?1, ?2, 'test.pdf', 'p/f.enc', 'application/pdf', 1000)",
            rusqlite::params![file_id, patient_id],
        )
        .unwrap();

        let original: Vec<f32> = (0..768).map(|i| i as f32 / 768.0).collect();
        save_embedding(&conn, &file_id, &original).unwrap();

        let loaded = load_embeddings(&conn).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].0, file_id);
        assert_eq!(loaded[0].1.len(), 768);

        for (a, b) in original.iter().zip(loaded[0].1.iter()) {
            assert!((a - b).abs() < 1e-6, "Round-trip mismatch: {a} vs {b}");
        }
    }

    #[test]
    fn test_cosine_similarity_via_search_module() {
        // Sanity-check that the cosine_similarity re-exported from embed works
        // when accessed through the search module's dependency.
        use crate::llm::embed::cosine_similarity;
        let v = vec![1.0f32, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
        let w = vec![0.0f32, 1.0, 0.0];
        assert!(cosine_similarity(&v, &w).abs() < 1e-6);
    }

    #[test]
    fn test_hybrid_search_fallback_no_embeddings() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        let patient = Patient {
            id: "patient-hybrid".to_string(),
            first_name: "Hybrid".to_string(),
            last_name: "Patient".to_string(),
            date_of_birth: "1985-03-20".to_string(),
            gender: None,
            ahv_number: "756.0000.0001.23".to_string(),
            email: None,
            phone: None,
            address: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        index_patient(&conn, &patient).unwrap();

        // No embeddings in DB; should still return FTS5 results
        let results = hybrid_search(&conn, "Hybrid", None, 10).unwrap();
        assert_eq!(results.len(), 1, "FTS5 fallback should return 1 result");
        assert_eq!(results[0].result_type, "patient");

        // With an empty query vec the code path is identical (no semantic results)
        let dummy_qvec: Vec<f32> = vec![0.0; 768];
        let results2 = hybrid_search(&conn, "Hybrid", Some(&dummy_qvec), 10).unwrap();
        assert_eq!(
            results2.len(),
            1,
            "FTS5 + empty semantic should still find the patient"
        );
    }

    #[test]
    fn test_remove_from_index() {
        let (_temp_dir, pool) = setup_test_db();
        let conn = pool.conn().unwrap();

        let patient = Patient {
            id: "patient-456".to_string(),
            first_name: "Anna".to_string(),
            last_name: "Schmidt".to_string(),
            date_of_birth: "1990-05-15".to_string(),
            gender: Some("female".to_string()),
            ahv_number: "756.0000.0004.56".to_string(),
            email: None,
            phone: None,
            address: None,
            insurance: None,
            gp_name: None,
            gp_address: None,
            notes: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        index_patient(&conn, &patient).unwrap();

        let results = search(&conn, "Schmidt", 10).unwrap();
        assert_eq!(results.len(), 1);

        remove_from_index(&conn, "patient", &patient.id).unwrap();

        let results = search(&conn, "Schmidt", 10).unwrap();
        assert_eq!(results.len(), 0);
    }
}
