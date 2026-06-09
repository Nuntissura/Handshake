-- WP-KERNEL-005 MT-032: preserve intake sheet-version and collection
-- links through export. PostgreSQL authority only.

CREATE TABLE IF NOT EXISTS atelier_export_intake_link (
    link_id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    export_id               UUID NOT NULL REFERENCES atelier_export_request(export_id) ON DELETE CASCADE,
    batch_id                UUID NOT NULL REFERENCES atelier_intake_batch(batch_id) ON DELETE CASCADE,
    item_id                 UUID NOT NULL REFERENCES atelier_intake_item(item_id) ON DELETE CASCADE,
    target_character_id     UUID REFERENCES atelier_character(internal_id) ON DELETE SET NULL,
    target_sheet_version_id UUID REFERENCES atelier_sheet_version(version_id) ON DELETE SET NULL,
    target_collection_id    UUID REFERENCES atelier_collection(collection_id) ON DELETE SET NULL,
    version_agnostic        BOOLEAN NOT NULL,
    created_at_utc          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (export_id, batch_id, item_id)
);

CREATE INDEX IF NOT EXISTS idx_atelier_export_intake_link_export
    ON atelier_export_intake_link(export_id, created_at_utc);

CREATE INDEX IF NOT EXISTS idx_atelier_export_intake_link_batch
    ON atelier_export_intake_link(batch_id);

CREATE INDEX IF NOT EXISTS idx_atelier_export_intake_link_item
    ON atelier_export_intake_link(item_id);

CREATE INDEX IF NOT EXISTS idx_atelier_export_intake_link_targets
    ON atelier_export_intake_link(target_character_id, target_sheet_version_id, target_collection_id);
