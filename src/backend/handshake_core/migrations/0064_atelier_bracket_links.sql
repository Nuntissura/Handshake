-- WP-KERNEL-005 MT-041: rebuildable bracket-link/backlink projections.
-- Source text stays authoritative in atelier_character_document_version.

CREATE TABLE IF NOT EXISTS atelier_bracket_link_projection (
    link_id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_document_id UUID NOT NULL REFERENCES atelier_character_document(document_id) ON DELETE CASCADE,
    source_version_id  UUID NOT NULL REFERENCES atelier_character_document_version(version_id) ON DELETE CASCADE,
    source_doc_type    TEXT NOT NULL,
    seq                BIGINT NOT NULL,
    raw_marker         TEXT NOT NULL,
    target_kind        TEXT NOT NULL,
    target_id          TEXT NOT NULL,
    target_label       TEXT,
    created_at_utc     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_bracket_link_source_doc_type
        CHECK (source_doc_type IN ('note', 'story', 'moodboard')),
    CONSTRAINT chk_atelier_bracket_link_target_kind
        CHECK (target_kind IN ('character', 'document', 'story', 'moodboard', 'image')),
    CONSTRAINT chk_atelier_bracket_link_seq_positive
        CHECK (seq > 0),
    CONSTRAINT chk_atelier_bracket_link_raw_marker
        CHECK (raw_marker LIKE '[[%]]' AND raw_marker = btrim(raw_marker)),
    CONSTRAINT chk_atelier_bracket_link_target_id
        CHECK (target_id <> '' AND target_id = btrim(target_id)),
    CONSTRAINT chk_atelier_bracket_link_target_label
        CHECK (target_label IS NULL OR (target_label <> '' AND target_label = btrim(target_label))),
    UNIQUE (source_document_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_atelier_bracket_link_source
    ON atelier_bracket_link_projection(source_document_id, seq ASC);

CREATE INDEX IF NOT EXISTS idx_atelier_bracket_link_target
    ON atelier_bracket_link_projection(target_kind, target_id, source_document_id, seq ASC);
