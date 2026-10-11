-- WP-KERNEL-012 MT-033: durable canonical Atelier intake item -> Loom block identity.
--
-- Canvas placement accepts a Loom `placed_block_id`; it must never infer that
-- identity from an Atelier item id or from a test-only projection.  This table
-- is the authoritative cross-pillar relation returned by the Atelier API.

CREATE TABLE IF NOT EXISTS atelier_intake_item_loom_projection (
    item_id          UUID PRIMARY KEY
        REFERENCES atelier_intake_item(item_id) ON DELETE CASCADE,
    loom_block_id    TEXT NOT NULL UNIQUE
        REFERENCES loom_blocks(block_id) ON DELETE RESTRICT,
    workspace_id     TEXT NOT NULL
        REFERENCES workspaces(id) ON DELETE RESTRICT,
    linked_by        TEXT NOT NULL,
    linked_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_intake_item_loom_projection_linked_by
        CHECK (btrim(linked_by) = linked_by AND linked_by <> '')
);

CREATE INDEX IF NOT EXISTS idx_atelier_intake_item_loom_projection_workspace
    ON atelier_intake_item_loom_projection(workspace_id, item_id);
