-- WP-KERNEL-009 MT-254 DebugAdapterCore — durable per-document breakpoints.
-- Master Spec anchor: 2.3.13.11 (Project Knowledge Index & Rich Document
-- authority); DEC-007 reverses the ED-WB-011 descope and ships a real DAP core.
--
-- Breakpoints set in a RichDocument's Monaco code blocks are operator state that
-- must survive a tab/app restart. Under the Handshake-native durability rule
-- (PostgreSQL + EventLedger is canonical, UI artifacts are projections) they are
-- persisted here keyed by rich_document_id, mirroring 0328
-- (knowledge_rich_document_drafts). One row per (document, source_url, line) so a
-- breakpoint is idempotently upsertable and individually removable.

CREATE TABLE IF NOT EXISTS knowledge_debug_breakpoints (
    breakpoint_id TEXT PRIMARY KEY,
    rich_document_id TEXT NOT NULL
        REFERENCES knowledge_rich_documents(rich_document_id)
        ON UPDATE RESTRICT ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id)
        ON UPDATE RESTRICT ON DELETE CASCADE,
    -- The code-block source the breakpoint lives in (block source url / path).
    source_url TEXT NOT NULL CHECK (btrim(source_url) = source_url AND source_url <> ''),
    line INTEGER NOT NULL CHECK (line >= 1),
    -- Optional condition expression evaluated by the adapter.
    condition TEXT,
    -- Whether the adapter verified (bound) this breakpoint in the last session.
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Receipt for the persistence write (EventLedger is canonical).
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    CONSTRAINT uq_knowledge_debug_breakpoints_doc_src_line
        UNIQUE (rich_document_id, source_url, line)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_debug_breakpoints_document
    ON knowledge_debug_breakpoints (rich_document_id, source_url, line);

CREATE INDEX IF NOT EXISTS idx_knowledge_debug_breakpoints_event
    ON knowledge_debug_breakpoints (event_ledger_event_id);

INSERT INTO knowledge_schema_registry (
    family_key, table_name, record_family, authority_class, migration_file, mt_id
) VALUES
    ('debug_breakpoints', 'knowledge_debug_breakpoints',
     'DebugBreakpoints', 'support',
     '0331_debug_breakpoints.sql', 'MT-254')
ON CONFLICT (family_key) DO NOTHING;
