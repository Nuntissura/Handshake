-- Down: WP-KERNEL-012 E3 MT-024 block pin/favorite EventLedger receipt column.
DROP INDEX IF EXISTS idx_loom_blocks_event_ledger_event_id;
ALTER TABLE loom_blocks DROP COLUMN IF EXISTS event_ledger_event_id;
