-- WP-KERNEL-009 MT-059 PostgresEventLedgerCore-059-RichDocumentTables.
-- Master Spec anchor: 2.3.13.11 RichDocument ("a versioned ProseMirror/Tiptap
-- document JSON authority record, with schema version, CRDT snapshot refs,
-- EventLedger promotion refs, and projection refs") and EditorCodeNode ("a
-- Monaco-backed code block embedded in a RichDocument, with language id,
-- worker/bundling requirements, source mapping, lint diagnostics, and
-- round-trip hash").
--
-- Versioning model: knowledge_rich_documents holds the CURRENT authority
-- revision (doc_version, content_json, content_sha256);
-- knowledge_rich_document_versions is the append-only version history —
-- every promoted revision lands there with its EventLedger promotion
-- receipt. Saves are optimistic (WHERE doc_version = expected) so concurrent
-- writers fail closed with a typed Conflict instead of overwriting.
--
-- CRDT refs: crdt_document_id/crdt_snapshot_id are soft TEXT refs into
-- kernel_crdt_snapshots, whose PRIMARY KEY is composite
-- (workspace_id, document_id, crdt_document_id, snapshot_id) — a simple FK
-- cannot target it; the kernel CRDT promotion bridge owns that integrity.
--
-- Projection refs: projection_refs JSONB points OUT to projections
-- (knowledge_wiki_projections rows). The reverse direction (projection ->
-- authority) lives only in the projection's source_records JSONB; no FK ever
-- points from this authority table into projection content (MT-058 boundary).

CREATE TABLE IF NOT EXISTS knowledge_rich_documents (
    rich_document_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- Optional anchor to the legacy documents surface.
    document_id TEXT REFERENCES documents(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    -- ProseMirror/Tiptap schema version token (e.g. 'hsk_richdoc_v1').
    schema_version TEXT NOT NULL,
    doc_version BIGINT NOT NULL DEFAULT 1 CHECK (doc_version >= 1),
    -- The document JSON authority (ProseMirror doc node).
    content_json JSONB NOT NULL,
    content_sha256 TEXT NOT NULL,
    -- Soft refs into kernel CRDT storage (composite PK; see header).
    crdt_document_id TEXT,
    crdt_snapshot_id TEXT,
    -- EventLedger promotion receipt for the CURRENT revision.
    promotion_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    -- Outbound projection refs: [{"projection_id": "KWP-..."}, ...].
    projection_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_rich_documents_id
        CHECK (rich_document_id ~ '^KRD-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_rich_documents_title
        CHECK (btrim(title) = title AND title <> ''),
    CONSTRAINT chk_knowledge_rich_documents_schema_version
        CHECK (btrim(schema_version) = schema_version AND schema_version <> ''),
    CONSTRAINT chk_knowledge_rich_documents_content_sha256
        CHECK (content_sha256 ~ '^[0-9a-f]{64}$')
);

CREATE INDEX IF NOT EXISTS idx_knowledge_rich_documents_workspace
    ON knowledge_rich_documents (workspace_id);

-- Append-only promoted revision history.
CREATE TABLE IF NOT EXISTS knowledge_rich_document_versions (
    rich_document_id TEXT NOT NULL
        REFERENCES knowledge_rich_documents(rich_document_id) ON DELETE CASCADE,
    doc_version BIGINT NOT NULL CHECK (doc_version >= 1),
    schema_version TEXT NOT NULL,
    content_json JSONB NOT NULL,
    content_sha256 TEXT NOT NULL,
    crdt_snapshot_id TEXT,
    promotion_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_rich_document_versions_sha
        CHECK (content_sha256 ~ '^[0-9a-f]{64}$'),
    PRIMARY KEY (rich_document_id, doc_version)
);

-- EditorCodeNode payloads embedded in a RichDocument (Monaco-backed).
CREATE TABLE IF NOT EXISTS knowledge_editor_code_nodes (
    code_node_id TEXT PRIMARY KEY,
    rich_document_id TEXT NOT NULL
        REFERENCES knowledge_rich_documents(rich_document_id) ON DELETE CASCADE,
    -- Stable node path inside the document block tree (e.g. 'body.3.code').
    node_path TEXT NOT NULL,
    language_id TEXT NOT NULL,
    code_text TEXT NOT NULL,
    -- SHA-256 over code_text: the editor round-trip integrity hash. A Monaco
    -- mount/unmount cycle must reproduce this hash or the round-trip failed.
    round_trip_sha256 TEXT NOT NULL,
    -- Worker/bundling requirements: {"worker": "ts", "bundled": true, ...}.
    worker_requirements JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Source mapping back into project sources, when the block mirrors one.
    source_mapping JSONB,
    lint_diagnostics JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_editor_code_nodes_id
        CHECK (code_node_id ~ '^KCN-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_editor_code_nodes_node_path
        CHECK (btrim(node_path) = node_path AND node_path <> ''),
    CONSTRAINT chk_knowledge_editor_code_nodes_language
        CHECK (btrim(language_id) = language_id AND language_id <> ''),
    CONSTRAINT chk_knowledge_editor_code_nodes_round_trip
        CHECK (round_trip_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT uq_knowledge_editor_code_nodes_path
        UNIQUE (rich_document_id, node_path)
);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('rich_documents', 'knowledge_rich_documents', 'RichDocument',
     'authority', '0140_knowledge_rich_documents.sql', 'MT-059'),
    ('rich_document_versions', 'knowledge_rich_document_versions', 'RichDocument',
     'authority', '0140_knowledge_rich_documents.sql', 'MT-059'),
    ('editor_code_nodes', 'knowledge_editor_code_nodes', 'EditorCodeNode',
     'authority', '0140_knowledge_rich_documents.sql', 'MT-059')
ON CONFLICT (family_key) DO NOTHING;
