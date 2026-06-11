-- WP-KERNEL-009 MT-059 rollback (MT-063 RollbackAndRepairMigrations).

DROP TABLE IF EXISTS knowledge_editor_code_nodes;
DROP TABLE IF EXISTS knowledge_rich_document_versions;
DROP TABLE IF EXISTS knowledge_rich_documents;
DELETE FROM knowledge_schema_registry
WHERE family_key IN ('rich_documents', 'rich_document_versions', 'editor_code_nodes');
