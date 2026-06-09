-- WP-KERNEL-005 MT-042: moodboard JSON Schema v1 and layer model snapshots.
-- Exact source JSON text is preserved for round-trip; JSONB is the queryable
-- projection guarded against direct-SQL rows that omit the required structure.

CREATE TABLE IF NOT EXISTS atelier_moodboard (
    snapshot_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id         UUID NOT NULL REFERENCES atelier_character_document(document_id) ON DELETE CASCADE,
    document_version_id UUID NOT NULL REFERENCES atelier_character_document_version(version_id) ON DELETE CASCADE,
    schema_id           TEXT NOT NULL,
    schema_version      BIGINT NOT NULL,
    raw_json_text       TEXT NOT NULL,
    moodboard_json      JSONB NOT NULL,
    content_sha256      TEXT NOT NULL,
    author              TEXT NOT NULL,
    created_at_utc      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_moodboard_schema_identity
        CHECK (schema_id = 'hsk.atelier.moodboard@1' AND schema_version = 1),
    CONSTRAINT chk_atelier_moodboard_raw_nonempty
        CHECK (btrim(raw_json_text) <> ''),
    CONSTRAINT chk_atelier_moodboard_json_object
        CHECK (jsonb_typeof(moodboard_json) = 'object'),
    CONSTRAINT chk_atelier_moodboard_content_sha256
        CHECK (content_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_atelier_moodboard_author_nonempty
        CHECK (btrim(author) <> ''),
    CONSTRAINT chk_atelier_moodboard_required_structure
        CHECK (
            moodboard_json->>'schema_id' = schema_id
            AND moodboard_json->>'schema_version' = schema_version::text
            AND moodboard_json ? 'moodboard_id'
            AND moodboard_json ? 'name'
            AND jsonb_typeof(moodboard_json->'canvas') = 'object'
            AND jsonb_typeof(moodboard_json->'layers') = 'array'
            AND jsonb_typeof(moodboard_json->'images') = 'array'
            AND jsonb_typeof(moodboard_json->'text') = 'array'
            AND jsonb_typeof(moodboard_json->'shapes') = 'array'
            AND jsonb_typeof(moodboard_json->'connectors') = 'array'
            AND jsonb_typeof(moodboard_json->'folders') = 'array'
            AND jsonb_typeof(moodboard_json->'guides') = 'array'
            AND jsonb_typeof(moodboard_json->'flags') = 'object'
            AND jsonb_typeof(moodboard_json->'style') = 'object'
            AND jsonb_typeof(moodboard_json->'history') = 'array'
        )
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_atelier_moodboard_document_version_hash
    ON atelier_moodboard(document_id, document_version_id, content_sha256);

CREATE INDEX IF NOT EXISTS idx_atelier_moodboard_document_latest
    ON atelier_moodboard(document_id, created_at_utc DESC, snapshot_id DESC);

CREATE OR REPLACE FUNCTION atelier_moodboard_guard()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_doc_type TEXT;
    v_version_document_id UUID;
BEGIN
    SELECT doc_type
      INTO v_doc_type
      FROM atelier_character_document
     WHERE document_id = NEW.document_id;

    IF v_doc_type IS NULL THEN
        RAISE EXCEPTION 'moodboard document % does not exist', NEW.document_id;
    END IF;

    IF v_doc_type <> 'moodboard' THEN
        RAISE EXCEPTION 'document % must be a moodboard document', NEW.document_id;
    END IF;

    SELECT document_id
      INTO v_version_document_id
      FROM atelier_character_document_version
     WHERE version_id = NEW.document_version_id;

    IF v_version_document_id IS NULL THEN
        RAISE EXCEPTION 'moodboard document version % does not exist', NEW.document_version_id;
    END IF;

    IF v_version_document_id <> NEW.document_id THEN
        RAISE EXCEPTION 'moodboard document version % does not belong to document %',
            NEW.document_version_id, NEW.document_id;
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_atelier_moodboard_guard ON atelier_moodboard;
CREATE TRIGGER trg_atelier_moodboard_guard
BEFORE INSERT OR UPDATE OF document_id, document_version_id
ON atelier_moodboard
FOR EACH ROW
EXECUTE FUNCTION atelier_moodboard_guard();
