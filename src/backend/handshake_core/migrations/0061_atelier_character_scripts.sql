-- WP-KERNEL-005 MT-040: per-character image-sourcing scripts as data-only records.
-- Scripts preserve provenance and usage refs but cannot become hidden executable authority.

CREATE TABLE IF NOT EXISTS atelier_character_script (
    script_id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    character_internal_id       UUID NOT NULL REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    script_name                 TEXT NOT NULL,
    script_body_raw_text        TEXT NOT NULL,
    provenance_refs_json        JSONB NOT NULL DEFAULT '[]'::jsonb,
    usage_refs_json             JSONB NOT NULL DEFAULT '[]'::jsonb,
    authority_mode              TEXT NOT NULL DEFAULT 'data_only',
    hidden_executable_authority BOOLEAN NOT NULL DEFAULT FALSE,
    created_by                  TEXT NOT NULL,
    created_at_utc              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_character_script_provenance_refs_array
        CHECK (jsonb_typeof(provenance_refs_json) = 'array'),
    CONSTRAINT chk_atelier_character_script_usage_refs_array
        CHECK (jsonb_typeof(usage_refs_json) = 'array'),
    CONSTRAINT chk_atelier_character_script_data_only_authority
        CHECK (authority_mode = 'data_only' AND hidden_executable_authority = FALSE)
);

CREATE INDEX IF NOT EXISTS idx_atelier_character_script_character
    ON atelier_character_script(character_internal_id, updated_at_utc DESC, script_id ASC);
