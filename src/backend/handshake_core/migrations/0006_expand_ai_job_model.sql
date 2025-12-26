-- Expand ai_jobs with normative fields for WP-1-AI-Job-Model-v2

ALTER TABLE ai_jobs ADD COLUMN trace_id TEXT;
ALTER TABLE ai_jobs ADD COLUMN workflow_run_id TEXT;
ALTER TABLE ai_jobs ADD COLUMN status_reason TEXT;
ALTER TABLE ai_jobs ADD COLUMN entity_refs TEXT;
ALTER TABLE ai_jobs ADD COLUMN planned_operations TEXT;
ALTER TABLE ai_jobs ADD COLUMN metrics TEXT;

-- Backfill existing rows with sensible defaults
UPDATE ai_jobs SET trace_id = COALESCE(trace_id, id);
UPDATE ai_jobs SET status_reason = COALESCE(status_reason, 'queued');
UPDATE ai_jobs SET entity_refs = COALESCE(entity_refs, '[]');
UPDATE ai_jobs SET planned_operations = COALESCE(planned_operations, '[]');
UPDATE ai_jobs
SET metrics = COALESCE(
    metrics,
    '{"duration_ms":0,"total_tokens":0,"input_tokens":0,"output_tokens":0,"entities_read":0,"entities_written":0}'
);
