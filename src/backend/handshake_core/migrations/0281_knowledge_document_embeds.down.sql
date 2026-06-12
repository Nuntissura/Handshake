-- WP-KERNEL-009 MT-152 rollback (MT-063 RollbackAndRepairMigrations).

DROP TABLE IF EXISTS knowledge_document_embeds;
DELETE FROM knowledge_schema_registry WHERE family_key = 'document_embeds';
