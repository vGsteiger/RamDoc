-- Add clinical_summary field to sessions table
-- This field stores the LLM-generated structured clinical summary for each session

ALTER TABLE sessions ADD COLUMN clinical_summary TEXT;
