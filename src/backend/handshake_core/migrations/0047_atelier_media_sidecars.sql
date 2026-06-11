-- WP-KERNEL-005 MT-022 sidecar visibility matrix.
-- Sidecar artifacts remain first-class media assets, but normal gallery
-- listing excludes assets registered as hidden sidecars. Relation search stays
-- explicit through this matrix.

CREATE TABLE IF NOT EXISTS atelier_media_sidecar (
    sidecar_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_asset_id         UUID NOT NULL REFERENCES atelier_media_asset(asset_id) ON DELETE CASCADE,
    sidecar_asset_id        UUID NOT NULL REFERENCES atelier_media_asset(asset_id) ON DELETE CASCADE,
    relation_kind           TEXT NOT NULL CHECK (relation_kind IN ('openpose_json', 'workflow_json')),
    hidden_from_gallery     BOOLEAN NOT NULL DEFAULT TRUE,
    searchable_by_relation  BOOLEAN NOT NULL DEFAULT TRUE,
    created_by              TEXT NOT NULL,
    created_at_utc          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (parent_asset_id <> sidecar_asset_id),
    CHECK (hidden_from_gallery = TRUE),
    CHECK (searchable_by_relation = TRUE),
    UNIQUE (parent_asset_id, sidecar_asset_id, relation_kind)
);

CREATE INDEX IF NOT EXISTS idx_atelier_media_sidecar_parent_relation
    ON atelier_media_sidecar(parent_asset_id, relation_kind, updated_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_atelier_media_sidecar_sidecar
    ON atelier_media_sidecar(sidecar_asset_id);

CREATE INDEX IF NOT EXISTS idx_atelier_media_sidecar_gallery_hidden
    ON atelier_media_sidecar(sidecar_asset_id)
    WHERE hidden_from_gallery;
