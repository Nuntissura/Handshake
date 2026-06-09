-- WP-KERNEL-005 MT-008 typed sheet template parser snapshots.
-- Raw sheet text remains append-only in atelier_sheet_version; this table stores
-- derived typed AST snapshots for later block-list and selective-edit work.

CREATE TABLE IF NOT EXISTS atelier_sheet_parse_snapshot (
    parse_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id      UUID NOT NULL REFERENCES atelier_sheet_version(version_id) ON DELETE CASCADE,
    template_id     TEXT NOT NULL,
    source_path     TEXT,
    template_version TEXT,
    template_hash   TEXT NOT NULL,
    ast             JSONB NOT NULL,
    unmapped_lines  JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (version_id, template_id, template_hash)
);

CREATE INDEX IF NOT EXISTS idx_atelier_sheet_parse_snapshot_version
    ON atelier_sheet_parse_snapshot(version_id);
CREATE INDEX IF NOT EXISTS idx_atelier_sheet_parse_snapshot_template
    ON atelier_sheet_parse_snapshot(template_id, template_hash);
