-- WP-KERNEL-005 MT-041 hardening v3: direct SQL projection rows must be
-- backed by the current source text and use canonical target identifiers.

CREATE OR REPLACE FUNCTION atelier_bracket_link_source_marker_at(
    source_text TEXT,
    expected_seq BIGINT
)
RETURNS TEXT
LANGUAGE plpgsql
AS $$
DECLARE
    cursor_pos INTEGER := 1;
    relative_start INTEGER;
    start_pos INTEGER;
    inner_start INTEGER;
    relative_end INTEGER;
    raw_length INTEGER;
    inner_text TEXT;
    raw_marker TEXT;
    current_seq BIGINT := 0;
    matched_marker TEXT := NULL;
BEGIN
    IF expected_seq <= 0 THEN
        RETURN NULL;
    END IF;

    LOOP
        relative_start := strpos(substr(source_text, cursor_pos), '[[');
        IF relative_start = 0 THEN
            RETURN matched_marker;
        END IF;

        start_pos := cursor_pos + relative_start - 1;
        inner_start := start_pos + 2;
        relative_end := strpos(substr(source_text, inner_start), ']]');
        IF relative_end = 0 THEN
            RAISE EXCEPTION 'unterminated bracket link marker near source byte %', start_pos
                USING ERRCODE = '23514';
        END IF;

        inner_text := substr(source_text, inner_start, relative_end - 1);
        IF inner_text = ''
           OR position('[[' IN inner_text) > 0
           OR position(chr(10) IN inner_text) > 0
           OR position(chr(13) IN inner_text) > 0 THEN
            RAISE EXCEPTION 'malformed bracket link marker near source byte %', start_pos
                USING ERRCODE = '23514';
        END IF;

        raw_length := inner_start + relative_end - start_pos + 1;
        raw_marker := substr(source_text, start_pos, raw_length);
        current_seq := current_seq + 1;
        IF current_seq = expected_seq THEN
            matched_marker := raw_marker;
        END IF;

        cursor_pos := inner_start + relative_end + 1;
    END LOOP;
END;
$$;

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
    source_current_version_id UUID;
    source_document_type TEXT;
    source_body_raw_text TEXT;
    source_marker TEXT;
    target_document_type TEXT;
    raw_character_id UUID;
    raw_character_uuid UUID;
    target_uuid UUID;
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

    SELECT d.current_version_id, d.doc_type, v.body_raw_text
    INTO source_current_version_id, source_document_type, source_body_raw_text
    FROM atelier_character_document_version v
    JOIN atelier_character_document d ON d.document_id = v.document_id
    WHERE v.version_id = NEW.source_version_id
      AND v.document_id = NEW.source_document_id;
    IF source_current_version_id IS NULL THEN
        RAISE EXCEPTION 'bracket link source version % does not belong to source document %',
            NEW.source_version_id, NEW.source_document_id
            USING ERRCODE = '23514';
    END IF;
    IF source_current_version_id <> NEW.source_version_id THEN
        RAISE EXCEPTION 'bracket link source version % is not current for source document %',
            NEW.source_version_id, NEW.source_document_id
            USING ERRCODE = '23514';
    END IF;
    IF source_document_type <> NEW.source_doc_type THEN
        RAISE EXCEPTION 'bracket link source_doc_type % does not match source document %',
            NEW.source_doc_type, NEW.source_document_id
            USING ERRCODE = '23514';
    END IF;

    source_marker := atelier_bracket_link_source_marker_at(source_body_raw_text, NEW.seq);
    IF source_marker IS NULL OR source_marker <> NEW.raw_marker THEN
        RAISE EXCEPTION 'bracket link projection seq % is not backed by source marker %',
            NEW.seq, NEW.raw_marker
            USING ERRCODE = '23514';
    END IF;

    IF NEW.target_kind = 'character' THEN
        BEGIN
            target_uuid := NEW.target_id::uuid;
        EXCEPTION WHEN invalid_text_representation THEN
            RAISE EXCEPTION 'bracket link character target % must be canonical UUID', NEW.target_id
                USING ERRCODE = '23514';
        END;
        IF target_uuid::text <> NEW.target_id THEN
            RAISE EXCEPTION 'bracket link character target % must be canonical UUID text',
                NEW.target_id
                USING ERRCODE = '23514';
        END IF;
        IF NOT EXISTS (SELECT 1 FROM atelier_character WHERE internal_id = target_uuid) THEN
            RAISE EXCEPTION 'bracket link character target % does not exist', NEW.target_id
                USING ERRCODE = '23514';
        END IF;

        BEGIN
            raw_character_uuid := marker_target_id::uuid;
        EXCEPTION WHEN invalid_text_representation THEN
            raw_character_uuid := NULL;
        END;
        IF raw_character_uuid IS NOT NULL THEN
            SELECT internal_id
            INTO raw_character_id
            FROM atelier_character
            WHERE internal_id = raw_character_uuid;
        END IF;
        IF raw_character_id IS NULL THEN
            SELECT internal_id
            INTO raw_character_id
            FROM atelier_character
            WHERE public_id = marker_target_id
            LIMIT 1;
        END IF;
        IF raw_character_id IS NULL THEN
            RAISE EXCEPTION 'bracket link raw character target % does not exist', marker_target_id
                USING ERRCODE = '23514';
        END IF;
        IF raw_character_id <> target_uuid THEN
            RAISE EXCEPTION 'bracket link raw character target % does not match canonical target %',
                marker_target_id, NEW.target_id
                USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;

    IF NEW.target_kind IN ('document', 'story', 'moodboard') THEN
        BEGIN
            target_uuid := NEW.target_id::uuid;
        EXCEPTION WHEN invalid_text_representation THEN
            RAISE EXCEPTION 'bracket link document target % must be canonical UUID', NEW.target_id
                USING ERRCODE = '23514';
        END;
        IF target_uuid::text <> NEW.target_id THEN
            RAISE EXCEPTION 'bracket link document target % must be canonical UUID text',
                NEW.target_id
                USING ERRCODE = '23514';
        END IF;
        IF marker_target_id::uuid <> target_uuid THEN
            RAISE EXCEPTION 'bracket link raw target % does not match canonical target %',
                marker_target_id, NEW.target_id
                USING ERRCODE = '23514';
        END IF;

        SELECT doc_type
        INTO target_document_type
        FROM atelier_character_document
        WHERE document_id = target_uuid;
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
        RETURN NEW;
    END IF;

    IF NEW.target_kind = 'image' THEN
        BEGIN
            target_uuid := NEW.target_id::uuid;
        EXCEPTION WHEN invalid_text_representation THEN
            RAISE EXCEPTION 'bracket link image target % must be canonical UUID', NEW.target_id
                USING ERRCODE = '23514';
        END;
        IF target_uuid::text <> NEW.target_id THEN
            RAISE EXCEPTION 'bracket link image target % must be canonical UUID text',
                NEW.target_id
                USING ERRCODE = '23514';
        END IF;
        IF marker_target_id::uuid <> target_uuid THEN
            RAISE EXCEPTION 'bracket link raw image target % does not match canonical target %',
                marker_target_id, NEW.target_id
                USING ERRCODE = '23514';
        END IF;
        IF NOT EXISTS (SELECT 1 FROM atelier_media_asset WHERE asset_id = target_uuid) THEN
            RAISE EXCEPTION 'bracket link image target % does not exist', NEW.target_id
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
        seq, raw_marker, target_kind, target_id
    ON atelier_bracket_link_projection
    FOR EACH ROW
    EXECUTE FUNCTION atelier_bracket_link_projection_guard();

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_atelier_bracket_link_projection_guard_v3'
          AND conrelid = 'atelier_bracket_link_projection'::regclass
    ) THEN
        ALTER TABLE atelier_bracket_link_projection
        ADD CONSTRAINT chk_atelier_bracket_link_projection_guard_v3 CHECK (TRUE);
    END IF;
END $$;
