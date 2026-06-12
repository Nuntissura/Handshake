-- WP-KERNEL-009 MT-155 DocumentBacklinkBridge (+ MT-154 search-index inputs).
--
-- Master Spec anchor: 2.3.13.11 (relationships carry a stable relationship_id;
-- the knowledge graph is re-derivable). A RichDocument emits typed link
-- references (file/folder/project/spec/wp/symbol link blocks, and inline
-- wikilinks/mentions/tags). MT-155 persists each as a backlink row keyed by a
-- STABLE relationship_id derived from the source document, the link kind, the
-- target, and the originating block id (knowledge_document::backlink::
-- derive_document_link_relationship_id) -- the same length-prefixed injective
-- discipline as knowledge_edges.relationship_id, so re-extracting a document's
-- links re-derives the same ids and a re-save upserts in place instead of
-- duplicating edges.
--
-- These are document-scoped backlinks. They are intentionally a thin, derivable
-- bridge: the row's source of truth is the document content, so a rebuild
-- (delete all rows for a document, re-extract, re-insert) is always safe and
-- idempotent. relationship_id is unique per workspace.

CREATE TABLE IF NOT EXISTS knowledge_document_backlinks (
    backlink_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- Stable, deterministic across re-extraction runs (MT-155).
    relationship_id TEXT NOT NULL,
    source_document_id TEXT NOT NULL
        REFERENCES knowledge_rich_documents(rich_document_id) ON DELETE CASCADE,
    -- 'file'|'folder'|'project'|'spec'|'wp'|'symbol'|'wikilink'|'mention'|'tag'
    link_kind TEXT NOT NULL,
    -- The link target: a typed id/path token, wikilink title, handle, or tag.
    target TEXT NOT NULL,
    -- MT-148 stable block id the reference came from.
    block_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_document_backlinks_id
        CHECK (backlink_id ~ '^KDBL-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_document_backlinks_relationship_id
        CHECK (relationship_id ~ '^KDLNK-[0-9a-f]{64}$'),
    CONSTRAINT chk_knowledge_document_backlinks_link_kind
        CHECK (link_kind IN (
            'file', 'folder', 'project', 'spec', 'wp', 'symbol',
            'wikilink', 'mention', 'tag'
        )),
    CONSTRAINT chk_knowledge_document_backlinks_target
        CHECK (btrim(target) = target AND target <> ''),
    CONSTRAINT chk_knowledge_document_backlinks_block_id
        CHECK (btrim(block_id) = block_id AND block_id <> ''),
    CONSTRAINT uq_knowledge_document_backlinks_relationship
        UNIQUE (workspace_id, relationship_id)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_document_backlinks_source
    ON knowledge_document_backlinks (source_document_id);
-- Reverse lookup: who links TO this target (the backlink direction, MT-155).
CREATE INDEX IF NOT EXISTS idx_knowledge_document_backlinks_target
    ON knowledge_document_backlinks (workspace_id, link_kind, target);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('document_backlinks', 'knowledge_document_backlinks', 'KnowledgeEdge',
     'authority', '0282_knowledge_document_backlinks.sql', 'MT-155')
ON CONFLICT (family_key) DO NOTHING;
