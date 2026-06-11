-- WP-KERNEL-009 authority-hardening #5 (ADVERSARIAL_CODE_REVIEWER-20260611-PM).
-- Affected MT: MT-074 (AiEditProposalReviewFlow).
-- Spec anchor: 02-system-architecture.md 2.3.13.11 — AI edit proposals carry a
-- diff_sha256; approval/promotion flips the row but NOTHING bound the update
-- subsequently pushed to the document back to the approved diff hash. An
-- operator could approve diff A and a model could then push unrelated content;
-- the authority trail claimed "approved + applied" with no integrity link.
--
-- This migration binds applied <-> approved at the schema level:
--   * applied_update_id      — the kernel_crdt_updates.update_id that applied
--                              the approved edit;
--   * applied_update_sha256  — the content hash of that applied update.
-- A CHECK enforces (a) the two columns are set together, (b) they may be set
-- only on an approved/promoted row, and (c) applied_update_sha256 EQUALS the
-- proposal's diff_sha256 — a push whose content does not hash to the approved
-- diff can never be recorded as the applied edit. The Rust binder
-- (kernel/crdt/ai_edit_proposal.rs::bind_applied_ai_edit_update) refuses a
-- mismatch with a durable 'ai_edit_applied_mismatch' denial receipt; this
-- CHECK is the schema backstop.
--
-- It also widens the 0150 denial-receipt receipt_kind CHECK to allow the new
-- 'ai_edit_applied_mismatch' kind (additive; the original 0150 CHECK cannot be
-- re-run for already-migrated databases, so it is replaced here).

ALTER TABLE knowledge_crdt_ai_edit_proposals
    ADD COLUMN IF NOT EXISTS applied_update_id TEXT,
    ADD COLUMN IF NOT EXISTS applied_update_sha256 TEXT;

ALTER TABLE knowledge_crdt_ai_edit_proposals
    DROP CONSTRAINT IF EXISTS chk_knowledge_crdt_ai_edit_proposals_applied;

ALTER TABLE knowledge_crdt_ai_edit_proposals
    ADD CONSTRAINT chk_knowledge_crdt_ai_edit_proposals_applied
    CHECK (
        -- not yet applied
        (applied_update_id IS NULL AND applied_update_sha256 IS NULL)
        -- applied: both set, only on approved/promoted rows, and the applied
        -- content hash MUST equal the approved diff hash.
        OR (
            applied_update_id IS NOT NULL
            AND applied_update_sha256 IS NOT NULL
            AND applied_update_sha256 ~ '^[0-9a-f]{64}$'
            AND review_state IN ('approved', 'promoted')
            AND applied_update_sha256 = diff_sha256
        )
    );

-- Widen the denial-receipt kind allow-list (replaces the 0150 inline CHECK).
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
