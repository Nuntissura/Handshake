-- WP-KERNEL-005 MT-044: character relationships and graph projection.
-- Relationships are character-to-character edges with endpoint validation.

CREATE TABLE IF NOT EXISTS atelier_character_relationship (
    relationship_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_character_id  UUID NOT NULL REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    target_character_id  UUID NOT NULL REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    relationship_kind    TEXT NOT NULL,
    label                TEXT,
    notes                TEXT NOT NULL DEFAULT '',
    created_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_character_relationship_distinct_endpoints
        CHECK (source_character_id <> target_character_id),
    CONSTRAINT chk_atelier_character_relationship_kind_trimmed
        CHECK (btrim(relationship_kind) = relationship_kind AND relationship_kind <> ''),
    CONSTRAINT chk_atelier_character_relationship_label_trimmed
        CHECK (label IS NULL OR (btrim(label) = label AND label <> '')),
    CONSTRAINT chk_atelier_character_relationship_notes_trimmed
        CHECK (btrim(notes) = notes),
    CONSTRAINT uq_atelier_character_relationship_edge
        UNIQUE (source_character_id, target_character_id, relationship_kind)
);

CREATE INDEX IF NOT EXISTS idx_atelier_character_relationship_source
    ON atelier_character_relationship(source_character_id, updated_at_utc DESC);
CREATE INDEX IF NOT EXISTS idx_atelier_character_relationship_target
    ON atelier_character_relationship(target_character_id, updated_at_utc DESC);

CREATE OR REPLACE VIEW atelier_character_relationship_graph_projection AS
SELECT
    relationship_id AS edge_id,
    source_character_id,
    target_character_id,
    relationship_kind,
    label,
    notes,
    updated_at_utc
FROM atelier_character_relationship;
