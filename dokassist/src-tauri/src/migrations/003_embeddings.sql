CREATE TABLE IF NOT EXISTS document_embeddings (
    file_id    TEXT PRIMARY KEY NOT NULL,
    vector     BLOB NOT NULL,   -- little-endian f32 array, 768 dims
    model      TEXT NOT NULL DEFAULT 'nomic-embed-text-v1.5',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (file_id) REFERENCES files(id) ON DELETE CASCADE
);
