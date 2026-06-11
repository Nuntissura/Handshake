-- WP-KERNEL-009 MT-057 rollback (MT-063 RollbackAndRepairMigrations).

DROP TABLE IF EXISTS knowledge_passage_evidence;
DROP TABLE IF EXISTS knowledge_memory_passages;
DROP FUNCTION IF EXISTS knowledge_passage_requires_evidence();
DELETE FROM knowledge_schema_registry
WHERE family_key IN ('memory_passages', 'passage_evidence');
