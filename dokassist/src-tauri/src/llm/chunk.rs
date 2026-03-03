use crate::error::AppError;

/// Configuration for document chunking
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Target size of each chunk in tokens (approximate)
    pub chunk_size: usize,
    /// Overlap between chunks in tokens to maintain context
    pub chunk_overlap: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512,
            chunk_overlap: 128,
        }
    }
}

/// A text chunk with metadata
#[derive(Debug, Clone)]
pub struct TextChunk {
    pub content: String,
    pub token_count: usize,
    pub chunk_index: usize,
}

/// Split a document into overlapping chunks for RAG retrieval
pub fn chunk_document(text: &str, config: &ChunkConfig) -> Result<Vec<TextChunk>, AppError> {
    if text.is_empty() {
        return Ok(vec![]);
    }

    // Split text into sentences to avoid breaking in the middle of a sentence
    let sentences = split_into_sentences(text);

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_tokens = 0;
    let mut chunk_index = 0;
    let mut overlap_sentences = Vec::new();

    for sentence in sentences {
        let sentence_tokens = estimate_token_count(&sentence);

        // If adding this sentence would exceed chunk size, finalize the current chunk
        if current_tokens + sentence_tokens > config.chunk_size && !current_chunk.is_empty() {
            chunks.push(TextChunk {
                content: current_chunk.trim().to_string(),
                token_count: current_tokens,
                chunk_index,
            });

            chunk_index += 1;

            // Start new chunk with overlap from previous chunk
            current_chunk = overlap_sentences.join(" ") + " ";
            current_tokens = overlap_sentences.iter().map(|s: &String| estimate_token_count(s)).sum();
            overlap_sentences.clear();
        }

        // Add sentence to current chunk
        if !current_chunk.is_empty() && !current_chunk.ends_with(' ') {
            current_chunk.push(' ');
        }
        current_chunk.push_str(&sentence);
        current_tokens += sentence_tokens;

        // Track sentences for overlap
        overlap_sentences.push(sentence.clone());
        let overlap_tokens: usize = overlap_sentences.iter().map(|s: &String| estimate_token_count(s)).sum();

        // Remove old sentences if overlap is too large
        while overlap_tokens > config.chunk_overlap && overlap_sentences.len() > 1 {
            overlap_sentences.remove(0);
        }
    }

    // Add the last chunk if it has content
    if !current_chunk.is_empty() {
        chunks.push(TextChunk {
            content: current_chunk.trim().to_string(),
            token_count: current_tokens,
            chunk_index,
        });
    }

    Ok(chunks)
}

/// Split text into sentences using simple heuristics
fn split_into_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        current.push(c);

        // Check for sentence ending punctuation
        if matches!(c, '.' | '!' | '?') {
            // Look ahead to see if this is really the end of a sentence
            if let Some(&next) = chars.peek() {
                // Not a sentence end if followed by lowercase letter or digit
                if next.is_whitespace() {
                    // Consume whitespace
                    while let Some(&ws) = chars.peek() {
                        if ws.is_whitespace() {
                            current.push(ws);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    // Check if next char after whitespace suggests sentence end
                    if let Some(&next_after_space) = chars.peek() {
                        if next_after_space.is_uppercase() || matches!(next_after_space, '"' | '\'') {
                            sentences.push(current.trim().to_string());
                            current = String::new();
                        }
                    } else {
                        // End of text
                        sentences.push(current.trim().to_string());
                        current = String::new();
                    }
                }
            } else {
                // End of text
                sentences.push(current.trim().to_string());
                current = String::new();
            }
        }

        // Also split on newlines for structured text
        if c == '\n' && !current.trim().is_empty() {
            // Check if we have multiple newlines (paragraph break)
            if let Some(&'\n') = chars.peek() {
                sentences.push(current.trim().to_string());
                current = String::new();
            }
        }
    }

    // Add any remaining text
    if !current.trim().is_empty() {
        sentences.push(current.trim().to_string());
    }

    sentences
}

/// Estimate token count for a text string
/// Uses a simple heuristic: ~4 characters per token for European languages
fn estimate_token_count(text: &str) -> usize {
    // Remove extra whitespace
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    // Rough estimate: 1 token ≈ 4 characters
    (normalized.len() + 3) / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_empty_document() {
        let config = ChunkConfig::default();
        let chunks = chunk_document("", &config).unwrap();
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_chunk_short_document() {
        let config = ChunkConfig::default();
        let text = "This is a short document. It has only two sentences.";
        let chunks = chunk_document(text, &config).unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].chunk_index, 0);
        assert!(chunks[0].content.contains("short document"));
    }

    #[test]
    fn test_chunk_with_overlap() {
        let config = ChunkConfig {
            chunk_size: 50,
            chunk_overlap: 20,
        };

        // Create a document with multiple sentences
        let text = "This is the first sentence. \
                   This is the second sentence. \
                   This is the third sentence. \
                   This is the fourth sentence. \
                   This is the fifth sentence. \
                   This is the sixth sentence.";

        let chunks = chunk_document(text, &config).unwrap();

        // Should create multiple chunks
        assert!(chunks.len() > 1);

        // Each chunk should have an index
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, i);
        }
    }

    #[test]
    fn test_estimate_token_count() {
        // Test basic token estimation
        assert!(estimate_token_count("Hello world") > 0);
        assert!(estimate_token_count("A longer sentence with more words") > estimate_token_count("Short"));
    }

    #[test]
    fn test_split_into_sentences() {
        let text = "First sentence. Second sentence! Third sentence? Fourth.";
        let sentences = split_into_sentences(text);

        assert!(sentences.len() >= 3);
        assert!(sentences[0].contains("First"));
    }

    #[test]
    fn test_chunk_preserves_content() {
        let config = ChunkConfig {
            chunk_size: 100,
            chunk_overlap: 20,
        };

        let text = "Medical diagnosis includes patient symptoms. \
                   Patient presents with fever and cough. \
                   Treatment plan includes rest and medication. \
                   Follow-up appointment scheduled for next week.";

        let chunks = chunk_document(text, &config).unwrap();

        // All chunks should contain meaningful content
        for chunk in chunks {
            assert!(!chunk.content.is_empty());
            assert!(chunk.token_count > 0);
        }
    }
}
