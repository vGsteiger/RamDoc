-- Add scheduled_time column to sessions table for calendar view support
-- This allows sessions to have a specific time (with date) in addition to the date-only session_date
ALTER TABLE sessions ADD COLUMN scheduled_time TEXT;

-- Create index for calendar queries filtering by scheduled_time
CREATE INDEX IF NOT EXISTS idx_sessions_scheduled_time ON sessions(scheduled_time);
