-- WP-1-Postgres-MCP-Durable-Progress-v1: portable MCP durable mapping store

CREATE TABLE IF NOT EXISTS ai_job_mcp_fields (
    job_id TEXT PRIMARY KEY NOT NULL REFERENCES ai_jobs(id) ON DELETE CASCADE,
    mcp_server_id TEXT,
    mcp_call_id TEXT,
    mcp_progress_token TEXT
);

-- Reverse lookup: progress_token -> job_id (1:1 when token is set; NULL allowed).
CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_job_mcp_fields_progress_token ON ai_job_mcp_fields(mcp_progress_token);

