-- Migration 014: add a cryptographic HMAC chain to the audit log.
--
-- The existing append-only triggers must be removed temporarily so legacy rows can
-- be backfilled inside the migration transaction. Rust migration code computes each
-- MAC before installing the v14 integrity triggers and committing the transaction.

DROP TRIGGER IF EXISTS audit_log_no_update;
DROP TRIGGER IF EXISTS audit_log_no_delete;
DROP TRIGGER IF EXISTS audit_log_signed_insert;

ALTER TABLE audit_log ADD COLUMN sequence INTEGER;
ALTER TABLE audit_log ADD COLUMN previous_mac BLOB;
ALTER TABLE audit_log ADD COLUMN entry_mac BLOB;

CREATE UNIQUE INDEX IF NOT EXISTS idx_audit_sequence ON audit_log(sequence);
