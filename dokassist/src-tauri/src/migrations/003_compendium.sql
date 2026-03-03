-- PKG-RAG: Compendium and Document Chunking for RAG
-- Add support for medical compendium as LLM context

-- Add is_compendium flag to files table to mark reference documents
ALTER TABLE files ADD COLUMN is_compendium INTEGER NOT NULL DEFAULT 0;

-- Document chunks table for RAG retrieval
CREATE TABLE IF NOT EXISTS document_chunks (
    id TEXT PRIMARY KEY NOT NULL,
    file_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    token_count INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
);

-- Index for efficient chunk retrieval
CREATE INDEX IF NOT EXISTS idx_chunks_file ON document_chunks(file_id, chunk_index);

-- Compendium entries table for tracking which documents are used as reference material
CREATE TABLE IF NOT EXISTS compendium_entries (
    id TEXT PRIMARY KEY NOT NULL,
    file_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    category TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
);

-- Index for compendium lookups
CREATE INDEX IF NOT EXISTS idx_compendium_file ON compendium_entries(file_id);
CREATE INDEX IF NOT EXISTS idx_compendium_category ON compendium_entries(category);
CREATE INDEX IF NOT EXISTS idx_compendium_priority ON compendium_entries(priority DESC);

-- Trigger to update updated_at on compendium_entries
CREATE TRIGGER IF NOT EXISTS compendium_entries_updated_at
AFTER UPDATE ON compendium_entries
BEGIN
    UPDATE compendium_entries SET updated_at = datetime('now') WHERE id = NEW.id;
END;
