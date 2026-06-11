-- WP-KERNEL-009 MT-081 rollback.
DELETE FROM knowledge_schema_registry
    WHERE family_key IN ('ingestion_root_policies', 'ingestion_policy_decisions');
DROP TABLE IF EXISTS knowledge_ingestion_policy_decisions;
DROP TABLE IF EXISTS knowledge_ingestion_root_policies;
