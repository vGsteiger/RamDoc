-- Migration 012: Practice Settings
-- Stores practice configuration and onboarding completion state

-- Practice settings table: stores practice details and preferences
CREATE TABLE IF NOT EXISTS practice_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1), -- Single-row table
    practice_name TEXT,
    practice_address TEXT,
    practice_phone TEXT,
    practice_email TEXT,
    therapist_name TEXT,
    zsr_number TEXT, -- Swiss provider number for TARMED billing
    canton TEXT, -- Swiss canton for Taxpunktwert
    clinical_specialty TEXT,
    language_preference TEXT DEFAULT 'de',
    onboarding_completed INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Insert default row
INSERT OR IGNORE INTO practice_settings (id) VALUES (1);

-- Trigger to update updated_at on practice_settings
CREATE TRIGGER IF NOT EXISTS practice_settings_updated_at
AFTER UPDATE ON practice_settings
BEGIN
    UPDATE practice_settings SET updated_at = datetime('now') WHERE id = 1;
END;
