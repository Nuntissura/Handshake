-- WP-KERNEL-001-Event-Ledger-Session-Broker-v1 rollback

DROP INDEX IF EXISTS idx_kernel_event_ledger_created_at;
DROP INDEX IF EXISTS idx_kernel_event_ledger_causation;
DROP INDEX IF EXISTS idx_kernel_event_ledger_correlation;
DROP INDEX IF EXISTS idx_kernel_event_ledger_payload_hash;
DROP INDEX IF EXISTS idx_kernel_event_ledger_event_type;
DROP INDEX IF EXISTS idx_kernel_event_ledger_aggregate_replay;
DROP INDEX IF EXISTS idx_kernel_event_ledger_session;
DROP INDEX IF EXISTS idx_kernel_event_ledger_task;
DROP INDEX IF EXISTS idx_kernel_event_ledger_idempotency;
DROP INDEX IF EXISTS idx_kernel_event_ledger_sequence;
DROP TABLE IF EXISTS kernel_event_ledger;
