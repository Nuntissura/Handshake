-- WP-KERNEL-004 MT-007 rollback

DROP INDEX IF EXISTS idx_kernel_process_lifecycle_engine_started;
DROP INDEX IF EXISTS idx_kernel_process_lifecycle_parent_session_started;
DROP INDEX IF EXISTS idx_kernel_process_lifecycle_adapter_spawned;
DROP INDEX IF EXISTS idx_kernel_process_lifecycle_wp_spawned;
DROP TABLE IF EXISTS kernel_process_lifecycle;
