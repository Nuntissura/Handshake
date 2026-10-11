-- Down: WP-1 MT-018 applied-binding update-identity foreign key.
-- Drops only the composite FK added by 0362. Migration 0192's
-- `chk_knowledge_crdt_ai_edit_proposals_applied` CHECK is untouched here
-- because 0362 never modified it (0192 is RETAINED, not superseded).
ALTER TABLE knowledge_crdt_ai_edit_proposals
    DROP CONSTRAINT IF EXISTS fk_knowledge_crdt_ai_edit_proposals_applied_update;
