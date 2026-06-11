-- WP-KERNEL-005 MT-039: ordered story cards and separate StoryBeat records.
-- PostgreSQL preserves stable ids, round-trip order, and raw text.

CREATE TABLE IF NOT EXISTS atelier_story_card (
    card_id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    story_document_id   UUID NOT NULL REFERENCES atelier_character_document(document_id) ON DELETE CASCADE,
    seq                 BIGINT NOT NULL,
    title               TEXT NOT NULL,
    body_raw_text       TEXT NOT NULL,
    tags_json           JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at_utc      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_story_card_tags_array
        CHECK (jsonb_typeof(tags_json) = 'array'),
    UNIQUE (story_document_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_atelier_story_card_story
    ON atelier_story_card(story_document_id, seq ASC);

CREATE TABLE IF NOT EXISTS atelier_story_beat (
    beat_id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    story_document_id   UUID NOT NULL REFERENCES atelier_character_document(document_id) ON DELETE CASCADE,
    card_id             UUID REFERENCES atelier_story_card(card_id) ON DELETE SET NULL,
    seq                 BIGINT NOT NULL,
    beat_text           TEXT NOT NULL,
    created_at_utc      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (story_document_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_atelier_story_beat_story
    ON atelier_story_beat(story_document_id, seq ASC);

CREATE INDEX IF NOT EXISTS idx_atelier_story_beat_card
    ON atelier_story_beat(card_id, seq ASC);
