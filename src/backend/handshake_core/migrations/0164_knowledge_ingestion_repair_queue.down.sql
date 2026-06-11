-- WP-KERNEL-009 MT-094 rollback.
DELETE FROM knowledge_schema_registry WHERE family_key = 'ingestion_repair_queue';
DROP TABLE IF EXISTS knowledge_ingestion_repair_queue;
