-- WP-KERNEL-005 MT-043 hardening: PostgreSQL CHECK constraints pass UNKNOWN,
-- so the v3 contract guards use null-strict IS TRUE comparisons to reject
-- missing manifest/receipt JSON keys as well as wrong values.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_atelier_moodboard_operation_receipt_contract_v3'
          AND conrelid = 'atelier_moodboard_operation_receipt'::regclass
    ) THEN
        ALTER TABLE atelier_moodboard_operation_receipt
        ADD CONSTRAINT chk_atelier_moodboard_operation_receipt_contract_v3
        CHECK (
            (receipt_json->>'operation_id' = operation_id::text) IS TRUE
            AND (receipt_json->>'snapshot_id' = snapshot_id::text) IS TRUE
            AND (receipt_json->>'document_id' = document_id::text) IS TRUE
            AND (receipt_json->>'document_version_id' = document_version_id::text) IS TRUE
            AND (receipt_json->>'operation_kind' = operation_kind) IS TRUE
            AND (receipt_json->>'operation_payload_sha256' = operation_payload_sha256) IS TRUE
            AND (receipt_json->>'actor' = actor) IS TRUE
        );
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_atelier_moodboard_export_manifest_contract_v3'
          AND conrelid = 'atelier_moodboard_export_request'::regclass
    ) THEN
        ALTER TABLE atelier_moodboard_export_request
        ADD CONSTRAINT chk_atelier_moodboard_export_manifest_contract_v3
        CHECK (
            (manifest_json->>'export_id' = export_id::text) IS TRUE
            AND (manifest_json->>'snapshot_id' = snapshot_id::text) IS TRUE
            AND (manifest_json->>'document_id' = document_id::text) IS TRUE
            AND (manifest_json->>'document_version_id' = document_version_id::text) IS TRUE
            AND (manifest_json->>'format' = format) IS TRUE
            AND (manifest_json->>'status' = status) IS TRUE
            AND (manifest_json->'output'->>'artifact' = 'not_produced') IS TRUE
        );
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_atelier_moodboard_export_receipt_contract_v3'
          AND conrelid = 'atelier_moodboard_export_request'::regclass
    ) THEN
        ALTER TABLE atelier_moodboard_export_request
        ADD CONSTRAINT chk_atelier_moodboard_export_receipt_contract_v3
        CHECK (
            (receipt_json->>'export_id' = export_id::text) IS TRUE
            AND (receipt_json->>'snapshot_id' = snapshot_id::text) IS TRUE
            AND (receipt_json->>'document_id' = document_id::text) IS TRUE
            AND (receipt_json->>'document_version_id' = document_version_id::text) IS TRUE
            AND (receipt_json->>'format' = format) IS TRUE
            AND (receipt_json->>'status' = status) IS TRUE
            AND (receipt_json->>'requested_by' = requested_by) IS TRUE
            AND (receipt_json->>'output_artifact' = 'not_produced') IS TRUE
        );
    END IF;
END;
$$;
