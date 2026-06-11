-- WP-KERNEL-005 MT-040 hardening: match Rust reader whitespace rules for refs.
-- 0062 used btrim(), which misses tab/newline padding.

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
               OR ref.item #>> '{}' ~ '^[[:space:]]|[[:space:]]$'
        )
    END;
$$;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1
        FROM atelier_character_script
        WHERE NOT atelier_jsonb_string_ref_array_valid(provenance_refs_json, TRUE)
           OR NOT atelier_jsonb_string_ref_array_valid(usage_refs_json, FALSE)
    ) THEN
        RAISE EXCEPTION 'atelier_character_script contains invalid padded or non-string refs';
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_atelier_character_script_ref_whitespace_guard_v2'
          AND conrelid = 'atelier_character_script'::regclass
    ) THEN
        ALTER TABLE atelier_character_script
        ADD CONSTRAINT chk_atelier_character_script_ref_whitespace_guard_v2
            CHECK (
                atelier_jsonb_string_ref_array_valid(provenance_refs_json, TRUE)
                AND atelier_jsonb_string_ref_array_valid(usage_refs_json, FALSE)
            );
    END IF;
END $$;
