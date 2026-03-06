-- Migration for literature management and document chunking (RAG support)

-- Literature documents (general reference material not tied to specific patients)
CREATE TABLE literature (
    id          TEXT PRIMARY KEY NOT NULL,
    filename    TEXT NOT NULL,
    vault_path  TEXT NOT NULL,
    mime_type   TEXT NOT NULL,
    size_bytes  INTEGER NOT NULL,
    description TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- Document chunks for RAG retrieval (supports both patient files and literature)
-- Using ~200 words per chunk (≈200-250 tokens) with overlap for better context and reduced memory pressure
CREATE TABLE document_chunks (
    id          TEXT PRIMARY KEY NOT NULL,
    file_id     TEXT,
    literature_id TEXT,
    chunk_index INTEGER NOT NULL,
    content     TEXT NOT NULL,
    word_count  INTEGER NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),

    -- Exactly one of file_id or literature_id must be set
    CHECK ((file_id IS NOT NULL AND literature_id IS NULL) OR (file_id IS NULL AND literature_id IS NOT NULL)),

    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY (literature_id) REFERENCES literature(id) ON DELETE CASCADE
) STRICT;

-- Embeddings for document chunks (768-dim vectors for semantic search)
CREATE TABLE chunk_embeddings (
    chunk_id   TEXT PRIMARY KEY NOT NULL,
    vector     BLOB NOT NULL,
    model      TEXT NOT NULL DEFAULT 'nomic-embed-text-v1.5',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (chunk_id) REFERENCES document_chunks(id) ON DELETE CASCADE
) STRICT;

-- Index for efficient chunk lookup
CREATE INDEX idx_chunks_file ON document_chunks(file_id);
CREATE INDEX idx_chunks_literature ON document_chunks(literature_id);
