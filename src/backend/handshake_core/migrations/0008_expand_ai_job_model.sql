-- Harden AI Job model: enforce non-null metrics and preserve enum mapping
-- Implements WP-1-AI-Job-Model-v3 (v02.92 A2.6.6.2.8)
-- Portable rebuild: avoid sqlite-only PRAGMA and keep Postgres-compatible DDL.

ALTER TABLE workflow_node_executions RENAME TO workflow_node_executions_old;
ALTER TABLE workflow_runs RENAME TO workflow_runs_old;
ALTER TABLE ai_jobs RENAME TO ai_jobs_old;

CREATE TABLE ai_jobs (
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

INSERT INTO ai_jobs (
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
FROM ai_jobs_old;

CREATE TABLE workflow_runs (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL,
    status TEXT NOT NULL,
    last_heartbeat TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (job_id) REFERENCES ai_jobs(id) ON DELETE CASCADE
);

INSERT INTO workflow_runs (
    id,
    job_id,
    status,
    last_heartbeat,
    created_at,
    updated_at
)
SELECT
    id,
    job_id,
    status,
    COALESCE(last_heartbeat, CURRENT_TIMESTAMP),
    created_at,
    updated_at
FROM workflow_runs_old;

CREATE TABLE workflow_node_executions (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_run_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    node_type TEXT NOT NULL,
    status TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    input_payload TEXT NULL,
    output_payload TEXT NULL,
    error_message TEXT NULL,
    started_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (workflow_run_id) REFERENCES workflow_runs(id) ON DELETE CASCADE
);

INSERT INTO workflow_node_executions (
    id,
    workflow_run_id,
    node_id,
    node_type,
    status,
    sequence,
    input_payload,
    output_payload,
    error_message,
    started_at,
    finished_at,
    created_at,
    updated_at
)
SELECT
    id,
    workflow_run_id,
    node_id,
    node_type,
    status,
    sequence,
    input_payload,
    output_payload,
    error_message,
    started_at,
    finished_at,
    created_at,
    updated_at
FROM workflow_node_executions_old;

-- Drop legacy indexes to avoid name collisions, then re-create on new tables.
DROP INDEX IF EXISTS idx_ai_jobs_gc;
DROP INDEX IF EXISTS idx_wne_run_sequence;
DROP INDEX IF EXISTS idx_wne_run_node;
DROP INDEX IF EXISTS idx_wne_status;

CREATE INDEX idx_ai_jobs_gc ON ai_jobs(status, created_at, is_pinned);
CREATE INDEX idx_wne_run_sequence ON workflow_node_executions (workflow_run_id, sequence);
CREATE INDEX idx_wne_run_node ON workflow_node_executions (workflow_run_id, node_id);
CREATE INDEX idx_wne_status ON workflow_node_executions (status);

DROP TABLE workflow_node_executions_old;
DROP TABLE workflow_runs_old;
DROP TABLE ai_jobs_old;
