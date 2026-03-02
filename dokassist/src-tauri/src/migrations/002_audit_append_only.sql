-- Migration 002: Enforce append-only audit log (CRIT-5)
--
-- The audit_log table must be immutable for healthcare compliance.
-- These triggers raise a hard error on any UPDATE or DELETE attempt,
-- regardless of which code path invokes the statement.
--
-- Note: INSERT and SELECT are still permitted.

CREATE TRIGGER IF NOT EXISTS audit_log_no_update
BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log is append-only: UPDATE not permitted');
END;

CREATE TRIGGER IF NOT EXISTS audit_log_no_delete
BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log is append-only: DELETE not permitted');
END;
