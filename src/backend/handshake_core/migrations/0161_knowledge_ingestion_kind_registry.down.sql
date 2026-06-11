-- WP-KERNEL-009 MT-082 rollback.
DELETE FROM knowledge_schema_registry WHERE family_key = 'ingestion_kind_registry';
DROP TABLE IF EXISTS knowledge_ingestion_kind_registry;
