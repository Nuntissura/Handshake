-- WP-KERNEL-009 MT-060 rollback (MT-063 RollbackAndRepairMigrations).

DROP TABLE IF EXISTS knowledge_retrieval_traces;
DROP TABLE IF EXISTS knowledge_context_bundle_items;
DROP TABLE IF EXISTS knowledge_context_bundles;
DELETE FROM knowledge_schema_registry
WHERE family_key IN ('context_bundles', 'context_bundle_items', 'retrieval_traces');
