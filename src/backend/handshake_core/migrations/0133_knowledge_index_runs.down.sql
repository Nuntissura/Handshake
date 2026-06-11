-- WP-KERNEL-009 MT-052 rollback (MT-063 RollbackAndRepairMigrations).
-- Span/entity/edge rows referencing runs (0134+) must be rolled back first.

DROP TABLE IF EXISTS knowledge_index_runs;
DELETE FROM knowledge_schema_registry WHERE family_key = 'index_runs';
