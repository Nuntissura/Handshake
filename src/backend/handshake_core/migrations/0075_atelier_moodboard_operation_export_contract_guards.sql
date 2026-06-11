-- WP-KERNEL-005 MT-043 hardening: operation/export receipt and manifest JSON
-- must stay aligned with authoritative row columns on every direct-SQL update,
-- including primary-key and actor/requester drift.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_atelier_moodboard_operation_receipt_contract_v2'
          AND conrelid = 'atelier_moodboard_operation_receipt'::regclass
    ) THEN
        ALTER TABLE atelier_moodboard_operation_receipt
        ADD CONSTRAINT chk_atelier_moodboard_operation_receipt_contract_v2
        CHECK (
            receipt_json->>'operation_id' = operation_id::text
            AND receipt_json->>'snapshot_id' = snapshot_id::text
            AND receipt_json->>'document_id' = document_id::text
            AND receipt_json->>'document_version_id' = document_version_id::text
            AND receipt_json->>'operation_kind' = operation_kind
            AND receipt_json->>'operation_payload_sha256' = operation_payload_sha256
            AND receipt_json->>'actor' = actor
        );
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_atelier_moodboard_export_manifest_contract_v2'
          AND conrelid = 'atelier_moodboard_export_request'::regclass
    ) THEN
        ALTER TABLE atelier_moodboard_export_request
        ADD CONSTRAINT chk_atelier_moodboard_export_manifest_contract_v2
        CHECK (
            manifest_json->>'export_id' = export_id::text
            AND manifest_json->>'snapshot_id' = snapshot_id::text
            AND manifest_json->>'document_id' = document_id::text
            AND manifest_json->>'document_version_id' = document_version_id::text
            AND manifest_json->>'format' = format
            AND manifest_json->>'status' = status
            AND manifest_json->'output'->>'artifact' = 'not_produced'
        );
    END IF;

    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_atelier_moodboard_export_receipt_contract_v2'
          AND conrelid = 'atelier_moodboard_export_request'::regclass
    ) THEN
        ALTER TABLE atelier_moodboard_export_request
        ADD CONSTRAINT chk_atelier_moodboard_export_receipt_contract_v2
        CHECK (
            receipt_json->>'export_id' = export_id::text
            AND receipt_json->>'snapshot_id' = snapshot_id::text
            AND receipt_json->>'document_id' = document_id::text
            AND receipt_json->>'document_version_id' = document_version_id::text
            AND receipt_json->>'format' = format
            AND receipt_json->>'status' = status
            AND receipt_json->>'requested_by' = requested_by
            AND receipt_json->>'output_artifact' = 'not_produced'
        );
    END IF;
END;
$$;
