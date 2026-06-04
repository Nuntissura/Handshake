-- WP-KERNEL-005 atelier foundation (CKC fold-in).
-- PostgreSQL/EventLedger authority only. SQLite is forbidden (MT-004).
-- Characters with stable public id separate from internal id (MT-006),
-- append-only sheet versions (MT-012), content-hash media assets (MT-015),
-- and the atelier event family table (MT-005).

CREATE TABLE IF NOT EXISTS atelier_character (
    internal_id   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    public_id     TEXT NOT NULL UNIQUE,
    display_name  TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS atelier_sheet_version (
    version_id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    character_internal_id UUID NOT NULL REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    parent_version_id     UUID REFERENCES atelier_sheet_version(version_id),
    seq                   BIGINT NOT NULL,
    raw_text              TEXT NOT NULL,
    author                TEXT NOT NULL,
    tool                  TEXT,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (character_internal_id, seq)
);

CREATE TABLE IF NOT EXISTS atelier_media_asset (
    asset_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    content_hash      TEXT NOT NULL UNIQUE,
    mime              TEXT NOT NULL,
    byte_len          BIGINT NOT NULL,
    source_provenance TEXT,
    artifact_ref      TEXT NOT NULL,
    created_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS atelier_event (
    event_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_family   TEXT NOT NULL,
    aggregate_type TEXT NOT NULL,
    aggregate_id   TEXT NOT NULL,
    payload        JSONB NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_event_aggregate ON atelier_event(aggregate_type, aggregate_id);
