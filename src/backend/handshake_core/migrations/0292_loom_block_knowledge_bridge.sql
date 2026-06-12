-- WP-KERNEL-009 MT-177 LoomBlockKnowledgeBridge.
--
-- Master Spec anchors:
--   * 2.2.1.14 LoomBlock Entity / 10.12 #4 — the LoomBlock is the atom of the
--     Loom library surface.
--   * 2.3.13.11 KnowledgeEntity — `loom_block` is already a first-class
--     entity_kind in knowledge_entities (migration 0135). The ProjectKnowledge
--     Index is the authority graph; a LoomBlock participates in it as an entity.
--   * 10.12 #9.1.1 "WP-KERNEL-009 authority supersession" — for WP-009 the
--     ONLY authority path is PostgreSQL + EventLedger. The Loom MVP rode on its
--     own loom_blocks/loom_edges tables; this bridge makes every LoomBlock
--     resolve to a knowledge_entities row (ProjectKnowledgeIndex authority) and
--     carry a KNOWLEDGE_LOOM_BLOCK_INDEXED EventLedger receipt, so Loom is no
--     longer a parallel store. No SQLite path exists in the compiled runtime
--     (storage/mod.rs declares no `sqlite` module); this bridge is the positive
--     authority binding for the supersession.
--
-- This table is the queryable, idempotent authority link:
--   loom_blocks.block_id  <->  knowledge_entities.entity_id  (+ EventLedger id)
--
-- It is a thin, re-derivable bridge: the knowledge entity's natural identity is
-- (workspace_id, 'loom_block', block_id) (0135 uq_knowledge_entities_identity),
-- so re-bridging the same block upserts the same entity and updates this row in
-- place rather than duplicating. Deleting a LoomBlock cascades the bridge row;
-- the knowledge entity is retired by the service layer (entities are retired,
-- not hard-deleted, per 0135 lifecycle_state) so detection history survives.

CREATE TABLE IF NOT EXISTS loom_block_knowledge_bridge (
    -- One bridge row per LoomBlock (the block is the natural key).
    block_id TEXT PRIMARY KEY
        REFERENCES loom_blocks(block_id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL
        REFERENCES workspaces(id) ON DELETE CASCADE,
    -- The ProjectKnowledgeIndex authority handle for this block.
    entity_id TEXT NOT NULL
        REFERENCES knowledge_entities(entity_id) ON DELETE CASCADE,
    -- EventLedger receipt proving the bridge/index operation
    -- (KNOWLEDGE_LOOM_BLOCK_INDEXED). NOT NULL: a bridged block MUST have an
    -- EventLedger receipt (10.12 #9.1.1 — EventLedger is authority). FK targets
    -- the kernel event ledger, matching every knowledge_* receipt column
    -- (e.g. migrations 0132/0133/0137/0139).
    index_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- A knowledge entity bridges at most one LoomBlock in a workspace: the
    -- entity_key IS the block_id, so this is a 1:1 binding. Enforce it so a
    -- stray re-key can never fan one entity onto two blocks.
    CONSTRAINT uq_loom_block_knowledge_bridge_entity
        UNIQUE (entity_id)
);

CREATE INDEX IF NOT EXISTS idx_loom_block_knowledge_bridge_workspace
    ON loom_block_knowledge_bridge (workspace_id);

CREATE INDEX IF NOT EXISTS idx_loom_block_knowledge_bridge_entity
    ON loom_block_knowledge_bridge (entity_id);
