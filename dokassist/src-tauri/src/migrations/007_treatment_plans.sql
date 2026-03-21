-- Treatment Plans Management
CREATE TABLE IF NOT EXISTS treatment_plans (
    id TEXT PRIMARY KEY NOT NULL,
    patient_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    start_date TEXT NOT NULL,
    end_date TEXT,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'completed', 'revised', 'discontinued')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (patient_id) REFERENCES patients(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_treatment_plans_patient ON treatment_plans(patient_id, status);
CREATE INDEX IF NOT EXISTS idx_treatment_plans_dates ON treatment_plans(start_date, end_date);

CREATE TRIGGER IF NOT EXISTS treatment_plans_updated_at
AFTER UPDATE ON treatment_plans
BEGIN
    UPDATE treatment_plans SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- Treatment Goals
CREATE TABLE IF NOT EXISTS treatment_goals (
    id TEXT PRIMARY KEY NOT NULL,
    treatment_plan_id TEXT NOT NULL,
    description TEXT NOT NULL,
    target_date TEXT,
    status TEXT NOT NULL DEFAULT 'in_progress' CHECK (status IN ('pending', 'in_progress', 'achieved', 'revised', 'discontinued')),
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (treatment_plan_id) REFERENCES treatment_plans(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_treatment_goals_plan ON treatment_goals(treatment_plan_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_treatment_goals_status ON treatment_goals(status);

CREATE TRIGGER IF NOT EXISTS treatment_goals_updated_at
AFTER UPDATE ON treatment_goals
BEGIN
    UPDATE treatment_goals SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- Treatment Interventions
CREATE TABLE IF NOT EXISTS treatment_interventions (
    id TEXT PRIMARY KEY NOT NULL,
    treatment_plan_id TEXT NOT NULL,
    type TEXT NOT NULL CHECK (type IN ('psychotherapy', 'medication', 'referral', 'other')),
    description TEXT NOT NULL,
    frequency TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (treatment_plan_id) REFERENCES treatment_plans(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_treatment_interventions_plan ON treatment_interventions(treatment_plan_id);
CREATE INDEX IF NOT EXISTS idx_treatment_interventions_type ON treatment_interventions(type);

CREATE TRIGGER IF NOT EXISTS treatment_interventions_updated_at
AFTER UPDATE ON treatment_interventions
BEGIN
    UPDATE treatment_interventions SET updated_at = datetime('now') WHERE id = NEW.id;
END;
