-- WP-KERNEL-009 MT-055 rollback (MT-063 RollbackAndRepairMigrations).
-- Entity/edge/claim span links (0135+) must be rolled back first.

DROP TABLE IF EXISTS knowledge_spans;
DELETE FROM knowledge_schema_registry WHERE family_key = 'spans';
