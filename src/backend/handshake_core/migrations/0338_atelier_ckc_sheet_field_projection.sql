CREATE TABLE IF NOT EXISTS atelier_sheet_field_value_projection (
    projection_id UUID PRIMARY KEY,
    field_id TEXT NOT NULL,
    value TEXT NOT NULL,
    character_internal_id UUID NOT NULL REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    sheet_version_id UUID NOT NULL REFERENCES atelier_sheet_version(version_id) ON DELETE CASCADE,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (field_id, value, character_internal_id, sheet_version_id)
);

CREATE INDEX IF NOT EXISTS idx_atelier_sheet_field_value_projection_field_latest
    ON atelier_sheet_field_value_projection (field_id, created_at_utc DESC, sheet_version_id);
