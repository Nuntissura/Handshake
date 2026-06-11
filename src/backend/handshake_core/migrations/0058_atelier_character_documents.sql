-- WP-KERNEL-005 MT-038: character-scoped note/story/moodboard documents with
-- title/body/tags/type and append-only versioning. PostgreSQL authority only.

CREATE TABLE IF NOT EXISTS atelier_character_document (
    document_id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    character_internal_id UUID NOT NULL REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    doc_type              TEXT NOT NULL,
    title                 TEXT NOT NULL,
    tags_json             JSONB NOT NULL DEFAULT '[]'::jsonb,
    current_version_id    UUID,
    current_version_seq   BIGINT NOT NULL DEFAULT 0,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_character_document_type
        CHECK (doc_type IN ('note', 'story', 'moodboard')),
    CONSTRAINT chk_atelier_character_document_tags_array
        CHECK (jsonb_typeof(tags_json) = 'array')
);

CREATE INDEX IF NOT EXISTS idx_atelier_character_document_character
    ON atelier_character_document(character_internal_id, doc_type, updated_at_utc DESC);

CREATE TABLE IF NOT EXISTS atelier_character_document_version (
    version_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id       UUID NOT NULL REFERENCES atelier_character_document(document_id) ON DELETE CASCADE,
    version_seq       BIGINT NOT NULL,
    title             TEXT NOT NULL,
    body_raw_text     TEXT NOT NULL,
    tags_json         JSONB NOT NULL DEFAULT '[]'::jsonb,
    author            TEXT NOT NULL,
    parent_version_id UUID REFERENCES atelier_character_document_version(version_id),
    created_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_character_document_version_tags_array
        CHECK (jsonb_typeof(tags_json) = 'array'),
    UNIQUE (document_id, version_seq)
);

CREATE INDEX IF NOT EXISTS idx_atelier_character_document_version_document
    ON atelier_character_document_version(document_id, version_seq ASC);
