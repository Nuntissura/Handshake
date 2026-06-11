-- WP-KERNEL-009 MT-087 rollback.
DELETE FROM knowledge_schema_registry WHERE family_key = 'ingestion_spans';
DROP TABLE IF EXISTS knowledge_ingestion_spans;
