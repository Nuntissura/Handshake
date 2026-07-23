-- Down: WP-KERNEL-012 E3 MT-022 folder EventLedger receipt columns.
DROP INDEX IF EXISTS idx_loom_folder_members_event_ledger_event_id;
DROP INDEX IF EXISTS idx_loom_folders_event_ledger_event_id;
ALTER TABLE loom_folder_members DROP COLUMN IF EXISTS event_ledger_event_id;
ALTER TABLE loom_folders DROP COLUMN IF EXISTS event_ledger_event_id;
