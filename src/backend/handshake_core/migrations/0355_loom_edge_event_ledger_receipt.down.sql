-- Down: WP-KERNEL-012 E3 MT-023 tag-edge EventLedger receipt column.
DROP INDEX IF EXISTS idx_loom_edges_event_ledger_event_id;
ALTER TABLE loom_edges DROP COLUMN IF EXISTS event_ledger_event_id;
