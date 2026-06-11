-- WP-KERNEL-009 MT-051 rollback (MT-063 RollbackAndRepairMigrations).
-- Span/entity/edge/claim rows referencing sources (0134+) must be rolled
-- back first.

DROP TABLE IF EXISTS knowledge_sources;
DELETE FROM knowledge_schema_registry WHERE family_key = 'sources';
