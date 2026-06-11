-- WP-KERNEL-009 MT-085/094 hardening (#8) rollback (MT-063 RollbackAndRepairMigrations).
-- Reverse-order rollback runs this before 0162.down / 0164.down, so both
-- tables still exist; relax the NOT NULL pins back to the original nullable
-- columns. The synthetic backfill ledger event (if one was minted on the way
-- up) is intentionally NOT deleted: kernel_event_ledger is append-only
-- authority and rows may still reference it.

ALTER TABLE knowledge_ingestion_repair_queue
    ALTER COLUMN enqueue_event_id DROP NOT NULL;

ALTER TABLE knowledge_ingestion_receipts
    ALTER COLUMN receipt_event_id DROP NOT NULL;
