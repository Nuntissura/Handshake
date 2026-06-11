-- WP-KERNEL-005 MT-042 hardening: direct-SQL moodboard snapshots must keep
-- exact raw JSON, JSONB projection, content hash, and document/version
-- invariants aligned after the initial 0072 rollout.

CREATE OR REPLACE FUNCTION atelier_moodboard_guard()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_doc_type TEXT;
    v_version_document_id UUID;
    v_raw_json JSONB;
    v_content_sha256 TEXT;
BEGIN
    BEGIN
        v_raw_json := NEW.raw_json_text::jsonb;
    EXCEPTION WHEN OTHERS THEN
        RAISE EXCEPTION 'moodboard raw_json_text must parse as JSON';
    END;

    IF v_raw_json <> NEW.moodboard_json THEN
        RAISE EXCEPTION 'moodboard raw_json_text and moodboard_json must represent the same JSON value';
    END IF;

    v_content_sha256 := encode(sha256(convert_to(NEW.raw_json_text, 'UTF8')), 'hex');
    IF NEW.content_sha256 <> v_content_sha256 THEN
        RAISE EXCEPTION 'moodboard content_sha256 must match raw_json_text';
    END IF;

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
BEFORE INSERT OR UPDATE OF document_id, document_version_id, schema_id, schema_version,
    raw_json_text, moodboard_json, content_sha256
ON atelier_moodboard
FOR EACH ROW
EXECUTE FUNCTION atelier_moodboard_guard();

CREATE OR REPLACE FUNCTION atelier_moodboard_document_guard()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.doc_type = 'moodboard'
       AND NEW.doc_type <> 'moodboard'
       AND EXISTS (
           SELECT 1
           FROM atelier_moodboard
           WHERE document_id = OLD.document_id
       ) THEN
        RAISE EXCEPTION 'cannot change document % away from moodboard while moodboard snapshots exist',
            OLD.document_id;
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_atelier_moodboard_document_guard ON atelier_character_document;
CREATE TRIGGER trg_atelier_moodboard_document_guard
BEFORE UPDATE OF doc_type
ON atelier_character_document
FOR EACH ROW
EXECUTE FUNCTION atelier_moodboard_document_guard();

CREATE OR REPLACE FUNCTION atelier_moodboard_version_guard()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.document_id <> OLD.document_id
       AND EXISTS (
           SELECT 1
           FROM atelier_moodboard
           WHERE document_version_id = OLD.version_id
       ) THEN
        RAISE EXCEPTION 'cannot move version % to another document while moodboard snapshots reference it',
            OLD.version_id;
    END IF;

    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_atelier_moodboard_version_guard ON atelier_character_document_version;
CREATE TRIGGER trg_atelier_moodboard_version_guard
BEFORE UPDATE OF document_id
ON atelier_character_document_version
FOR EACH ROW
EXECUTE FUNCTION atelier_moodboard_version_guard();
