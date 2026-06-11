-- WP-KERNEL-009 MT-085/094 hardening (#8): make the EventLedger receipt refs
-- NOT NULL so "every ingestion mutation has a ledger receipt" is
-- schema-enforced, not merely a code convention.
--
-- Master Spec anchor: 2.3.13.11 backend-navigation receipt law -- every
-- engine mutation must leave an EventLedger receipt carrying actor, session,
-- and correlation ids. The engine
-- (knowledge_ingestion::engine::persist_attempt) ALWAYS mints the ledger
-- event first and passes it into both writes:
--   * record_extraction_receipt(receipt, Some(&event_id))   -> receipts.receipt_event_id
--   * enqueue_repair(NewRepairEntry { enqueue_event_id: Some(event_id) }) -> repair.enqueue_event_id
-- so the code invariant is already "never NULL". But 0162/0164 declared both
-- columns NULLABLE, so a raw-SQL writer (migration, ops script, future code
-- path) could insert a receipt or repair entry with no ledger receipt and the
-- schema would accept it -- a silent break of the receipt law.
--
-- Gap closed (HARDENING-20260611, MEDIUM): pin both refs NOT NULL.
--   * knowledge_ingestion_receipts.receipt_event_id  (0162, MT-085)
--   * knowledge_ingestion_repair_queue.enqueue_event_id (0164, MT-094)
-- The existing FKs to kernel_event_ledger(event_id) (ON UPDATE RESTRICT,
-- ON DELETE RESTRICT) stay; NOT NULL closes the "no receipt at all" hole the
-- FK could not (a NULL passes an FK).
--
-- Backfill / guard for any pre-existing NULL row: both refs FK to
-- kernel_event_ledger with ON DELETE RESTRICT, so a NULL cannot be backfilled
-- with an arbitrary sentinel id (that would violate the FK). When -- and ONLY
-- when -- a legacy NULL row exists, this migration mints ONE synthetic
-- backfill ledger event (all kernel_event_ledger NOT NULL columns satisfied)
-- and repoints the NULL rows at it, so NOT NULL + FK both hold without data
-- loss. Fresh schemas and this WP's own deployments have zero such rows (the
-- ingestion tables are introduced by 0160+ in this same WP), so the synthetic
-- event is normally never created.
--
-- Migration range note: 0210-0219 is the WP-KERNEL-009 MT-091/094 DB-guard
-- hardening band; 0162/0164 are frozen, so this ships additively.

DO $$
DECLARE
    needs_backfill BOOLEAN;
    backfill_event_id TEXT := 'KEVT-backfill-0211-ingestion-receipt-notnull';
BEGIN
    SELECT
        EXISTS (SELECT 1 FROM knowledge_ingestion_receipts WHERE receipt_event_id IS NULL)
        OR EXISTS (SELECT 1 FROM knowledge_ingestion_repair_queue WHERE enqueue_event_id IS NULL)
    INTO needs_backfill;

    IF needs_backfill THEN
        -- Mint a single, idempotent synthetic backfill ledger event to satisfy
        -- the FK for every legacy NULL row. Real ledger payload columns are
        -- all populated so kernel_event_ledger's own NOT NULLs are honored.
        INSERT INTO kernel_event_ledger (
            event_id, event_version, kernel_task_run_id, session_run_id,
            aggregate_type, aggregate_id, idempotency_key, event_type,
            actor_kind, actor_id, payload_hash, source_component, payload
        )
        VALUES (
            backfill_event_id, 'v1', 'KTR-MIGRATION-0211', 'SR-MIGRATION-0211',
            'knowledge_ingestion_backfill', backfill_event_id,
            'idem-' || backfill_event_id, 'ValidationRecorded',
            'system', 'migration-0211',
            repeat('0', 64), 'knowledge_ingestion',
            jsonb_build_object(
                'kind', 'receipt_event_backfill',
                'migration', '0211_knowledge_ingestion_receipt_event_not_null',
                'note', 'synthetic ledger event minted to backfill pre-existing NULL receipt refs before SET NOT NULL'
            )
        )
        ON CONFLICT (event_id) DO NOTHING;

        UPDATE knowledge_ingestion_receipts
            SET receipt_event_id = backfill_event_id
            WHERE receipt_event_id IS NULL;

        UPDATE knowledge_ingestion_repair_queue
            SET enqueue_event_id = backfill_event_id
            WHERE enqueue_event_id IS NULL;
    END IF;
END $$;

ALTER TABLE knowledge_ingestion_receipts
    ALTER COLUMN receipt_event_id SET NOT NULL;

ALTER TABLE knowledge_ingestion_repair_queue
    ALTER COLUMN enqueue_event_id SET NOT NULL;
