-- WP-KERNEL-005 MT-040 hardening: character script refs must stay readable refs.
-- 0061 has already shipped in live schema tests, so ref-shape guards live here.

CREATE OR REPLACE FUNCTION atelier_jsonb_string_ref_array_valid(value JSONB, require_non_empty BOOLEAN)
RETURNS BOOLEAN
LANGUAGE SQL
IMMUTABLE
AS $$
    SELECT CASE
        WHEN jsonb_typeof(value) <> 'array' THEN FALSE
        WHEN require_non_empty AND jsonb_array_length(value) = 0 THEN FALSE
        ELSE NOT EXISTS (
            SELECT 1
            FROM jsonb_array_elements(value) AS ref(item)
            WHERE jsonb_typeof(ref.item) <> 'string'
               OR ref.item #>> '{}' = ''
               OR ref.item #>> '{}' <> btrim(ref.item #>> '{}')
        )
    END;
$$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_atelier_character_script_provenance_refs_string_array'
          AND conrelid = 'atelier_character_script'::regclass
    ) THEN
        ALTER TABLE atelier_character_script
        ADD CONSTRAINT chk_atelier_character_script_provenance_refs_string_array
            CHECK (atelier_jsonb_string_ref_array_valid(provenance_refs_json, TRUE));
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_atelier_character_script_usage_refs_string_array'
          AND conrelid = 'atelier_character_script'::regclass
    ) THEN
        ALTER TABLE atelier_character_script
        ADD CONSTRAINT chk_atelier_character_script_usage_refs_string_array
            CHECK (atelier_jsonb_string_ref_array_valid(usage_refs_json, FALSE));
    END IF;
END $$;
