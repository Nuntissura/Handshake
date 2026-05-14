-- WP-KERNEL-001-Event-Ledger-Session-Broker-v1:
-- Durable Postgres queue and claim leases for Kernel V1 SessionBroker work.

CREATE TABLE IF NOT EXISTS kernel_session_queue (
    session_run_id TEXT PRIMARY KEY NOT NULL,
    kernel_task_run_id TEXT NOT NULL,
    adapter_id TEXT NOT NULL,
    state TEXT NOT NULL,
    claimed_by TEXT,
    lease_expires_at TIMESTAMP,
    attempt_count BIGINT NOT NULL DEFAULT 0,
    available_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_kernel_session_queue_task
    ON kernel_session_queue (kernel_task_run_id);

CREATE INDEX IF NOT EXISTS idx_kernel_session_queue_state_available
    ON kernel_session_queue (state, available_at);

CREATE INDEX IF NOT EXISTS idx_kernel_session_queue_lease
    ON kernel_session_queue (lease_expires_at);

