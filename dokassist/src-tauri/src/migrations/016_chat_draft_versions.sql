-- Migration 016: durable, reversible versions for clinician-reviewed chat drafts.
--
-- The draft is deliberately tied to the immutable tool-result message that
-- proposed it.  It is not a report row and therefore cannot make a clinical
-- record change by itself.
CREATE TABLE IF NOT EXISTS chat_draft_versions (
    id                     TEXT PRIMARY KEY NOT NULL,
    tool_result_message_id TEXT NOT NULL REFERENCES chat_messages(id) ON DELETE CASCADE,
    version_number         INTEGER NOT NULL,
    content                TEXT NOT NULL,
    origin                 TEXT NOT NULL CHECK (origin IN ('ai', 'manual')),
    claim_resolutions_json TEXT NOT NULL DEFAULT '[]',
    created_at             TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(tool_result_message_id, version_number)
);

CREATE INDEX IF NOT EXISTS idx_chat_draft_versions_message
    ON chat_draft_versions(tool_result_message_id, version_number ASC);
