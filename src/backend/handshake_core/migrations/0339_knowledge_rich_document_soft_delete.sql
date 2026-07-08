-- WP-KERNEL-012 MT-045 (Route 6, DELETE document): a SOFT delete (tombstone) for
-- RichDocuments so a delete preserves EventLedger lineage instead of destroying
-- the authority row. `deleted_at` marks the tombstone; `deleted_receipt_event_id`
-- points at the KNOWLEDGE_RICH_DOCUMENT_DELETED EventLedger receipt that recorded
-- the delete (who/when/why is auditable). Both are NULL for every live document,
-- so the change is additive and back-compatible. The document's knowledge SOURCE
-- is marked stale by the handler (the index unit); no hard row deletion occurs.

ALTER TABLE knowledge_rich_documents
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;

ALTER TABLE knowledge_rich_documents
    ADD COLUMN IF NOT EXISTS deleted_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT;

CREATE INDEX IF NOT EXISTS idx_knowledge_rich_documents_deleted_at
    ON knowledge_rich_documents (deleted_at)
    WHERE deleted_at IS NOT NULL;
