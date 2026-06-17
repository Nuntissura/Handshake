-- WP-KERNEL-009 MT-255 durable RichDocument draft recovery.
-- Unsaved editor content is backend-persisted per RichDocument so a tab/app
-- crash can offer restore/discard on reopen without relying on localStorage.

CREATE TABLE IF NOT EXISTS knowledge_rich_document_drafts (
    rich_document_id TEXT PRIMARY KEY
        REFERENCES knowledge_rich_documents(rich_document_id)
        ON UPDATE RESTRICT ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id)
        ON UPDATE RESTRICT ON DELETE CASCADE,
    base_doc_version BIGINT NOT NULL CHECK (base_doc_version >= 1),
    base_content_sha256 TEXT NOT NULL CHECK (base_content_sha256 ~ '^[0-9a-f]{64}$'),
    draft_content_json JSONB NOT NULL,
    draft_content_sha256 TEXT NOT NULL CHECK (draft_content_sha256 ~ '^[0-9a-f]{64}$'),
    actor_kind TEXT NOT NULL CHECK (actor_kind IN (
        'operator', 'local_model', 'cloud_model', 'validator', 'system', 'unauthenticated'
    )),
    actor_id TEXT NOT NULL CHECK (btrim(actor_id) = actor_id AND actor_id <> ''),
    kernel_task_run_id TEXT NOT NULL CHECK (
        btrim(kernel_task_run_id) = kernel_task_run_id AND kernel_task_run_id <> ''
    ),
    session_run_id TEXT NOT NULL CHECK (
        btrim(session_run_id) = session_run_id AND session_run_id <> ''
    ),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_knowledge_rich_document_drafts_workspace
    ON knowledge_rich_document_drafts (workspace_id, updated_at DESC);

INSERT INTO knowledge_schema_registry (
    family_key, table_name, record_family, authority_class, migration_file, mt_id
) VALUES
    ('rich_document_drafts', 'knowledge_rich_document_drafts',
     'RichDocumentDraftRecovery', 'support',
     '0328_rich_document_draft_recovery.sql', 'MT-255')
ON CONFLICT (family_key) DO NOTHING;
