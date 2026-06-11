-- WP-KERNEL-005 MT-094: append-only pose context state.
-- Tracks blank, single-image, character-linked, and collection-linked editor
-- context switches without deleting rigs, source media, or linked collections.

CREATE TABLE IF NOT EXISTS atelier_pose_context_state (
    context_id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    state_seq             BIGSERIAL UNIQUE,
    workspace_ref         TEXT NOT NULL,
    kind                  TEXT NOT NULL CHECK (
        kind IN ('blank','single_image','character_linked','collection_linked')
    ),
    source_asset_id       UUID REFERENCES atelier_media_asset(asset_id) ON DELETE RESTRICT,
    character_internal_id UUID REFERENCES atelier_character(internal_id) ON DELETE RESTRICT,
    collection_id         UUID REFERENCES atelier_collection(collection_id) ON DELETE RESTRICT,
    selected_rig_id       UUID REFERENCES atelier_pose_rig(rig_id) ON DELETE RESTRICT,
    requested_by          TEXT NOT NULL,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_pose_context_state_workspace_ref CHECK (
        btrim(workspace_ref) = workspace_ref
        AND workspace_ref <> ''
        AND workspace_ref NOT ILIKE '%.GOV%'
        AND workspace_ref !~ '\s'
        AND workspace_ref !~ '\\'
        AND workspace_ref !~ '^[A-Za-z]:'
        AND workspace_ref NOT LIKE 'file:%'
    ),
    CONSTRAINT chk_atelier_pose_context_state_requested_by CHECK (
        btrim(requested_by) = requested_by
        AND requested_by <> ''
        AND requested_by NOT ILIKE '%.GOV%'
        AND requested_by !~ '^[A-Za-z]:'
        AND requested_by NOT LIKE 'file:%'
    ),
    CONSTRAINT chk_atelier_pose_context_state_kind_links CHECK (
        (
            kind = 'blank'
            AND source_asset_id IS NULL
            AND character_internal_id IS NULL
            AND collection_id IS NULL
            AND selected_rig_id IS NULL
        )
        OR (
            kind = 'single_image'
            AND source_asset_id IS NOT NULL
            AND character_internal_id IS NULL
            AND collection_id IS NULL
        )
        OR (
            kind = 'character_linked'
            AND character_internal_id IS NOT NULL
            AND collection_id IS NULL
        )
        OR (
            kind = 'collection_linked'
            AND collection_id IS NOT NULL
        )
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_pose_context_state_workspace_seq
    ON atelier_pose_context_state(workspace_ref, state_seq DESC);

CREATE INDEX IF NOT EXISTS idx_atelier_pose_context_state_links
    ON atelier_pose_context_state(character_internal_id, collection_id, selected_rig_id);
