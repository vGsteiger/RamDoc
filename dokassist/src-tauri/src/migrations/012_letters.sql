-- Letters table for referral letters, insurance authorization, and therapy extension requests
CREATE TABLE IF NOT EXISTS letters (
    id TEXT PRIMARY KEY NOT NULL,
    patient_id TEXT NOT NULL,
    letter_type TEXT NOT NULL CHECK (letter_type IN ('referral', 'insurance_authorization', 'therapy_extension')),
    template_language TEXT NOT NULL CHECK (template_language IN ('de', 'fr')),
    recipient_name TEXT,
    recipient_address TEXT,
    subject TEXT NOT NULL,
    content TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'finalized', 'sent')),
    model_name TEXT,
    session_ids TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    finalized_at TEXT,
    sent_at TEXT,
    FOREIGN KEY (patient_id) REFERENCES patients(id) ON DELETE CASCADE
);

-- Index for performance
CREATE INDEX IF NOT EXISTS idx_letters_patient ON letters(patient_id, created_at DESC);

-- Trigger to update updated_at on letters
CREATE TRIGGER IF NOT EXISTS letters_updated_at
AFTER UPDATE ON letters
BEGIN
    UPDATE letters SET updated_at = datetime('now') WHERE id = NEW.id;
END;
