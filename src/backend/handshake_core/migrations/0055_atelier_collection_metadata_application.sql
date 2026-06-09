-- WP-KERNEL-005 MT-035: apply collection batch metadata/tags to member
-- photos while preserving existing photo tags unless explicitly removed.
-- PostgreSQL authority only.

CREATE TABLE IF NOT EXISTS atelier_media_asset_tag (
    asset_id       UUID NOT NULL REFERENCES atelier_media_asset(asset_id) ON DELETE CASCADE,
    tag_id         UUID NOT NULL REFERENCES atelier_tag(tag_id) ON DELETE CASCADE,
    source         TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (asset_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_atelier_media_asset_tag_tag
    ON atelier_media_asset_tag(tag_id, asset_id);

CREATE TABLE IF NOT EXISTS atelier_collection_metadata_application (
    application_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_id        UUID NOT NULL REFERENCES atelier_collection(collection_id) ON DELETE CASCADE,
    requested_by         TEXT NOT NULL,
    applied_tags_json    JSONB NOT NULL DEFAULT '[]'::jsonb,
    removed_tags_json    JSONB NOT NULL DEFAULT '[]'::jsonb,
    affected_asset_count BIGINT NOT NULL DEFAULT 0,
    created_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_atelier_collection_metadata_application_collection
    ON atelier_collection_metadata_application(collection_id, created_at_utc DESC);
