-- WP-KERNEL-009 MT-185 WikiPageEditableOverlay.
--
-- Master Spec anchor: §10.12 §9.1.1 / MT-185 — an operator may annotate a
-- generated wiki page WITHOUT the generated projection text becoming canonical
-- authority. The projection (knowledge_wiki_projections) stays regenerable and
-- NEVER authority; the operator's annotations live here as their OWN authority
-- rows. Editing an overlay never promotes the projection. Deleting the
-- projection is independent of these overlays (and vice versa).
--
-- This is the editable-overlay half of the projection-never-authority boundary
-- (MT-184 generates the projection; MT-185 lets the operator annotate it; the
-- authority split is: projection = display/regenerable, overlay = authored).

CREATE TABLE IF NOT EXISTS loom_wiki_overlays (
    overlay_id TEXT PRIMARY KEY,
    -- The projection this annotation is attached to. ON DELETE CASCADE: if the
    -- projection is regenerated under a new id or removed, its overlays go too;
    -- but an overlay carries no FK FROM any authority record, so authority is
    -- never affected by overlay churn.
    projection_id TEXT NOT NULL
        REFERENCES knowledge_wiki_projections(projection_id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- The operator's annotation (authority — this IS canonical).
    annotation TEXT NOT NULL,
    -- Optional free-form anchor into the projection (e.g. a source block id);
    -- NEVER an absolute filesystem path (MT-152 typed-reference discipline).
    anchor TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_loom_wiki_overlays_annotation
        CHECK (btrim(annotation) <> '')
);

CREATE INDEX IF NOT EXISTS idx_loom_wiki_overlays_projection
    ON loom_wiki_overlays (projection_id);

CREATE INDEX IF NOT EXISTS idx_loom_wiki_overlays_workspace
    ON loom_wiki_overlays (workspace_id);
