-- WP-KERNEL-009 MT-058 PostgresEventLedgerCore-058-WikiProjectionTables.
-- Master Spec anchor: 2.3.13.11 ("Generated markdown, wiki pages, HTML,
-- context bundles, trace reports, visual-debug exports, and operator
-- summaries are projections only. Deleting or editing a projection MUST NOT
-- mutate authority records. A projection MAY contain stable refs into
-- authority records but MUST NOT become the only location where knowledge,
-- claims, edges, or manual instructions exist.").
--
-- PROJECTIONS ARE NEVER AUTHORITY. Structural enforcement:
--   * registered with authority_class = 'projection' in the schema registry;
--   * NO authority table carries any FK or column referencing this table —
--     the dependency arrow points one way (projection -> authority refs in
--     source_records JSONB);
--   * deleting a projection row touches nothing else (no CASCADE leaves this
--     table toward authority records);
--   * regenerability is tracked via rebuild_status + staleness_hash so a
--     deleted/stale projection is simply rebuilt from canonical records.

CREATE TABLE IF NOT EXISTS knowledge_wiki_projections (
    projection_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    projection_kind TEXT NOT NULL CHECK (projection_kind IN (
        'wiki_page', 'loom_view', 'graph_view', 'manual_page', 'operator_summary'
    )),
    title TEXT NOT NULL,
    -- Stable refs into authority records this projection renders:
    -- [{"record_family": "KnowledgeClaim", "record_id": "KCL-..."}, ...].
    source_records JSONB NOT NULL DEFAULT '[]'::jsonb,
    -- The rendered, regenerable content. NEVER authority.
    rendered_content TEXT NOT NULL DEFAULT '',
    rebuild_status TEXT NOT NULL DEFAULT 'stale'
        CHECK (rebuild_status IN ('fresh', 'stale', 'rebuilding', 'failed')),
    -- Hash over the source records' content hashes at render time; a
    -- mismatch against current authority state marks the projection stale.
    staleness_hash TEXT NOT NULL,
    rebuild_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    last_rebuilt_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_wiki_projections_id
        CHECK (projection_id ~ '^KWP-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_wiki_projections_title
        CHECK (btrim(title) = title AND title <> ''),
    CONSTRAINT chk_knowledge_wiki_projections_staleness_hash
        CHECK (staleness_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT uq_knowledge_wiki_projections_identity
        UNIQUE (workspace_id, projection_kind, title)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_wiki_projections_status
    ON knowledge_wiki_projections (workspace_id, rebuild_status);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('wiki_projections', 'knowledge_wiki_projections', 'Projection',
     'projection', '0139_knowledge_wiki_projections.sql', 'MT-058')
ON CONFLICT (family_key) DO NOTHING;
