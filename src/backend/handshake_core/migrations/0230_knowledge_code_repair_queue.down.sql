-- WP-KERNEL-009 MT-108 rollback.
DELETE FROM knowledge_schema_registry WHERE family_key = 'code_repair_queue';
DROP TABLE IF EXISTS knowledge_code_repair_queue;
