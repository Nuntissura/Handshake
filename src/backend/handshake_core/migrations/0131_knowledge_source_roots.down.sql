-- WP-KERNEL-009 MT-050 rollback (MT-063 RollbackAndRepairMigrations).
-- Dependent file/source rows (0132+) must be rolled back first; this file
-- only removes the root table and its registry row.

DROP TABLE IF EXISTS knowledge_source_roots;
DELETE FROM knowledge_schema_registry WHERE family_key = 'source_roots';
