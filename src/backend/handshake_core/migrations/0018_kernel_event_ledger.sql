-- WP-KERNEL-001-Event-Ledger-Session-Broker-v1:
-- Kernel V1 append-only EventLedger authority rows.

CREATE TABLE IF NOT EXISTS kernel_event_ledger (
    event_id TEXT PRIMARY KEY NOT NULL,
    event_sequence BIGSERIAL NOT NULL,
    event_version TEXT NOT NULL,
    kernel_task_run_id TEXT NOT NULL,
    session_run_id TEXT NOT NULL,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    event_type TEXT NOT NULL,
    actor_kind TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    causation_id TEXT,
    correlation_id TEXT,
    payload_hash TEXT NOT NULL,
    source_component TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_kernel_event_ledger_sequence
    ON kernel_event_ledger (event_sequence);

CREATE UNIQUE INDEX IF NOT EXISTS idx_kernel_event_ledger_idempotency
    ON kernel_event_ledger (idempotency_key);

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_task
    ON kernel_event_ledger (kernel_task_run_id);

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_session
    ON kernel_event_ledger (session_run_id);

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_aggregate_replay
    ON kernel_event_ledger (aggregate_type, aggregate_id, event_sequence);

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_event_type
    ON kernel_event_ledger (event_type);

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_payload_hash
    ON kernel_event_ledger (payload_hash);

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_correlation
    ON kernel_event_ledger (correlation_id);

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_causation
    ON kernel_event_ledger (causation_id);

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_created_at
    ON kernel_event_ledger (created_at);
