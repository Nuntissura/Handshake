-- WP-KERNEL-012 E3 MT-024 PinsFavoritesBacklinksUnlinked — durable EventLedger
-- receipts for block pin/favorite mutations.
--
-- FAIL_V2 remediation: pin/favorite mutations in the live Loom API persisted
-- WITHOUT a corresponding transactional EventLedger append, and pin removal was
-- a two-call sequence (PUT /pin-order(null) THEN PATCH {pinned:false}) that could
-- clear pin_order before the second request failed — leaving partial persisted
-- state with no reliable recovery evidence. This migration adds a receipt column
-- on loom_blocks that references the append-only kernel_event_ledger. The storage
-- layer now appends the KNOWLEDGE_LOOM_BLOCK_MUTATED event and performs the block
-- pin/favorite/pin-order/pin-removal write in ONE PostgreSQL transaction; the
-- foreign key makes it schema-impossible for a mutated block row to reference a
-- receipt that is not in the ledger. Existing rows keep NULL (legacy).

ALTER TABLE loom_blocks
    ADD COLUMN IF NOT EXISTS event_ledger_event_id TEXT
        REFERENCES kernel_event_ledger(event_id);

CREATE INDEX IF NOT EXISTS idx_loom_blocks_event_ledger_event_id
    ON loom_blocks (event_ledger_event_id);
