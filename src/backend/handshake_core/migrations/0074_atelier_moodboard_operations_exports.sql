-- WP-KERNEL-005 MT-043: moodboard operation receipts and PNG/PDF export hooks.
-- Export hooks are planned database contracts only: no rendered artifact refs,
-- no byte counts, no filesystem paths, and never .GOV outputs.

CREATE TABLE IF NOT EXISTS atelier_moodboard_operation_receipt (
    operation_id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    snapshot_id              UUID NOT NULL REFERENCES atelier_moodboard(snapshot_id) ON DELETE CASCADE,
    document_id              UUID NOT NULL REFERENCES atelier_character_document(document_id) ON DELETE CASCADE,
    document_version_id      UUID NOT NULL REFERENCES atelier_character_document_version(version_id) ON DELETE CASCADE,
    operation_kind           TEXT NOT NULL,
    operation_payload        JSONB NOT NULL,
    operation_payload_sha256 TEXT NOT NULL,
    receipt_json             JSONB NOT NULL,
    actor                    TEXT NOT NULL,
    created_at_utc           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_moodboard_operation_kind
        CHECK (operation_kind IN (
            'layer_reordered',
            'element_moved',
            'element_updated',
            'layer_visibility_changed',
            'style_updated',
            'history_annotated'
        )),
    CONSTRAINT chk_atelier_moodboard_operation_payload_object
        CHECK (jsonb_typeof(operation_payload) = 'object'),
    CONSTRAINT chk_atelier_moodboard_operation_payload_sha256
        CHECK (operation_payload_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_atelier_moodboard_operation_receipt_object
        CHECK (
            jsonb_typeof(receipt_json) = 'object'
            AND receipt_json->>'schema' = 'hsk.atelier.moodboard_operation_receipt@1'
            AND receipt_json->>'status' = 'recorded'
        ),
    CONSTRAINT chk_atelier_moodboard_operation_actor_nonempty
        CHECK (btrim(actor) <> '')
);

CREATE INDEX IF NOT EXISTS idx_atelier_moodboard_operation_receipt_snapshot
    ON atelier_moodboard_operation_receipt(snapshot_id, created_at_utc ASC, operation_id ASC);

CREATE TABLE IF NOT EXISTS atelier_moodboard_export_request (
    export_id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    snapshot_id         UUID NOT NULL REFERENCES atelier_moodboard(snapshot_id) ON DELETE CASCADE,
    document_id         UUID NOT NULL REFERENCES atelier_character_document(document_id) ON DELETE CASCADE,
    document_version_id UUID NOT NULL REFERENCES atelier_character_document_version(version_id) ON DELETE CASCADE,
    format              TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'planned',
    label               TEXT,
    manifest_json       JSONB NOT NULL,
    receipt_json        JSONB NOT NULL,
    requested_by        TEXT NOT NULL,
    created_at_utc      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_moodboard_export_format
        CHECK (format IN ('png', 'pdf')),
    CONSTRAINT chk_atelier_moodboard_export_status
        CHECK (status = 'planned'),
    CONSTRAINT chk_atelier_moodboard_export_label
        CHECK (label IS NULL OR (btrim(label) <> '' AND position('.gov' in lower(label)) = 0)),
    CONSTRAINT chk_atelier_moodboard_export_manifest_object
        CHECK (
            jsonb_typeof(manifest_json) = 'object'
            AND manifest_json->>'schema' = 'hsk.atelier.moodboard_export_manifest@1'
            AND manifest_json->>'status' = 'planned'
        ),
    CONSTRAINT chk_atelier_moodboard_export_receipt_object
        CHECK (
            jsonb_typeof(receipt_json) = 'object'
            AND receipt_json->>'schema' = 'hsk.atelier.moodboard_export_receipt@1'
            AND receipt_json->>'status' = 'planned'
            AND receipt_json->>'output_artifact' = 'not_produced'
        ),
    CONSTRAINT chk_atelier_moodboard_export_requested_by_nonempty
        CHECK (btrim(requested_by) <> ''),
    UNIQUE (snapshot_id, format)
);

CREATE INDEX IF NOT EXISTS idx_atelier_moodboard_export_request_snapshot
    ON atelier_moodboard_export_request(snapshot_id, created_at_utc ASC, format ASC);

CREATE OR REPLACE FUNCTION atelier_moodboard_operation_receipt_guard()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_document_id UUID;
    v_document_version_id UUID;
    v_content_sha256 TEXT;
    v_payload_sha256 TEXT;
BEGIN
    SELECT document_id, document_version_id, content_sha256
      INTO v_document_id, v_document_version_id, v_content_sha256
      FROM atelier_moodboard
     WHERE snapshot_id = NEW.snapshot_id;

    IF v_document_id IS NULL THEN
        RAISE EXCEPTION 'moodboard snapshot % does not exist', NEW.snapshot_id;
    END IF;

    IF NEW.document_id <> v_document_id THEN
        RAISE EXCEPTION 'moodboard operation document % does not match snapshot document %',
            NEW.document_id, v_document_id;
    END IF;

    IF NEW.document_version_id <> v_document_version_id THEN
        RAISE EXCEPTION 'moodboard operation version % does not match snapshot version %',
            NEW.document_version_id, v_document_version_id;
    END IF;

    v_payload_sha256 := encode(sha256(convert_to(NEW.operation_payload::text, 'UTF8')), 'hex');
    IF NEW.operation_payload_sha256 <> v_payload_sha256 THEN
        RAISE EXCEPTION 'moodboard operation payload hash must match operation_payload JSONB';
    END IF;

    IF NEW.receipt_json->>'operation_id' <> NEW.operation_id::text
       OR NEW.receipt_json->>'snapshot_id' <> NEW.snapshot_id::text
       OR NEW.receipt_json->>'document_id' <> NEW.document_id::text
       OR NEW.receipt_json->>'document_version_id' <> NEW.document_version_id::text
       OR NEW.receipt_json->>'operation_kind' <> NEW.operation_kind
       OR NEW.receipt_json->>'operation_payload_sha256' <> NEW.operation_payload_sha256
       OR NEW.receipt_json->>'source_content_sha256' <> v_content_sha256 THEN
        RAISE EXCEPTION 'moodboard operation receipt_json must match operation receipt row';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_atelier_moodboard_operation_receipt_guard
    ON atelier_moodboard_operation_receipt;
CREATE TRIGGER trg_atelier_moodboard_operation_receipt_guard
BEFORE INSERT OR UPDATE OF snapshot_id, document_id, document_version_id,
    operation_kind, operation_payload, operation_payload_sha256, receipt_json
ON atelier_moodboard_operation_receipt
FOR EACH ROW
EXECUTE FUNCTION atelier_moodboard_operation_receipt_guard();

CREATE OR REPLACE FUNCTION atelier_moodboard_export_request_guard()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_document_id UUID;
    v_document_version_id UUID;
    v_content_sha256 TEXT;
BEGIN
    SELECT document_id, document_version_id, content_sha256
      INTO v_document_id, v_document_version_id, v_content_sha256
      FROM atelier_moodboard
     WHERE snapshot_id = NEW.snapshot_id;

    IF v_document_id IS NULL THEN
        RAISE EXCEPTION 'moodboard snapshot % does not exist', NEW.snapshot_id;
    END IF;

    IF NEW.document_id <> v_document_id THEN
        RAISE EXCEPTION 'moodboard export document % does not match snapshot document %',
            NEW.document_id, v_document_id;
    END IF;

    IF NEW.document_version_id <> v_document_version_id THEN
        RAISE EXCEPTION 'moodboard export version % does not match snapshot version %',
            NEW.document_version_id, v_document_version_id;
    END IF;

    IF position('.gov' in lower(NEW.manifest_json::text)) > 0
       OR position('.gov' in lower(NEW.receipt_json::text)) > 0 THEN
        RAISE EXCEPTION 'moodboard export manifests and receipts must not reference .GOV outputs';
    END IF;

    IF NEW.manifest_json->>'export_id' <> NEW.export_id::text
       OR NEW.manifest_json->>'snapshot_id' <> NEW.snapshot_id::text
       OR NEW.manifest_json->>'document_id' <> NEW.document_id::text
       OR NEW.manifest_json->>'document_version_id' <> NEW.document_version_id::text
       OR NEW.manifest_json->>'format' <> NEW.format
       OR NEW.manifest_json->>'status' <> NEW.status
       OR NEW.manifest_json->>'source_content_sha256' <> v_content_sha256
       OR NEW.manifest_json->'output'->>'artifact' <> 'not_produced' THEN
        RAISE EXCEPTION 'moodboard export manifest_json must match export request row';
    END IF;

    IF NEW.receipt_json->>'export_id' <> NEW.export_id::text
       OR NEW.receipt_json->>'snapshot_id' <> NEW.snapshot_id::text
       OR NEW.receipt_json->>'document_id' <> NEW.document_id::text
       OR NEW.receipt_json->>'document_version_id' <> NEW.document_version_id::text
       OR NEW.receipt_json->>'format' <> NEW.format
       OR NEW.receipt_json->>'status' <> NEW.status
       OR NEW.receipt_json->>'source_content_sha256' <> v_content_sha256
       OR NEW.receipt_json->>'output_artifact' <> 'not_produced' THEN
        RAISE EXCEPTION 'moodboard export receipt_json must match export request row';
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_atelier_moodboard_export_request_guard
    ON atelier_moodboard_export_request;
CREATE TRIGGER trg_atelier_moodboard_export_request_guard
BEFORE INSERT OR UPDATE OF snapshot_id, document_id, document_version_id,
    format, status, manifest_json, receipt_json
ON atelier_moodboard_export_request
FOR EACH ROW
EXECUTE FUNCTION atelier_moodboard_export_request_guard();
