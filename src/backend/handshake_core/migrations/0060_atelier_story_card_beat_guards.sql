-- WP-KERNEL-005 MT-039 hardening: database-level guards for story card/beat scope.
-- Kept separate from 0059 because 0059 may already be applied in sqlx-managed DBs.

CREATE OR REPLACE FUNCTION atelier_story_card_requires_story_document()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM atelier_character_document
        WHERE document_id = NEW.story_document_id
          AND doc_type = 'story'
    ) THEN
        RAISE EXCEPTION 'story cards require story document %', NEW.story_document_id
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_atelier_story_card_requires_story_document
    ON atelier_story_card;
CREATE TRIGGER trg_atelier_story_card_requires_story_document
    BEFORE INSERT OR UPDATE OF story_document_id
    ON atelier_story_card
    FOR EACH ROW
    EXECUTE FUNCTION atelier_story_card_requires_story_document();

CREATE OR REPLACE FUNCTION atelier_story_beat_card_matches_story_document()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.card_id IS NOT NULL AND NOT EXISTS (
        SELECT 1
        FROM atelier_story_card
        WHERE card_id = NEW.card_id
          AND story_document_id = NEW.story_document_id
    ) THEN
        RAISE EXCEPTION 'story beat card % does not belong to story document %',
            NEW.card_id, NEW.story_document_id
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_atelier_story_beat_card_matches_story_document
    ON atelier_story_beat;
CREATE TRIGGER trg_atelier_story_beat_card_matches_story_document
    BEFORE INSERT OR UPDATE OF card_id, story_document_id
    ON atelier_story_beat
    FOR EACH ROW
    EXECUTE FUNCTION atelier_story_beat_card_matches_story_document();
