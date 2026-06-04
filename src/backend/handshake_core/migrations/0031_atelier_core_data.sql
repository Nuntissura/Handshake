-- WP-KERNEL-005 Core-Data domain tables (intake, collections, search/tags/similarity,
-- exports, annotation, settings). PostgreSQL authority only; idempotent.

-- intake (MT-016): persistent batches + per-item accept/reject/defer lanes.
CREATE TABLE IF NOT EXISTS atelier_intake_batch (
    batch_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    idempotency_key       TEXT NOT NULL UNIQUE,
    source_label          TEXT NOT NULL,
    character_internal_id UUID REFERENCES atelier_character(internal_id) ON DELETE SET NULL,
    status                TEXT NOT NULL DEFAULT 'open',
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_intake_batch_status ON atelier_intake_batch(status, updated_at_utc DESC);
CREATE INDEX IF NOT EXISTS idx_atelier_intake_batch_character ON atelier_intake_batch(character_internal_id);

CREATE TABLE IF NOT EXISTS atelier_intake_item (
    item_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    batch_id       UUID NOT NULL REFERENCES atelier_intake_batch(batch_id) ON DELETE CASCADE,
    source_path    TEXT NOT NULL,
    file_name      TEXT NOT NULL,
    byte_len       BIGINT NOT NULL DEFAULT 0,
    content_hash   TEXT,
    lane           TEXT NOT NULL DEFAULT 'new',
    lane_reason    TEXT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (batch_id, source_path)
);
CREATE INDEX IF NOT EXISTS idx_atelier_intake_item_batch_lane ON atelier_intake_item(batch_id, lane);
CREATE INDEX IF NOT EXISTS idx_atelier_intake_item_hash ON atelier_intake_item(content_hash);

-- collections + contact sheets (MT-018).
CREATE TABLE IF NOT EXISTS atelier_collection (
    collection_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                  TEXT NOT NULL UNIQUE,
    notes                 TEXT NOT NULL DEFAULT '',
    tags_json             JSONB NOT NULL DEFAULT '[]'::jsonb,
    character_internal_id UUID REFERENCES atelier_character(internal_id) ON DELETE SET NULL,
    sheet_version_id      UUID REFERENCES atelier_sheet_version(version_id) ON DELETE SET NULL,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS atelier_collection_item (
    collection_id UUID NOT NULL REFERENCES atelier_collection(collection_id) ON DELETE CASCADE,
    asset_id      UUID NOT NULL REFERENCES atelier_media_asset(asset_id) ON DELETE CASCADE,
    sort_order    BIGINT NOT NULL DEFAULT 0,
    added_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (collection_id, asset_id)
);
CREATE INDEX IF NOT EXISTS idx_atelier_collection_item_order ON atelier_collection_item(collection_id, sort_order, added_at_utc);

CREATE TABLE IF NOT EXISTS atelier_contact_sheet (
    sheet_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                  TEXT NOT NULL DEFAULT '',
    source_type           TEXT NOT NULL DEFAULT 'manual',
    source_id             TEXT,
    tags_json             JSONB NOT NULL DEFAULT '[]'::jsonb,
    character_internal_id UUID REFERENCES atelier_character(internal_id) ON DELETE SET NULL,
    sheet_version_id      UUID REFERENCES atelier_sheet_version(version_id) ON DELETE SET NULL,
    manifest              JSONB NOT NULL DEFAULT '{}'::jsonb,
    image_count           BIGINT NOT NULL DEFAULT 0,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_contact_sheet_source ON atelier_contact_sheet(source_type, source_id);
CREATE INDEX IF NOT EXISTS idx_atelier_contact_sheet_created ON atelier_contact_sheet(created_at_utc DESC);

-- search / tags / similarity (MT-019/020/023).
CREATE TABLE IF NOT EXISTS atelier_tag (
    tag_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    text           TEXT NOT NULL UNIQUE,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS atelier_character_tag (
    character_internal_id UUID NOT NULL REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    tag_id                UUID NOT NULL REFERENCES atelier_tag(tag_id) ON DELETE CASCADE,
    tag_type              TEXT NOT NULL CHECK (tag_type IN ('manual','derived')),
    PRIMARY KEY (character_internal_id, tag_id)
);
CREATE INDEX IF NOT EXISTS idx_atelier_character_tag_tag ON atelier_character_tag(tag_id);
CREATE TABLE IF NOT EXISTS atelier_tag_rule (
    rule_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_field_id TEXT NOT NULL,
    match_type      TEXT NOT NULL CHECK (match_type IN ('equals','contains','regex')),
    pattern         TEXT NOT NULL,
    emit_tag        TEXT NOT NULL,
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    created_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS atelier_similarity_projection (
    asset_internal_id UUID PRIMARY KEY REFERENCES atelier_media_asset(asset_id) ON DELETE CASCADE,
    dhash_hex         TEXT,
    palette_json      JSONB NOT NULL DEFAULT '[]'::jsonb,
    updated_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_similarity_dhash ON atelier_similarity_projection(dhash_hex);

-- exports + share packs (MT-199).
CREATE TABLE IF NOT EXISTS atelier_export_request (
    export_id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    character_internal_id UUID NOT NULL REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    sheet_version_id      UUID NOT NULL REFERENCES atelier_sheet_version(version_id),
    format                TEXT NOT NULL,
    status                TEXT NOT NULL DEFAULT 'pending',
    label                 TEXT,
    requested_by          TEXT NOT NULL,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_export_request_character ON atelier_export_request(character_internal_id);
CREATE TABLE IF NOT EXISTS atelier_export_result (
    result_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    export_id      UUID NOT NULL REFERENCES atelier_export_request(export_id) ON DELETE CASCADE,
    artifact_ref   TEXT NOT NULL,
    content_hash   TEXT NOT NULL,
    byte_len       BIGINT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (export_id, content_hash)
);
CREATE TABLE IF NOT EXISTS atelier_export_manifest_entry (
    entry_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    export_id      UUID NOT NULL REFERENCES atelier_export_request(export_id) ON DELETE CASCADE,
    seq            BIGINT NOT NULL,
    kind           TEXT NOT NULL,
    artifact_ref   TEXT NOT NULL,
    pack_path      TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (export_id, seq)
);
CREATE INDEX IF NOT EXISTS idx_atelier_export_manifest_entry_export ON atelier_export_manifest_entry(export_id);

-- annotation overlays (MT-198).
CREATE TABLE IF NOT EXISTS atelier_media_annotation (
    annotation_id  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id       UUID NOT NULL REFERENCES atelier_media_asset(asset_id) ON DELETE CASCADE,
    kind           TEXT NOT NULL,
    label          TEXT,
    note           TEXT NOT NULL DEFAULT '',
    geometry       JSONB NOT NULL,
    seq            BIGINT NOT NULL,
    author         TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (asset_id, seq)
);
CREATE INDEX IF NOT EXISTS idx_atelier_media_annotation_asset ON atelier_media_annotation(asset_id, seq);

-- settings / preferences (MT-200).
CREATE TABLE IF NOT EXISTS atelier_preference (
    preference_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scope_kind            TEXT NOT NULL CHECK (scope_kind IN ('global', 'character')),
    character_internal_id UUID REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    key                   TEXT NOT NULL,
    value_type            TEXT NOT NULL CHECK (value_type IN ('string', 'bool', 'integer', 'float', 'json', 'path')),
    value                 TEXT NOT NULL,
    redacted              BOOLEAN NOT NULL DEFAULT FALSE,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT atelier_preference_scope_ck CHECK (
        (scope_kind = 'global' AND character_internal_id IS NULL)
        OR (scope_kind = 'character' AND character_internal_id IS NOT NULL)
    ),
    CONSTRAINT atelier_preference_scope_key_uq
        UNIQUE NULLS NOT DISTINCT (scope_kind, character_internal_id, key)
);
CREATE INDEX IF NOT EXISTS idx_atelier_preference_scope ON atelier_preference(scope_kind, character_internal_id);
