-- WP-KERNEL-009 CodeIndexingAndNavigation MT-105 ScipLspImportBoundary.
-- Master Spec anchor: 2.3.13.11 "LSP/SCIP-style precision is a Handshake-owned
-- import/projection path, NOT an outside service requirement".
--
-- knowledge_code_scip_imports is the durable ledger of OWNED imports of
-- SCIP/LSIF-style index artifacts. Handshake never spawns an LSP server or a
-- SCIP indexer; an operator (or an external offline tool) produces a SCIP/LSIF
-- artifact, and this boundary PARSES that provided artifact into knowledge
-- records (symbols + occurrences become entities/spans/edges through
-- storage::knowledge::KnowledgeStore). Each import attempt is recorded here
-- with its status and a typed reason, so a rejected/partial import is auditable
-- and never silently dropped. This path is gated/optional: the core index
-- (MT-097..MT-104, Tree-sitter) does not depend on it.

CREATE TABLE IF NOT EXISTS knowledge_code_scip_imports (
    scip_import_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- Import artifact format.
    artifact_format TEXT NOT NULL CHECK (artifact_format IN ('scip', 'lsif')),
    -- The tool that produced the artifact, as declared in it or by the caller
    -- (provenance only; never executed).
    tool_name TEXT,
    tool_version TEXT,
    -- sha256 of the imported artifact bytes (fidelity + dedupe).
    artifact_hash TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN (
        'imported', 'partial', 'rejected'
    )),
    -- Required reason for non-imported outcomes (typed, never silent).
    reason TEXT,
    symbols_imported INTEGER NOT NULL DEFAULT 0 CHECK (symbols_imported >= 0),
    occurrences_imported INTEGER NOT NULL DEFAULT 0 CHECK (occurrences_imported >= 0),
    edges_imported INTEGER NOT NULL DEFAULT 0 CHECK (edges_imported >= 0),
    import_detail JSONB,
    imported_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    import_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_code_scip_imports_id
        CHECK (scip_import_id ~ '^KSCIP-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_code_scip_imports_hash
        CHECK (artifact_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_knowledge_code_scip_imports_reason_shape
        CHECK (status = 'imported' OR reason IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_code_scip_imports_workspace
    ON knowledge_code_scip_imports (workspace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_knowledge_code_scip_imports_status
    ON knowledge_code_scip_imports (status);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('code_scip_imports', 'knowledge_code_scip_imports', 'KnowledgeEdge',
     'support', '0171_knowledge_code_scip_imports.sql', 'MT-105')
ON CONFLICT (family_key) DO NOTHING;
