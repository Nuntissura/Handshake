-- Harden AI Job model: enforce non-null metrics and preserve enum mapping
-- Implements WP-1-AI-Job-Model-v3 (v02.92 A2.6.6.2.8)

PRAGMA foreign_keys = OFF;

CREATE TABLE ai_jobs_new (
    id TEXT PRIMARY KEY NOT NULL,
    trace_id TEXT NOT NULL,
    workflow_run_id TEXT,
    job_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    status_reason TEXT NOT NULL DEFAULT 'queued',
    error_message TEXT,
    protocol_id TEXT NOT NULL,
    profile_id TEXT NOT NULL,
    capability_profile_id TEXT NOT NULL,
    access_mode TEXT NOT NULL,
    safety_mode TEXT NOT NULL,
    entity_refs TEXT NOT NULL DEFAULT '[]',
    planned_operations TEXT NOT NULL DEFAULT '[]',
    metrics TEXT NOT NULL DEFAULT '{"duration_ms":0,"total_tokens":0,"input_tokens":0,"output_tokens":0,"tokens_planner":0,"tokens_executor":0,"entities_read":0,"entities_written":0,"validators_run_count":0}',
    job_inputs TEXT,
    job_outputs TEXT,
    is_pinned INTEGER NOT NULL DEFAULT 0, -- Re-add is_pinned column [HSK-GC-002]
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO ai_jobs_new (
    id,
    trace_id,
    workflow_run_id,
    job_kind,
    status,
    status_reason,
    error_message,
    protocol_id,
    profile_id,
    capability_profile_id,
    access_mode,
    safety_mode,
    entity_refs,
    planned_operations,
    metrics,
    job_inputs,
    job_outputs,
    is_pinned, -- Include is_pinned
    created_at,
    updated_at
)
SELECT
    id,
    COALESCE(trace_id, id) as trace_id,
    workflow_run_id,
    job_kind,
    status,
    COALESCE(status_reason, 'queued'),
    error_message,
    protocol_id,
    profile_id,
    capability_profile_id,
    access_mode,
    safety_mode,
    COALESCE(entity_refs, '[]'),
    COALESCE(planned_operations, '[]'),
    CASE
        WHEN metrics IS NULL THEN '{"duration_ms":0,"total_tokens":0,"input_tokens":0,"output_tokens":0,"tokens_planner":0,"tokens_executor":0,"entities_read":0,"entities_written":0,"validators_run_count":0}'
        WHEN metrics NOT LIKE '%validators_run_count%' THEN '{"duration_ms":0,"total_tokens":0,"input_tokens":0,"output_tokens":0,"tokens_planner":0,"tokens_executor":0,"entities_read":0,"entities_written":0,"validators_run_count":0}'
        ELSE metrics
    END as metrics,
    job_inputs,
    job_outputs,
    COALESCE(is_pinned, 0) as is_pinned, -- Carry over is_pinned
    created_at,
    updated_at
FROM ai_jobs;

DROP TABLE ai_jobs;
ALTER TABLE ai_jobs_new RENAME TO ai_jobs;

-- Re-create the index lost during table recreation
CREATE INDEX IF NOT EXISTS idx_ai_jobs_gc ON ai_jobs(status, created_at, is_pinned);

PRAGMA foreign_keys = ON;