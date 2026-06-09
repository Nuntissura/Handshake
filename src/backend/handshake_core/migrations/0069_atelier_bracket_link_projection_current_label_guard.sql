-- WP-KERNEL-005 MT-041 hardening v5: projection labels are source-backed,
-- and future direct SQL writes must target the current document version.

CREATE OR REPLACE FUNCTION atelier_bracket_link_projection_is_rebuildable(
    p_source_document_id UUID,
    p_source_version_id UUID,
    p_source_doc_type TEXT,
    p_seq BIGINT,
    p_raw_marker TEXT,
    p_target_kind TEXT,
    p_target_id TEXT,
    p_target_label TEXT
)
RETURNS BOOLEAN
LANGUAGE plpgsql
AS $$
DECLARE
    marker_inner TEXT;
    marker_target TEXT;
    marker_kind TEXT;
    marker_target_id TEXT;
    marker_label TEXT;
    canonical_kind TEXT;
    source_document_type TEXT;
    source_body_raw_text TEXT;
    source_marker TEXT;
    target_document_type TEXT;
    raw_character_id UUID;
    raw_character_uuid UUID;
    target_uuid UUID;
BEGIN
    IF p_raw_marker !~ '^\[\[[^\[\]\r\n]+\]\]$' THEN
        RETURN FALSE;
    END IF;

    marker_inner := substring(p_raw_marker FROM 3 FOR char_length(p_raw_marker) - 4);
    IF position('|' IN marker_inner) > 0 THEN
        marker_label := btrim(substr(marker_inner, position('|' IN marker_inner) + 1));
        IF marker_label = '' THEN
            marker_label := NULL;
        END IF;
    ELSE
        marker_label := NULL;
    END IF;
    IF marker_label IS DISTINCT FROM p_target_label THEN
        RETURN FALSE;
    END IF;

    marker_target := split_part(marker_inner, '|', 1);
    IF position(':' IN marker_target) = 0 THEN
        RETURN FALSE;
    END IF;

    marker_kind := lower(btrim(split_part(marker_target, ':', 1)));
    marker_target_id := btrim(substr(marker_target, position(':' IN marker_target) + 1));
    IF marker_target_id = '' THEN
        RETURN FALSE;
    END IF;

    canonical_kind := CASE marker_kind
        WHEN 'char' THEN 'character'
        WHEN 'doc' THEN 'document'
        WHEN 'img' THEN 'image'
        WHEN 'media' THEN 'image'
        ELSE marker_kind
    END;
    IF canonical_kind <> p_target_kind THEN
        RETURN FALSE;
    END IF;

    SELECT d.doc_type, v.body_raw_text
    INTO source_document_type, source_body_raw_text
    FROM atelier_character_document_version v
    JOIN atelier_character_document d ON d.document_id = v.document_id
    WHERE v.version_id = p_source_version_id
      AND v.document_id = p_source_document_id;
    IF source_document_type IS NULL OR source_document_type <> p_source_doc_type THEN
        RETURN FALSE;
    END IF;

    source_marker := atelier_bracket_link_source_marker_at(source_body_raw_text, p_seq);
    IF source_marker IS NULL OR source_marker <> p_raw_marker THEN
        RETURN FALSE;
    END IF;

    IF p_target_kind = 'character' THEN
        target_uuid := p_target_id::uuid;
        IF target_uuid::text <> p_target_id THEN
            RETURN FALSE;
        END IF;
        IF NOT EXISTS (SELECT 1 FROM atelier_character WHERE internal_id = target_uuid) THEN
            RETURN FALSE;
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
        RETURN raw_character_id = target_uuid;
    END IF;

    IF p_target_kind IN ('document', 'story', 'moodboard') THEN
        target_uuid := p_target_id::uuid;
        IF target_uuid::text <> p_target_id THEN
            RETURN FALSE;
        END IF;
        IF marker_target_id::uuid <> target_uuid THEN
            RETURN FALSE;
        END IF;

        SELECT doc_type
        INTO target_document_type
        FROM atelier_character_document
        WHERE document_id = target_uuid;
        IF target_document_type IS NULL THEN
            RETURN FALSE;
        END IF;
        IF p_target_kind = 'story' AND target_document_type <> 'story' THEN
            RETURN FALSE;
        END IF;
        IF p_target_kind = 'moodboard' AND target_document_type <> 'moodboard' THEN
            RETURN FALSE;
        END IF;
        RETURN TRUE;
    END IF;

    IF p_target_kind = 'image' THEN
        target_uuid := p_target_id::uuid;
        IF target_uuid::text <> p_target_id THEN
            RETURN FALSE;
        END IF;
        IF marker_target_id::uuid <> target_uuid THEN
            RETURN FALSE;
        END IF;
        RETURN EXISTS (SELECT 1 FROM atelier_media_asset WHERE asset_id = target_uuid);
    END IF;

    RETURN FALSE;
EXCEPTION WHEN others THEN
    RETURN FALSE;
END;
$$;

CREATE OR REPLACE FUNCTION atelier_bracket_link_projection_is_current_rebuildable(
    p_source_document_id UUID,
    p_source_version_id UUID,
    p_source_doc_type TEXT,
    p_seq BIGINT,
    p_raw_marker TEXT,
    p_target_kind TEXT,
    p_target_id TEXT,
    p_target_label TEXT
)
RETURNS BOOLEAN
LANGUAGE plpgsql
AS $$
DECLARE
    source_current_version_id UUID;
BEGIN
    SELECT current_version_id
    INTO source_current_version_id
    FROM atelier_character_document
    WHERE document_id = p_source_document_id;

    RETURN source_current_version_id = p_source_version_id
       AND atelier_bracket_link_projection_is_rebuildable(
           p_source_document_id,
           p_source_version_id,
           p_source_doc_type,
           p_seq,
           p_raw_marker,
           p_target_kind,
           p_target_id,
           p_target_label
       );
EXCEPTION WHEN others THEN
    RETURN FALSE;
END;
$$;

DELETE FROM atelier_bracket_link_projection p
WHERE NOT atelier_bracket_link_projection_is_rebuildable(
    p.source_document_id,
    p.source_version_id,
    p.source_doc_type,
    p.seq,
    p.raw_marker,
    p.target_kind,
    p.target_id,
    p.target_label
);

CREATE OR REPLACE FUNCTION atelier_bracket_link_projection_guard()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT atelier_bracket_link_projection_is_current_rebuildable(
        NEW.source_document_id,
        NEW.source_version_id,
        NEW.source_doc_type,
        NEW.seq,
        NEW.raw_marker,
        NEW.target_kind,
        NEW.target_id,
        NEW.target_label
    ) THEN
        RAISE EXCEPTION 'bracket link projection row is not rebuildable from current source'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_atelier_bracket_link_projection_guard
    ON atelier_bracket_link_projection;
CREATE TRIGGER trg_atelier_bracket_link_projection_guard
    BEFORE INSERT OR UPDATE OF source_document_id, source_version_id, source_doc_type,
        seq, raw_marker, target_kind, target_id, target_label
    ON atelier_bracket_link_projection
    FOR EACH ROW
    EXECUTE FUNCTION atelier_bracket_link_projection_guard();

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'chk_atelier_bracket_link_projection_guard_v5'
          AND conrelid = 'atelier_bracket_link_projection'::regclass
    ) THEN
        ALTER TABLE atelier_bracket_link_projection
        ADD CONSTRAINT chk_atelier_bracket_link_projection_guard_v5 CHECK (TRUE);
    END IF;
END $$;
