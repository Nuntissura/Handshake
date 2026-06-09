-- WP-KERNEL-005 MT-036: materialize reproducible contact-sheet SVG artifacts
-- from the persisted manifest and source image ids. PostgreSQL authority only.

CREATE TABLE IF NOT EXISTS atelier_contact_sheet_svg_artifact (
    svg_artifact_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sheet_id        UUID NOT NULL REFERENCES atelier_contact_sheet(sheet_id) ON DELETE CASCADE,
    manifest_hash   TEXT NOT NULL,
    content_hash    TEXT NOT NULL,
    artifact_ref    TEXT NOT NULL UNIQUE,
    svg_text        TEXT NOT NULL,
    image_count     BIGINT NOT NULL,
    created_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (sheet_id, manifest_hash)
);

CREATE INDEX IF NOT EXISTS idx_atelier_contact_sheet_svg_artifact_sheet
    ON atelier_contact_sheet_svg_artifact(sheet_id, created_at_utc DESC);
