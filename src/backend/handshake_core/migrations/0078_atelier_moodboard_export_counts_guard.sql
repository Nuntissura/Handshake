-- WP-KERNEL-005 MT-043 hardening: export manifests must preserve the exact
-- source-derived moodboard collection counts, not only an object-shaped value.

CREATE OR REPLACE FUNCTION atelier_moodboard_export_counts_contract_guard_v5()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_expected_counts JSONB;
BEGIN
    SELECT jsonb_build_object(
               'layers', jsonb_array_length(moodboard_json->'layers'),
               'images', jsonb_array_length(moodboard_json->'images'),
               'text', jsonb_array_length(moodboard_json->'text'),
               'shapes', jsonb_array_length(moodboard_json->'shapes'),
               'connectors', jsonb_array_length(moodboard_json->'connectors'),
               'folders', jsonb_array_length(moodboard_json->'folders'),
               'guides', jsonb_array_length(moodboard_json->'guides'),
               'history', jsonb_array_length(moodboard_json->'history')
           )
      INTO v_expected_counts
      FROM atelier_moodboard
     WHERE snapshot_id = NEW.snapshot_id;

    IF v_expected_counts IS NULL THEN
        RAISE EXCEPTION 'moodboard snapshot % does not exist', NEW.snapshot_id;
    END IF;

    IF (NEW.manifest_json->'counts' = v_expected_counts) IS NOT TRUE THEN
        RAISE EXCEPTION 'moodboard export manifest_json counts must match the source snapshot';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_atelier_moodboard_export_counts_contract_guard_v5
    ON atelier_moodboard_export_request;
CREATE TRIGGER trg_atelier_moodboard_export_counts_contract_guard_v5
BEFORE INSERT OR UPDATE
ON atelier_moodboard_export_request
FOR EACH ROW
EXECUTE FUNCTION atelier_moodboard_export_counts_contract_guard_v5();
