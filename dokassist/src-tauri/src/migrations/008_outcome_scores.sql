-- Migration 007: Outcome scores table for standardized mental health questionnaires
-- Supports PHQ-9, GAD-7, BDI-II and their scoring

CREATE TABLE IF NOT EXISTS outcome_scores (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    scale_type TEXT NOT NULL,  -- 'PHQ-9', 'GAD-7', 'BDI-II'
    score INTEGER NOT NULL,
    interpretation TEXT,  -- 'Minimal', 'Mild', 'Moderate', etc.
    subscores TEXT,  -- JSON for item-level scores
    administered_at TEXT NOT NULL,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

-- Index for querying scores by session
CREATE INDEX IF NOT EXISTS idx_outcome_scores_session
ON outcome_scores(session_id, administered_at DESC);

-- Index for trend queries across sessions
CREATE INDEX IF NOT EXISTS idx_outcome_scores_scale_date
ON outcome_scores(scale_type, administered_at DESC);

-- Trigger to auto-update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS outcome_scores_updated_at
AFTER UPDATE ON outcome_scores
BEGIN
    UPDATE outcome_scores SET updated_at = datetime('now') WHERE id = NEW.id;
END;
