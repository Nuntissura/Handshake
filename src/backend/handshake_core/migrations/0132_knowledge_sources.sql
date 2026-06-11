-- WP-KERNEL-009 MT-051 PostgresEventLedgerCore-051-ProjectSourceFileTables.
-- Master Spec anchor: 2.3.13.11 KnowledgeSource ("stable source id, source
-- kind, content hash, provenance, permission scope, redaction state, and
-- last-index receipt").
--
-- knowledge_sources persists the per-source records under managed roots:
-- files, assets, rich documents, Loom blocks, external imports, and
-- operator-provided artifacts. File-kind sources are addressed by a
-- root-relative portable path; native record kinds reference their existing
-- authority rows (assets, loom_blocks, documents) via FKs so the knowledge
-- index can never cite a record that does not exist.
--
-- MT-051 contract fields: source kinds, content hashes (SHA-256), timestamps,
-- parser status, extraction status, stale markers, last-index receipt ref
-- (FK into kernel_event_ledger).

CREATE TABLE IF NOT EXISTS knowledge_sources (
    source_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    root_id TEXT REFERENCES knowledge_source_roots(root_id) ON DELETE CASCADE,
    source_kind TEXT NOT NULL CHECK (source_kind IN (
        'file', 'asset', 'rich_document', 'loom_block',
        'external_import', 'operator_artifact'
    )),
    -- Root-relative portable path (file kind; same normalization rules as
    -- knowledge_source_roots.repo_relative_path).
    relative_path TEXT,
    -- Native authority refs for non-file kinds.
    asset_id TEXT REFERENCES assets(asset_id) ON DELETE CASCADE,
    loom_block_id TEXT REFERENCES loom_blocks(block_id) ON DELETE CASCADE,
    document_id TEXT REFERENCES documents(id) ON DELETE CASCADE,
    content_hash TEXT NOT NULL,
    size_bytes BIGINT,
    provenance JSONB NOT NULL DEFAULT '{}'::jsonb,
    permission_scope TEXT NOT NULL DEFAULT 'workspace'
        CHECK (permission_scope IN ('workspace', 'operator_private', 'shared')),
    redaction_state TEXT NOT NULL DEFAULT 'none'
        CHECK (redaction_state IN ('none', 'partial', 'redacted')),
    parser_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (parser_status IN ('pending', 'parsed', 'failed', 'skipped')),
    extraction_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (extraction_status IN ('pending', 'extracted', 'failed', 'skipped')),
    stale BOOLEAN NOT NULL DEFAULT FALSE,
    -- Last successful index receipt: FK into the EventLedger so indexing
    -- evidence is always a replayable kernel event, never prose.
    last_index_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    source_modified_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_sources_source_id
        CHECK (source_id ~ '^KSRC-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_sources_content_hash
        CHECK (content_hash ~ '^[0-9a-f]{64}$'),
    -- File-kind sources must carry a root and a portable relative path.
    CONSTRAINT chk_knowledge_sources_file_shape
        CHECK (source_kind <> 'file'
               OR (root_id IS NOT NULL AND relative_path IS NOT NULL)),
    -- Native record kinds must carry their authority ref.
    CONSTRAINT chk_knowledge_sources_asset_shape
        CHECK (source_kind <> 'asset' OR asset_id IS NOT NULL),
    CONSTRAINT chk_knowledge_sources_loom_shape
        CHECK (source_kind <> 'loom_block' OR loom_block_id IS NOT NULL),
    CONSTRAINT chk_knowledge_sources_path_portable
        CHECK (
            relative_path IS NULL
            OR (
                btrim(relative_path) = relative_path
                AND relative_path <> ''
                AND relative_path !~ '^[A-Za-z]:'
                AND relative_path !~ '^[/\\]'
                AND relative_path !~ '\\'
                AND relative_path !~ '(^|/)\.\.(/|$)'
            )
        )
);

-- One source row per (root, path) for files: re-indexing updates in place.
CREATE UNIQUE INDEX IF NOT EXISTS uq_knowledge_sources_root_path
    ON knowledge_sources (root_id, relative_path)
    WHERE relative_path IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_knowledge_sources_workspace
    ON knowledge_sources (workspace_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_sources_root
    ON knowledge_sources (root_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_sources_stale
    ON knowledge_sources (stale) WHERE stale;

CREATE INDEX IF NOT EXISTS idx_knowledge_sources_content_hash
    ON knowledge_sources (content_hash);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('sources', 'knowledge_sources', 'KnowledgeSource',
     'authority', '0132_knowledge_sources.sql', 'MT-051')
ON CONFLICT (family_key) DO NOTHING;
