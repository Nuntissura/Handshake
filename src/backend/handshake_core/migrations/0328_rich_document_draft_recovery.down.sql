-- WP-KERNEL-009 MT-255 rollback.

DROP TABLE IF EXISTS knowledge_rich_document_drafts;
DELETE FROM knowledge_schema_registry
WHERE family_key = 'rich_document_drafts';
