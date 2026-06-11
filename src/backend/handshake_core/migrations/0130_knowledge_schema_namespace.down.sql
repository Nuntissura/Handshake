-- WP-KERNEL-009 MT-049 rollback (MT-063 RollbackAndRepairMigrations).
-- Safe rollback rule: the registry is dropped LAST among WP-009 down files
-- (it is the namespace boundary record; later family down-files only remove
-- their own rows/tables). Dropping the registry removes the WP-009 namespace
-- marker entirely.

DROP TABLE IF EXISTS knowledge_schema_registry;
