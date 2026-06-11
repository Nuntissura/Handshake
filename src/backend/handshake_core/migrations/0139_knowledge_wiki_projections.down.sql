-- WP-KERNEL-009 MT-058 rollback (MT-063 RollbackAndRepairMigrations).
-- Projections are regenerable by definition; dropping the table loses no
-- authority state.

DROP TABLE IF EXISTS knowledge_wiki_projections;
DELETE FROM knowledge_schema_registry WHERE family_key = 'wiki_projections';
