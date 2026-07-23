-- Rollback for 0360_preference_records.sql (WP-KERNEL-012 MT-072).
DROP INDEX IF EXISTS preference_change_receipts_history_idx;
DROP TABLE IF EXISTS preference_change_receipts;
DROP INDEX IF EXISTS preference_records_scope_idx;
DROP TABLE IF EXISTS preference_records;
