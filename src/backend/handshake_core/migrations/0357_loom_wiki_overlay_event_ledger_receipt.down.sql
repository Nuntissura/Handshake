-- Down: WP-KERNEL-012 E3 MT-025 wiki overlay EventLedger receipt column.
DROP INDEX IF EXISTS idx_loom_wiki_overlays_event_ledger_event_id;
ALTER TABLE loom_wiki_overlays DROP COLUMN IF EXISTS event_ledger_event_id;
