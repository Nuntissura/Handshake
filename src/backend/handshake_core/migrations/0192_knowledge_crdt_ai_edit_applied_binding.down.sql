-- Down: WP-KERNEL-009 authority-hardening #5 AI-edit applied binding.
ALTER TABLE knowledge_crdt_ai_edit_proposals
    DROP CONSTRAINT IF EXISTS chk_knowledge_crdt_ai_edit_proposals_applied;
ALTER TABLE knowledge_crdt_ai_edit_proposals
    DROP COLUMN IF EXISTS applied_update_sha256,
    DROP COLUMN IF EXISTS applied_update_id;

-- Restore the original 0150 receipt_kind allow-list (without the new kind).
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
        'ai_edit_promotion_denied'
    ));
