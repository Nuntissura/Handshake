-- WP-KERNEL-009 MT-085 rollback.
DELETE FROM knowledge_schema_registry WHERE family_key = 'ingestion_receipts';
DROP TABLE IF EXISTS knowledge_ingestion_receipts;
