-- WP-KERNEL-012 E3 MT-023 TagsTagHubs — durable EventLedger receipts.
--
-- FAIL_V2 remediation: tag mutations (POST/DELETE /loom/edges, edge_type='tag')
-- committed database changes before a best-effort Flight Recorder call whose
-- error was discarded, permitting a successful tag write with no durable
-- observability. This migration adds a receipt column on loom_edges that
-- references the append-only kernel_event_ledger. The storage layer now appends
-- the KNOWLEDGE_LOOM_TAG_MUTATED event and writes/deletes the edge row in ONE
-- PostgreSQL transaction; the Flight Recorder DuckDB mirror stays a best-effort
-- Tier-1 mirror, but the durable authority receipt is now atomic. The foreign
-- key makes it schema-impossible for a new edge row to reference a receipt that
-- is not in the ledger. Existing rows keep NULL (legacy).

ALTER TABLE loom_edges
    ADD COLUMN IF NOT EXISTS event_ledger_event_id TEXT
        REFERENCES kernel_event_ledger(event_id);

CREATE INDEX IF NOT EXISTS idx_loom_edges_event_ledger_event_id
    ON loom_edges (event_ledger_event_id);
