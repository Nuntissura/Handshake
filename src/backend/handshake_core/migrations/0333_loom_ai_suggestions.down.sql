-- Down migration for 0333_loom_ai_suggestions (WP-KERNEL-009 MT-260).
-- Replay-safe: drops the AI Loom suggestion table, and restores the
-- denial-receipt allow-list to the pre-0333 (0290) superset. The up migration
-- does NOT register loom_ai_suggestions in knowledge_schema_registry (it is a
-- Loom-domain sibling, not a `knowledge_`-prefixed namespace table), so there
-- is no registry row to remove.

DROP INDEX IF EXISTS idx_loom_ai_suggestions_workspace;
DROP INDEX IF EXISTS idx_loom_ai_suggestions_job;
DROP INDEX IF EXISTS uq_loom_ai_suggestions_idempotency;
DROP TABLE IF EXISTS loom_ai_suggestions;

ALTER TABLE knowledge_crdt_denial_receipts
    DROP CONSTRAINT IF EXISTS knowledge_crdt_denial_receipts_receipt_kind_check;

ALTER TABLE knowledge_crdt_denial_receipts
    ADD CONSTRAINT knowledge_crdt_denial_receipts_receipt_kind_check
    CHECK (receipt_kind IN (
        'stale_draft_save',
        'concurrent_draft_fork',
        'ahead_of_head_save',
        'update_content_mismatch',
        'sequence_slot_race',
        'lease_write_denied',
        'index_run_slot_rejected',
        'graph_promotion_denied',
        'ai_edit_promotion_denied',
        'ai_edit_applied_mismatch',
        'ai_edit_applied_update_missing'
    ));
