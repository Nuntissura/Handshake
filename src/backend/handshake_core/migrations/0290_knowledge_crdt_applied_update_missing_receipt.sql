-- WP-KERNEL-009 MT-074 V1 FAIL remediation
-- (validator finding ADVERSARIAL_CODE_REVIEWER re-submission).
-- Affected MT: MT-074 (AiEditProposalReviewFlow).
--
-- Spec anchor: 02-system-architecture.md 2.3.13.11 — an approved AI edit
-- proposal binds to the kernel_crdt_updates row that applied it. The 0192
-- binding hardened "applied content must hash to the approved diff", but the
-- binder still stamped the row purely on the hash match: it never proved a REAL
-- kernel_crdt_updates row existed for the cited update_id. An approved diff hash
-- could therefore be bound to an update_id that was never pushed (or to a row
-- whose stored content differs from what was presented), and the authority trail
-- would claim "approved + applied" against a non-existent / inconsistent update.
--
-- The Rust binder (kernel/crdt/ai_edit_proposal.rs::apply_approved_ai_edit) now
-- looks up the kernel_crdt_updates row by the proposal's
-- (workspace_id, document_id, crdt_document_id, applied_update_id) and verifies
-- its persisted update_sha256 equals the presented content hash BEFORE binding.
-- When no matching/consistent row exists it refuses the binding and emits a
-- durable 'ai_edit_applied_update_missing' denial receipt. This migration widens
-- the denial-receipt receipt_kind allow-list to admit that new kind (additive;
-- the prior CHECK from 0192 cannot be re-run for already-migrated databases, so
-- it is replaced here with the superset).

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
