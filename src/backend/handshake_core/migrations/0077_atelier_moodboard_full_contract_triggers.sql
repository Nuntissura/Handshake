-- WP-KERNEL-005 MT-043 hardening: full manifest/receipt contracts must stay
-- null-strictly aligned with their source moodboard snapshot and row columns.
-- This supplements v3 CHECK constraints with cross-row source validation.

CREATE OR REPLACE FUNCTION atelier_moodboard_operation_contract_guard_v4()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_document_id UUID;
    v_document_version_id UUID;
    v_schema_id TEXT;
    v_schema_version BIGINT;
    v_content_sha256 TEXT;
BEGIN
    SELECT document_id, document_version_id, schema_id, schema_version, content_sha256
      INTO v_document_id, v_document_version_id, v_schema_id, v_schema_version, v_content_sha256
      FROM atelier_moodboard
     WHERE snapshot_id = NEW.snapshot_id;

    IF v_document_id IS NULL THEN
        RAISE EXCEPTION 'moodboard snapshot % does not exist', NEW.snapshot_id;
    END IF;

    IF NEW.document_id IS DISTINCT FROM v_document_id
       OR NEW.document_version_id IS DISTINCT FROM v_document_version_id THEN
        RAISE EXCEPTION 'moodboard operation row does not match source snapshot';
    END IF;

    IF NEW.receipt_json->>'schema' IS DISTINCT FROM 'hsk.atelier.moodboard_operation_receipt@1'
       OR NEW.receipt_json->>'operation_id' IS DISTINCT FROM NEW.operation_id::text
       OR NEW.receipt_json->>'snapshot_id' IS DISTINCT FROM NEW.snapshot_id::text
       OR NEW.receipt_json->>'document_id' IS DISTINCT FROM NEW.document_id::text
       OR NEW.receipt_json->>'document_version_id' IS DISTINCT FROM NEW.document_version_id::text
       OR NEW.receipt_json->>'operation_kind' IS DISTINCT FROM NEW.operation_kind
       OR NEW.receipt_json->>'operation_payload_sha256' IS DISTINCT FROM NEW.operation_payload_sha256
       OR NEW.receipt_json->>'source_schema_id' IS DISTINCT FROM v_schema_id
       OR NEW.receipt_json->>'source_schema_version' IS DISTINCT FROM v_schema_version::text
       OR NEW.receipt_json->>'source_content_sha256' IS DISTINCT FROM v_content_sha256
       OR NEW.receipt_json->>'actor' IS DISTINCT FROM NEW.actor
       OR NEW.receipt_json->>'status' IS DISTINCT FROM 'recorded' THEN
        RAISE EXCEPTION 'moodboard operation receipt_json must preserve the full operation contract';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_atelier_moodboard_operation_contract_guard_v4
    ON atelier_moodboard_operation_receipt;
CREATE TRIGGER trg_atelier_moodboard_operation_contract_guard_v4
BEFORE INSERT OR UPDATE
ON atelier_moodboard_operation_receipt
FOR EACH ROW
EXECUTE FUNCTION atelier_moodboard_operation_contract_guard_v4();

CREATE OR REPLACE FUNCTION atelier_moodboard_export_contract_guard_v4()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_document_id UUID;
    v_document_version_id UUID;
    v_schema_id TEXT;
    v_schema_version BIGINT;
    v_content_sha256 TEXT;
BEGIN
    SELECT document_id, document_version_id, schema_id, schema_version, content_sha256
      INTO v_document_id, v_document_version_id, v_schema_id, v_schema_version, v_content_sha256
      FROM atelier_moodboard
     WHERE snapshot_id = NEW.snapshot_id;

    IF v_document_id IS NULL THEN
        RAISE EXCEPTION 'moodboard snapshot % does not exist', NEW.snapshot_id;
    END IF;

    IF NEW.document_id IS DISTINCT FROM v_document_id
       OR NEW.document_version_id IS DISTINCT FROM v_document_version_id THEN
        RAISE EXCEPTION 'moodboard export row does not match source snapshot';
    END IF;

    IF NEW.manifest_json->>'schema' IS DISTINCT FROM 'hsk.atelier.moodboard_export_manifest@1'
       OR NEW.manifest_json->>'export_id' IS DISTINCT FROM NEW.export_id::text
       OR NEW.manifest_json->>'snapshot_id' IS DISTINCT FROM NEW.snapshot_id::text
       OR NEW.manifest_json->>'document_id' IS DISTINCT FROM NEW.document_id::text
       OR NEW.manifest_json->>'document_version_id' IS DISTINCT FROM NEW.document_version_id::text
       OR NEW.manifest_json->>'format' IS DISTINCT FROM NEW.format
       OR NEW.manifest_json->>'status' IS DISTINCT FROM NEW.status
       OR NEW.manifest_json->>'source_schema_id' IS DISTINCT FROM v_schema_id
       OR NEW.manifest_json->>'source_schema_version' IS DISTINCT FROM v_schema_version::text
       OR NEW.manifest_json->>'source_content_sha256' IS DISTINCT FROM v_content_sha256
       OR jsonb_typeof(NEW.manifest_json->'counts') IS DISTINCT FROM 'object'
       OR NEW.manifest_json->'output'->>'artifact' IS DISTINCT FROM 'not_produced'
       OR NEW.manifest_json->'output'->>'reason' IS DISTINCT FROM 'deferred: PNG/PDF moodboard export is planned; no renderer is implemented and no output artifact was produced' THEN
        RAISE EXCEPTION 'moodboard export manifest_json must preserve the full export manifest contract';
    END IF;

    IF NEW.receipt_json->>'schema' IS DISTINCT FROM 'hsk.atelier.moodboard_export_receipt@1'
       OR NEW.receipt_json->>'export_id' IS DISTINCT FROM NEW.export_id::text
       OR NEW.receipt_json->>'snapshot_id' IS DISTINCT FROM NEW.snapshot_id::text
       OR NEW.receipt_json->>'document_id' IS DISTINCT FROM NEW.document_id::text
       OR NEW.receipt_json->>'document_version_id' IS DISTINCT FROM NEW.document_version_id::text
       OR NEW.receipt_json->>'format' IS DISTINCT FROM NEW.format
       OR NEW.receipt_json->>'status' IS DISTINCT FROM NEW.status
       OR NEW.receipt_json->>'requested_by' IS DISTINCT FROM NEW.requested_by
       OR NEW.receipt_json->>'source_content_sha256' IS DISTINCT FROM v_content_sha256
       OR NEW.receipt_json->>'output_artifact' IS DISTINCT FROM 'not_produced'
       OR NEW.receipt_json->>'deferred_reason' IS DISTINCT FROM 'deferred: PNG/PDF moodboard export is planned; no renderer is implemented and no output artifact was produced' THEN
        RAISE EXCEPTION 'moodboard export receipt_json must preserve the full export receipt contract';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_atelier_moodboard_export_contract_guard_v4
    ON atelier_moodboard_export_request;
CREATE TRIGGER trg_atelier_moodboard_export_contract_guard_v4
BEFORE INSERT OR UPDATE
ON atelier_moodboard_export_request
FOR EACH ROW
EXECUTE FUNCTION atelier_moodboard_export_contract_guard_v4();
