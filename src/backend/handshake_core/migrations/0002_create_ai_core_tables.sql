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
    created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE TRIGGER ai_jobs_updated_at
AFTER UPDATE ON ai_jobs
FOR EACH ROW
BEGIN
    UPDATE ai_jobs SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = OLD.id;
END;

CREATE TABLE workflow_runs (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    FOREIGN KEY (job_id) REFERENCES ai_jobs(id) ON DELETE CASCADE
);

CREATE TRIGGER workflow_runs_updated_at
AFTER UPDATE ON workflow_runs
FOR EACH ROW
BEGIN
    UPDATE workflow_runs SET updated_at = strftime('%Y-%m-%d %H:%M:%f', 'now') WHERE id = OLD.id;
END;