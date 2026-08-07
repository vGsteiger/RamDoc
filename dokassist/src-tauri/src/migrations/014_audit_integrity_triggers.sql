-- Trusted v14 audit triggers. Reinstalled on every database open so a modified
-- trigger definition cannot silently weaken enforcement for application writes.

DROP TRIGGER IF EXISTS audit_log_no_update;
DROP TRIGGER IF EXISTS audit_log_no_delete;
DROP TRIGGER IF EXISTS audit_log_signed_insert;

CREATE TRIGGER audit_log_no_update
BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log is append-only: UPDATE not permitted');
END;

CREATE TRIGGER audit_log_no_delete
BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log is append-only: DELETE not permitted');
END;

CREATE TRIGGER audit_log_signed_insert
BEFORE INSERT ON audit_log
WHEN NEW.id IS NULL
  OR NEW.id != COALESCE((SELECT MAX(id) FROM audit_log), 0) + 1
  OR NEW.sequence IS NULL
  OR NEW.sequence != COALESCE((SELECT MAX(sequence) FROM audit_log), 0) + 1
  OR NEW.previous_mac IS NULL
  OR length(NEW.previous_mac) != 32
  OR NEW.previous_mac != COALESCE(
      (SELECT entry_mac FROM audit_log ORDER BY sequence DESC LIMIT 1),
      zeroblob(32)
  )
  OR NEW.entry_mac IS NULL
  OR length(NEW.entry_mac) != 32
BEGIN
    SELECT RAISE(ABORT, 'audit_log requires a structurally valid chain entry');
END;
