-- Add migration script here
CREATE TABLE ai_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    job_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    error_message TEXT,
    protocol_id TEXT NOT NULL,
    profile_id TEXT NOT NULL,
    capability_profile_id TEXT NOT NULL,
    access_mode TEXT NOT NULL,
    safety_mode TEXT NOT NULL,
    job_inputs TEXT,
    job_outputs TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE workflow_runs (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (job_id) REFERENCES ai_jobs(id) ON DELETE CASCADE
);
