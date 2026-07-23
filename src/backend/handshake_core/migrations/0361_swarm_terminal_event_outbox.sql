-- Durable, bounded spool for terminal swarm Flight Recorder events.
-- Producer acknowledgement occurs only after a row commits here; recorder
-- delivery deletes the row, while shutdown/failure leaves it recoverable.
CREATE TABLE IF NOT EXISTS swarm_terminal_event_outbox (
    event_id UUID PRIMARY KEY,
    event_jsonb JSONB NOT NULL,
    attempts BIGINT NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    last_error TEXT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_attempt_at_utc TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_swarm_terminal_event_outbox_oldest
    ON swarm_terminal_event_outbox (created_at_utc, event_id);
