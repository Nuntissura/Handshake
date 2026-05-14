-- WP-KERNEL-001-Event-Ledger-Session-Broker-v1 rollback

DROP INDEX IF EXISTS idx_kernel_session_queue_lease;
DROP INDEX IF EXISTS idx_kernel_session_queue_state_available;
DROP INDEX IF EXISTS idx_kernel_session_queue_task;
DROP TABLE IF EXISTS kernel_session_queue;

