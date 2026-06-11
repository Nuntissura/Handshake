-- WP-KERNEL-009 CodeIndexingAndNavigation MT-098..MT-108 support table.
-- Master Spec anchor: 2.3.13.11 "code navigation must use Handshake-managed
-- parsers/adapters and store spans/citations in PostgreSQL" + KnowledgeSource
-- staleness ("mark stale, never serve stale silently").
--
-- knowledge_code_files is the per-code-file INDEX STATE that the code-index
-- engine maintains alongside the shared knowledge_sources row. It records the
-- exact content hash + parser version the current symbols/spans/edges were
-- produced from, so:
--   * MT-107 staleness can mark a file stale when the source hash OR the parser
--     version changes (re-index needed), and the nav API can refuse to serve
--     stale results silently;
--   * MT-108 partial-failure can record a precise per-file parse status
--     (parsed | failed | partial) without failing the whole run.
--
-- The symbols/spans/edges themselves live in the shared knowledge_entities /
-- knowledge_spans / knowledge_edges tables (0134-0136) and are written ONLY
-- through storage::knowledge::KnowledgeStore. This table never duplicates that
-- authority; it is the code-index bookkeeping that links a source file to the
-- run that last indexed it.

CREATE TABLE IF NOT EXISTS knowledge_code_files (
    code_file_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- The shared KnowledgeSource this code file is (one code-file row per
    -- source); unique so re-index upserts in place.
    source_id TEXT NOT NULL
        REFERENCES knowledge_sources(source_id) ON DELETE CASCADE,
    -- The `file` KnowledgeEntity that represents this file in the graph.
    file_entity_id TEXT
        REFERENCES knowledge_entities(entity_id) ON DELETE SET NULL,
    language TEXT NOT NULL CHECK (language IN (
        'rust', 'javascript', 'typescript', 'tsx'
    )),
    -- Content hash (raw sha256) the current index reflects.
    indexed_content_hash TEXT NOT NULL,
    -- Parser-version receipt the current index was produced with (MT-097).
    parser_version TEXT NOT NULL,
    -- Per-file parse outcome (MT-108): parsed (clean), partial (tree had
    -- syntax errors but symbols recovered), failed (no usable parse).
    parse_status TEXT NOT NULL DEFAULT 'parsed' CHECK (parse_status IN (
        'parsed', 'partial', 'failed'
    )),
    -- True when the source hash or parser version moved past indexed_* (MT-107).
    stale BOOLEAN NOT NULL DEFAULT FALSE,
    symbols_indexed INTEGER NOT NULL DEFAULT 0 CHECK (symbols_indexed >= 0),
    edges_indexed INTEGER NOT NULL DEFAULT 0 CHECK (edges_indexed >= 0),
    -- Optional typed failure detail for parse_status = failed/partial.
    failure_detail JSONB,
    last_indexed_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    last_index_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_code_files_id
        CHECK (code_file_id ~ '^KCF-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_code_files_hash
        CHECK (indexed_content_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_knowledge_code_files_parser_version
        CHECK (btrim(parser_version) = parser_version AND parser_version <> ''),
    CONSTRAINT uq_knowledge_code_files_source UNIQUE (source_id)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_code_files_workspace
    ON knowledge_code_files (workspace_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_code_files_stale
    ON knowledge_code_files (workspace_id, stale);

CREATE INDEX IF NOT EXISTS idx_knowledge_code_files_parse_status
    ON knowledge_code_files (workspace_id, parse_status);

CREATE INDEX IF NOT EXISTS idx_knowledge_code_files_file_entity
    ON knowledge_code_files (file_entity_id);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('code_files', 'knowledge_code_files', 'KnowledgeSource',
     'support', '0170_knowledge_code_files.sql', 'MT-107')
ON CONFLICT (family_key) DO NOTHING;
