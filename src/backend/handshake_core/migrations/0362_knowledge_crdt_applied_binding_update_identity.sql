-- WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1 MT-018.
-- Affected MTs: MT-018 (this migration), MT-002/MT-004/MT-005/MT-009 (the
-- ModelLane CRDT admission proofs it unblocks), MT-074 (the original
-- applied-binding design in 0192/0290).
--
-- PROBLEM. The approved-proposal -> applied-update -> CRDT-message authority
-- trail conflated two different hash spaces:
--
--   * `knowledge_crdt_ai_edit_proposals.applied_update_sha256` is the hash of
--     the approved JSON DIFF
--     (`sha256(serde_json::to_vec(applied_diff))`, computed in
--     `kernel/crdt/ai_edit_proposal.rs::apply_approved_ai_edit`), and migration
--     0192 pins it to `diff_sha256`.
--   * `kernel_crdt_updates.update_sha256` is the hash of the Yjs v1 BINARY
--     update; `swarm_orchestration/model_lane.rs::resolve_model_lane_crdt_authority_tx`
--     recomputes it over `update_bytes` and additionally requires
--     `Update::decode_v1(update_bytes)` to succeed.
--
-- Two independent sites required those two hashes to be EQUAL:
--   (1) the binder, which refused to stamp the applied binding unless the
--       persisted `kernel_crdt_updates.update_sha256` equalled the JSON-diff
--       hash, and
--   (2) the ModelLane resolver, which refused a `crdt_proposal_ref` unless
--       `applied_update_sha256 = kernel_crdt_updates.update_sha256`.
-- Short of a SHA-256 collision the only way to satisfy either was to persist
-- the JSON diff bytes AS the CRDT update bytes, which then fail
-- `Update::decode_v1`. Result: no honest Proposal-kind CRDT-bearing
-- ModelLaneMessage could ever be minted or admitted.
--
-- 0192 DISPOSITION: RETAINED, with clarified semantics. The CHECK clause
-- `applied_update_sha256 = diff_sha256` (0192:44) is CORRECT and stays exactly
-- as written. `applied_update_sha256` is a DIFF-hash column and was never a Yjs
-- hash; the defect was in the two Rust comparison sites, not in the schema. The
-- clause is now understood as an INTERNAL-CONSISTENCY invariant on the
-- proposal's own approved content, and the ModelLane resolver compares
-- `applied_update_sha256` against the proposal's own `diff_sha256` rather than
-- against `kernel_crdt_updates.update_sha256`. 0192 is NOT edited in place and
-- is NOT superseded. (Supersession-note precedent: 0290:1-22.)
--
-- WHAT THIS MIGRATION ADDS. The 0192 design assumed the applied binding pointed
-- at a real `kernel_crdt_updates` row but never made the database enforce it;
-- 0290 moved that check into Rust only. Yjs update identity is carried solely
-- by `applied_update_id`, so this migration adds the composite FOREIGN KEY that
-- makes the identity binding a schema invariant:
--
--   (workspace_id, document_id, crdt_document_id, applied_update_id)
--     REFERENCES kernel_crdt_updates (workspace_id, document_id,
--                                     crdt_document_id, update_id)
--
-- That parent tuple is exactly the `kernel_crdt_updates` PRIMARY KEY (0020:24),
-- so the reference is unique and viable. Under the default MATCH SIMPLE
-- semantics a row with `applied_update_id IS NULL` (not yet applied) stays
-- unconstrained, which preserves the "not yet applied" arm of the 0192 CHECK.
-- ON UPDATE/DELETE RESTRICT matches the 0154 convention and is consistent with
-- migration 0358, which already rejects UPDATE/DELETE on `kernel_crdt_updates`.
--
-- FAIL-CLOSED NOTE: this constraint is added VALIDATED. If a pre-existing
-- database holds an applied binding whose cited update row does not exist, the
-- migration fails loudly rather than silently admitting a phantom authority
-- trail. That is the intended posture for CRDT authority data.
--
-- No trigger exists on `knowledge_crdt_ai_edit_proposals` (only migrations 0154
-- and 0192 touch the table, and neither creates one), so the new constraint has
-- no trigger interaction.
--
-- Constraint-replacement idiom follows 0192:29-33 / 0290:24-28.

ALTER TABLE knowledge_crdt_ai_edit_proposals
    DROP CONSTRAINT IF EXISTS fk_knowledge_crdt_ai_edit_proposals_applied_update;

ALTER TABLE knowledge_crdt_ai_edit_proposals
    ADD CONSTRAINT fk_knowledge_crdt_ai_edit_proposals_applied_update
    FOREIGN KEY (workspace_id, document_id, crdt_document_id, applied_update_id)
    REFERENCES kernel_crdt_updates (workspace_id, document_id, crdt_document_id, update_id)
    ON UPDATE RESTRICT ON DELETE RESTRICT;
