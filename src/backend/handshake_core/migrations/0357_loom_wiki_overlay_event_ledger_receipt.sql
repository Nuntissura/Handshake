-- WP-KERNEL-012 E3 MT-025 WikiPageProjectionOverlay — durable EventLedger
-- receipts for wiki projection overlay mutations.
--
-- FAIL_V2 remediation: the wiki overlay POST (add_loom_wiki_overlay) persisted
-- the overlay row directly WITHOUT a durable EventLedger business-event receipt,
-- so a committed overlay annotation could lack durable evidence. This migration
-- adds a receipt column on loom_wiki_overlays that references the append-only
-- kernel_event_ledger. The storage layer now appends the KNOWLEDGE_LOOM_WIKI_MUTATED
-- event and inserts the overlay row in ONE PostgreSQL transaction; the foreign key
-- makes it schema-impossible for a committed overlay row to reference a receipt
-- that is not in the ledger. Existing rows keep NULL (legacy).

ALTER TABLE loom_wiki_overlays
    ADD COLUMN IF NOT EXISTS event_ledger_event_id TEXT
        REFERENCES kernel_event_ledger(event_id);

CREATE INDEX IF NOT EXISTS idx_loom_wiki_overlays_event_ledger_event_id
    ON loom_wiki_overlays (event_ledger_event_id);
