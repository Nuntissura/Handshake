-- WP-KERNEL-009 MT-053 rollback (MT-063 RollbackAndRepairMigrations).
-- Edge rows referencing entities (0136) must be rolled back first.

DROP TABLE IF EXISTS knowledge_entity_spans;
DROP TABLE IF EXISTS knowledge_entities;
DELETE FROM knowledge_schema_registry WHERE family_key IN ('entities', 'entity_spans');
