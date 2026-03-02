-- PKG-2 Initial Schema
-- Full schema for DokAssist encrypted database

-- Patients table
CREATE TABLE IF NOT EXISTS patients (
    id TEXT PRIMARY KEY NOT NULL,
    ahv_number TEXT NOT NULL UNIQUE,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    date_of_birth TEXT NOT NULL,
    gender TEXT,
    address TEXT,
    phone TEXT,
    email TEXT,
    insurance TEXT,
    gp_name TEXT,
    gp_address TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Sessions table (clinical visits)
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY NOT NULL,
    patient_id TEXT NOT NULL,
    session_date TEXT NOT NULL,
    session_type TEXT NOT NULL,
    duration_minutes INTEGER,
    notes TEXT,
    amdp_data TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (patient_id) REFERENCES patients(id) ON DELETE CASCADE
);

-- Files table (metadata only, actual files in encrypted vault)
CREATE TABLE IF NOT EXISTS files (
    id TEXT PRIMARY KEY NOT NULL,
    patient_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    vault_path TEXT NOT NULL UNIQUE,
    mime_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    document_type TEXT,
    extracted_text TEXT,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (patient_id) REFERENCES patients(id) ON DELETE CASCADE
);

-- Diagnoses table
CREATE TABLE IF NOT EXISTS diagnoses (
    id TEXT PRIMARY KEY NOT NULL,
    patient_id TEXT NOT NULL,
    icd10_code TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    diagnosed_date TEXT NOT NULL,
    resolved_date TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (patient_id) REFERENCES patients(id) ON DELETE CASCADE
);

-- Medications table
CREATE TABLE IF NOT EXISTS medications (
    id TEXT PRIMARY KEY NOT NULL,
    patient_id TEXT NOT NULL,
    substance TEXT NOT NULL,
    dosage TEXT NOT NULL,
    frequency TEXT NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (patient_id) REFERENCES patients(id) ON DELETE CASCADE
);

-- Reports table
CREATE TABLE IF NOT EXISTS reports (
    id TEXT PRIMARY KEY NOT NULL,
    patient_id TEXT NOT NULL,
    report_type TEXT NOT NULL,
    content TEXT NOT NULL,
    generated_at TEXT NOT NULL DEFAULT (datetime('now')),
    model_name TEXT,
    prompt_hash TEXT,
    session_ids TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (patient_id) REFERENCES patients(id) ON DELETE CASCADE
);

-- Audit log table (PKG-6, placeholder for now)
CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT,
    details TEXT
);

-- FTS5 virtual table for full-text search (PKG-5)
CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(
    entity_type,
    entity_id,
    patient_id,
    patient_name,
    title,
    content,
    date,
    tokenize = 'unicode61 remove_diacritics 2'
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_patients_ahv ON patients(ahv_number);
CREATE INDEX IF NOT EXISTS idx_patients_name ON patients(last_name, first_name);
CREATE INDEX IF NOT EXISTS idx_sessions_patient ON sessions(patient_id, session_date DESC);
CREATE INDEX IF NOT EXISTS idx_files_patient ON files(patient_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_diagnoses_patient ON diagnoses(patient_id, status);
CREATE INDEX IF NOT EXISTS idx_medications_patient ON medications(patient_id, start_date DESC);
CREATE INDEX IF NOT EXISTS idx_reports_patient ON reports(patient_id, generated_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit_log(entity_type, entity_id);

-- Trigger to update updated_at on patients
CREATE TRIGGER IF NOT EXISTS patients_updated_at
AFTER UPDATE ON patients
BEGIN
    UPDATE patients SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- Trigger to update updated_at on sessions
CREATE TRIGGER IF NOT EXISTS sessions_updated_at
AFTER UPDATE ON sessions
BEGIN
    UPDATE sessions SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- Trigger to update updated_at on diagnoses
CREATE TRIGGER IF NOT EXISTS diagnoses_updated_at
AFTER UPDATE ON diagnoses
BEGIN
    UPDATE diagnoses SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- Trigger to update updated_at on medications
CREATE TRIGGER IF NOT EXISTS medications_updated_at
AFTER UPDATE ON medications
BEGIN
    UPDATE medications SET updated_at = datetime('now') WHERE id = NEW.id;
END;
