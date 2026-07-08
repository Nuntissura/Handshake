-- WP-KERNEL-012 MT-045 (Route 6 soft delete) — down migration (replay-safe).
-- Drops the tombstone columns. Any soft-deleted documents revert to appearing
-- live (the tombstone metadata is lost); the EventLedger delete receipts remain.

DROP INDEX IF EXISTS idx_knowledge_rich_documents_deleted_at;

ALTER TABLE knowledge_rich_documents
    DROP COLUMN IF EXISTS deleted_receipt_event_id;

ALTER TABLE knowledge_rich_documents
    DROP COLUMN IF EXISTS deleted_at;
