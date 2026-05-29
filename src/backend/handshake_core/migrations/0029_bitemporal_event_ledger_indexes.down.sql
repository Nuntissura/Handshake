-- WP-KERNEL-004 MT-157 rollback.

DROP INDEX IF EXISTS idx_kernel_event_ledger_memory_bitemporal_manifest;
DROP INDEX IF EXISTS idx_kernel_event_ledger_memory_bitemporal_recorded;
DROP INDEX IF EXISTS idx_kernel_event_ledger_memory_bitemporal_world;
