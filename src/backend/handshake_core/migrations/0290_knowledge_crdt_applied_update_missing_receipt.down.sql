-- Down: WP-KERNEL-009 MT-074 V1 FAIL remediation
-- (applied-update-missing denial kind). Restore the post-0192 receipt_kind
-- allow-list (without 'ai_edit_applied_update_missing').
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
        'ai_edit_applied_mismatch'
    ));
