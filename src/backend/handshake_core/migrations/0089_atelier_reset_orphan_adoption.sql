-- WP-KERNEL-005 MT-050 reset modes and orphan adoption.
-- Preferences-only resets delete preference authority rows. Full resets keep
-- original media assets and record them in an adoptable orphan manifest.

CREATE TABLE IF NOT EXISTS atelier_reset_operation (
    reset_id                         UUID PRIMARY KEY,
    mode                             TEXT NOT NULL,
    requested_by                     TEXT NOT NULL,
    reason                           TEXT NOT NULL,
    preferences_deleted_count         BIGINT NOT NULL DEFAULT 0,
    original_media_preserved_count    BIGINT NOT NULL DEFAULT 0,
    orphan_manifest_id                UUID,
    created_at_utc                   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_reset_operation_mode
        CHECK (mode IN ('preferences_only', 'full_preserve_original_media')),
    CONSTRAINT chk_atelier_reset_operation_actor
        CHECK (btrim(requested_by) = requested_by AND btrim(requested_by) <> ''),
    CONSTRAINT chk_atelier_reset_operation_reason
        CHECK (btrim(reason) = reason AND btrim(reason) <> '')
);
CREATE INDEX IF NOT EXISTS idx_atelier_reset_operation_created
    ON atelier_reset_operation(created_at_utc DESC);

CREATE TABLE IF NOT EXISTS atelier_orphan_manifest (
    manifest_id     UUID PRIMARY KEY,
    reset_id        UUID NOT NULL UNIQUE REFERENCES atelier_reset_operation(reset_id) ON DELETE CASCADE,
    manifest_json   JSONB NOT NULL,
    item_count      BIGINT NOT NULL DEFAULT 0,
    created_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_orphan_manifest_json
        CHECK (
            manifest_json ? 'schema_id'
            AND manifest_json->>'schema_id' = 'hsk.atelier.orphan_manifest@1'
            AND manifest_json ? 'reset_id'
            AND manifest_json ? 'item_count'
        )
);

CREATE TABLE IF NOT EXISTS atelier_orphan_manifest_item (
    manifest_item_id  UUID PRIMARY KEY,
    manifest_id       UUID NOT NULL REFERENCES atelier_orphan_manifest(manifest_id) ON DELETE CASCADE,
    asset_id          UUID NOT NULL REFERENCES atelier_media_asset(asset_id) ON DELETE RESTRICT,
    content_hash      TEXT NOT NULL,
    artifact_ref      TEXT NOT NULL,
    mime              TEXT NOT NULL,
    byte_len          BIGINT NOT NULL,
    retention_class   TEXT NOT NULL,
    adoption_status   TEXT NOT NULL DEFAULT 'orphaned',
    adopted_batch_id  UUID REFERENCES atelier_intake_batch(batch_id) ON DELETE SET NULL,
    adopted_item_id   UUID REFERENCES atelier_intake_item(item_id) ON DELETE SET NULL,
    adopted_by        TEXT,
    adopted_at_utc    TIMESTAMPTZ,
    created_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_atelier_orphan_manifest_item_asset UNIQUE (manifest_id, asset_id),
    CONSTRAINT chk_atelier_orphan_manifest_item_status
        CHECK (adoption_status IN ('orphaned', 'adopted')),
    CONSTRAINT chk_atelier_orphan_manifest_item_payload
        CHECK (
            btrim(content_hash) = content_hash AND btrim(content_hash) <> ''
            AND btrim(artifact_ref) = artifact_ref AND btrim(artifact_ref) <> ''
            AND btrim(mime) = mime AND btrim(mime) <> ''
            AND byte_len > 0
            AND btrim(retention_class) = retention_class AND btrim(retention_class) <> ''
        ),
    CONSTRAINT chk_atelier_orphan_manifest_item_adoption
        CHECK (
            (adoption_status = 'orphaned'
                AND adopted_batch_id IS NULL
                AND adopted_item_id IS NULL
                AND adopted_by IS NULL
                AND adopted_at_utc IS NULL)
            OR
            (adoption_status = 'adopted'
                AND adopted_batch_id IS NOT NULL
                AND adopted_item_id IS NOT NULL
                AND adopted_by IS NOT NULL
                AND btrim(adopted_by) = adopted_by
                AND btrim(adopted_by) <> ''
                AND adopted_at_utc IS NOT NULL)
        )
);
CREATE INDEX IF NOT EXISTS idx_atelier_orphan_manifest_item_manifest_status
    ON atelier_orphan_manifest_item(manifest_id, adoption_status);
CREATE INDEX IF NOT EXISTS idx_atelier_orphan_manifest_item_asset
    ON atelier_orphan_manifest_item(asset_id);
