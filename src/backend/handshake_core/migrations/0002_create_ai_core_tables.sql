-- AI core tables

CREATE TABLE IF NOT EXISTS ai_jobs (
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
    is_pinned INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS workflow_runs (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL,
    status TEXT NOT NULL,
    last_heartbeat TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (job_id) REFERENCES ai_jobs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ai_jobs_gc ON ai_jobs(status, created_at, is_pinned);
