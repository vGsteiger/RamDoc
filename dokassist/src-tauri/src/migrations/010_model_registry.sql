-- Migration 010: Model Registry
-- Enables multi-model management with task-specific model assignment

-- Model registry table: tracks downloaded models and their metadata
CREATE TABLE IF NOT EXISTS models (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    filename TEXT NOT NULL UNIQUE,
    sha256 TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    downloaded_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_used TEXT,
    is_default INTEGER NOT NULL DEFAULT 0
);

-- Task-model assignments table: maps tasks to specific models
CREATE TABLE IF NOT EXISTS task_models (
    task_type TEXT PRIMARY KEY NOT NULL,
    model_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (model_id) REFERENCES models(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_models_filename ON models(filename);
CREATE INDEX IF NOT EXISTS idx_models_default ON models(is_default);
CREATE INDEX IF NOT EXISTS idx_models_last_used ON models(last_used DESC);

-- Trigger to update updated_at on task_models
CREATE TRIGGER IF NOT EXISTS task_models_updated_at
AFTER UPDATE ON task_models
BEGIN
    UPDATE task_models SET updated_at = datetime('now') WHERE task_type = NEW.task_type;
END;
