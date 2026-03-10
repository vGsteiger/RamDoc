use crate::error::AppError;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::path::Path;

/// Local embedding engine backed by fastembed / ONNX Runtime.
///
/// Uses `NomicEmbedTextV15` (768-dimensional) which performs well on
/// medical/clinical text.  The ONNX model (~130 MB) is downloaded once to
/// `cache_dir` and reused across sessions.
pub struct EmbedEngine {
    model: TextEmbedding,
}

impl EmbedEngine {
    /// Initialise the embedding engine, downloading the model to `cache_dir`
    /// on first use.  Subsequent calls reuse the cached files.
    ///
    /// This is a **blocking** call (disk I/O + optional network download).
    /// Always call from a `tokio::task::spawn_blocking` context.
    pub fn new(cache_dir: &Path) -> Result<Self, AppError> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::NomicEmbedTextV15)
                .with_cache_dir(cache_dir.to_path_buf())
                .with_show_download_progress(true),
        )
        .map_err(|e| AppError::Llm(format!("Failed to initialise embedding model: {e}")))?;

        Ok(Self { model })
    }

    /// Embed a batch of texts.  Returns one `Vec<f32>` per input string.
    pub fn embed_texts(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>, AppError> {
        self.model
            .embed(texts, None)
            .map_err(|e| AppError::Llm(format!("Embedding failed: {e}")))
    }

    /// Embed a single text — convenience wrapper around [`embed_texts`].
    pub fn embed_one(&mut self, text: &str) -> Result<Vec<f32>, AppError> {
        let mut vecs = self.embed_texts(&[text])?;
        vecs.pop()
            .ok_or_else(|| AppError::Llm("Embedding returned empty result".to_string()))
    }
}

// ── Vector serialisation helpers ────────────────────────────────────────────

/// Serialise a `f32` slice to a little-endian byte vector for BLOB storage.
pub fn vec_to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Deserialise a little-endian byte slice back to `Vec<f32>`.
pub fn blob_to_vec(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Cosine similarity between two equal-length vectors.
/// Returns `0.0` if either vector has zero magnitude.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(
        a.len(),
        b.len(),
        "Vector lengths must match for cosine similarity"
    );
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_blob_roundtrip() {
        let original: Vec<f32> = vec![0.1, -0.5, 1.0, 0.0, f32::MAX];
        let blob = vec_to_blob(&original);
        let recovered = blob_to_vec(&blob);
        assert_eq!(original.len(), recovered.len());
        for (a, b) in original.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < 1e-6, "Round-trip mismatch: {a} vs {b}");
        }
    }

    #[test]
    fn test_blob_alignment() {
        // Each f32 is exactly 4 bytes
        let v: Vec<f32> = vec![1.0, 2.0, 3.0];
        assert_eq!(vec_to_blob(&v).len(), 12);
    }

    #[test]
    fn test_cosine_identical_vectors() {
        let v = vec![1.0f32, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "Identical vectors → similarity 1.0, got {sim}"
        );
    }

    #[test]
    fn test_cosine_orthogonal_vectors() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![0.0f32, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            sim.abs() < 1e-6,
            "Orthogonal vectors → similarity 0.0, got {sim}"
        );
    }

    #[test]
    fn test_cosine_opposite_vectors() {
        let a = vec![1.0f32, 0.0];
        let b = vec![-1.0f32, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim + 1.0).abs() < 1e-6,
            "Opposite vectors → similarity -1.0, got {sim}"
        );
    }

    #[test]
    fn test_cosine_zero_vector() {
        let a = vec![0.0f32, 0.0, 0.0];
        let b = vec![1.0f32, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0, "Zero vector → similarity 0.0");
    }
}
