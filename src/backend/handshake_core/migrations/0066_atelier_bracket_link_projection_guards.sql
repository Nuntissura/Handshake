-- WP-KERNEL-005 MT-041 hardening: projection rows must remain rebuildable when
-- written through direct SQL, not only through the Rust rebuild API.

CREATE OR REPLACE FUNCTION atelier_bracket_link_projection_guard()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    marker_inner TEXT;
    marker_target TEXT;
    marker_kind TEXT;
    marker_target_id TEXT;
    canonical_kind TEXT;
    version_document_id UUID;
    source_document_type TEXT;
    target_document_type TEXT;
    raw_character_id UUID;
BEGIN
    IF NEW.raw_marker !~ '^\[\[[^\[\]\r\n]+\]\]$' THEN
        RAISE EXCEPTION 'invalid bracket link raw_marker %', NEW.raw_marker
            USING ERRCODE = '23514';
    END IF;

    marker_inner := substring(NEW.raw_marker FROM 3 FOR char_length(NEW.raw_marker) - 4);
    marker_target := split_part(marker_inner, '|', 1);
    IF position(':' IN marker_target) = 0 THEN
        RAISE EXCEPTION 'bracket link raw_marker must use kind:id: %', NEW.raw_marker
            USING ERRCODE = '23514';
    END IF;

    marker_kind := lower(btrim(split_part(marker_target, ':', 1)));
    marker_target_id := btrim(substr(marker_target, position(':' IN marker_target) + 1));
    IF marker_target_id = '' THEN
        RAISE EXCEPTION 'bracket link raw_marker has empty target id: %', NEW.raw_marker
            USING ERRCODE = '23514';
    END IF;

    canonical_kind := CASE marker_kind
        WHEN 'char' THEN 'character'
        WHEN 'doc' THEN 'document'
        WHEN 'img' THEN 'image'
        WHEN 'media' THEN 'image'
        ELSE marker_kind
    END;
    IF canonical_kind <> NEW.target_kind THEN
        RAISE EXCEPTION 'bracket link raw_marker kind % does not match target_kind %',
            canonical_kind, NEW.target_kind
            USING ERRCODE = '23514';
    END IF;

    SELECT document_id
    INTO version_document_id
    FROM atelier_character_document_version
    WHERE version_id = NEW.source_version_id;
    IF version_document_id IS NULL OR version_document_id <> NEW.source_document_id THEN
        RAISE EXCEPTION 'bracket link source version % does not belong to source document %',
            NEW.source_version_id, NEW.source_document_id
            USING ERRCODE = '23514';
    END IF;

    SELECT doc_type
    INTO source_document_type
    FROM atelier_character_document
    WHERE document_id = NEW.source_document_id;
    IF source_document_type IS NULL OR source_document_type <> NEW.source_doc_type THEN
        RAISE EXCEPTION 'bracket link source_doc_type % does not match source document %',
            NEW.source_doc_type, NEW.source_document_id
            USING ERRCODE = '23514';
    END IF;

    IF NEW.target_kind = 'character' THEN
        IF NOT EXISTS (
            SELECT 1 FROM atelier_character WHERE internal_id::text = NEW.target_id
        ) THEN
            RAISE EXCEPTION 'bracket link character target % does not exist', NEW.target_id
                USING ERRCODE = '23514';
        END IF;
        SELECT internal_id
        INTO raw_character_id
        FROM atelier_character
        WHERE public_id = marker_target_id
           OR internal_id::text = marker_target_id
        LIMIT 1;
        IF raw_character_id IS NULL THEN
            RAISE EXCEPTION 'bracket link raw character target % does not exist', marker_target_id
                USING ERRCODE = '23514';
        END IF;
        IF raw_character_id::text <> NEW.target_id THEN
            RAISE EXCEPTION 'bracket link raw character target % does not match canonical target %',
                marker_target_id, NEW.target_id
                USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;

    IF NEW.target_kind IN ('document', 'story', 'moodboard') THEN
        SELECT doc_type
        INTO target_document_type
        FROM atelier_character_document
        WHERE document_id = NEW.target_id::uuid;
        IF target_document_type IS NULL THEN
            RAISE EXCEPTION 'bracket link document target % does not exist', NEW.target_id
                USING ERRCODE = '23514';
        END IF;
        IF NEW.target_kind = 'story' AND target_document_type <> 'story' THEN
            RAISE EXCEPTION 'bracket link target % is not a story', NEW.target_id
                USING ERRCODE = '23514';
        END IF;
        IF NEW.target_kind = 'moodboard' AND target_document_type <> 'moodboard' THEN
            RAISE EXCEPTION 'bracket link target % is not a moodboard', NEW.target_id
                USING ERRCODE = '23514';
        END IF;
        IF marker_target_id <> NEW.target_id THEN
            RAISE EXCEPTION 'bracket link raw target % does not match canonical target %',
                marker_target_id, NEW.target_id
                USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;

    IF NEW.target_kind = 'image' THEN
        IF NOT EXISTS (
            SELECT 1 FROM atelier_media_asset WHERE asset_id = NEW.target_id::uuid
        ) THEN
            RAISE EXCEPTION 'bracket link image target % does not exist', NEW.target_id
                USING ERRCODE = '23514';
        END IF;
        IF marker_target_id <> NEW.target_id THEN
            RAISE EXCEPTION 'bracket link raw image target % does not match canonical target %',
                marker_target_id, NEW.target_id
                USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;

    RAISE EXCEPTION 'unsupported bracket link target_kind %', NEW.target_kind
        USING ERRCODE = '23514';
END;
$$;

DROP TRIGGER IF EXISTS trg_atelier_bracket_link_projection_guard
    ON atelier_bracket_link_projection;
CREATE TRIGGER trg_atelier_bracket_link_projection_guard
    BEFORE INSERT OR UPDATE OF source_document_id, source_version_id, source_doc_type,
        raw_marker, target_kind, target_id
    ON atelier_bracket_link_projection
    FOR EACH ROW
    EXECUTE FUNCTION atelier_bracket_link_projection_guard();
