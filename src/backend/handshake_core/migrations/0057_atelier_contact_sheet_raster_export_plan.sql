-- WP-KERNEL-005 MT-037: preserve PNG/JPG contact-sheet raster export as an
-- explicit planned capability without faking rendered output artifacts.
-- PostgreSQL authority only.

CREATE TABLE IF NOT EXISTS atelier_contact_sheet_raster_export_plan (
    plan_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sheet_id       UUID NOT NULL REFERENCES atelier_contact_sheet(sheet_id) ON DELETE CASCADE,
    format         TEXT NOT NULL,
    status         TEXT NOT NULL DEFAULT 'planned',
    reason         TEXT NOT NULL,
    requested_by   TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_contact_sheet_raster_export_format
        CHECK (format IN ('png', 'jpg')),
    CONSTRAINT chk_atelier_contact_sheet_raster_export_status
        CHECK (status = 'planned'),
    UNIQUE (sheet_id, format)
);

CREATE INDEX IF NOT EXISTS idx_atelier_contact_sheet_raster_export_plan_sheet
    ON atelier_contact_sheet_raster_export_plan(sheet_id, created_at_utc DESC);
