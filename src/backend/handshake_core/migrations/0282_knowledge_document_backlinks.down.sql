-- WP-KERNEL-009 MT-155 rollback (MT-063 RollbackAndRepairMigrations).

DROP TABLE IF EXISTS knowledge_document_backlinks;
DELETE FROM knowledge_schema_registry WHERE family_key = 'document_backlinks';
