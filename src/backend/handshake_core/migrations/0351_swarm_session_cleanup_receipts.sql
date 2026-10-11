CREATE TABLE IF NOT EXISTS swarm_session_cleanup_receipts (
    instance_id TEXT PRIMARY KEY,
    revision BIGINT NOT NULL CHECK (revision > 0),
    status TEXT NOT NULL CHECK (status IN ('cleanup_pending', 'teardown_succeeded', 'completed')),
    terminal_state TEXT NOT NULL,
    reason TEXT NOT NULL,
    exit_code INTEGER NOT NULL,
    last_error TEXT,
    record_json JSONB NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_swarm_session_cleanup_pending
    ON swarm_session_cleanup_receipts (updated_at_unix_ms)
    WHERE status <> 'completed';
