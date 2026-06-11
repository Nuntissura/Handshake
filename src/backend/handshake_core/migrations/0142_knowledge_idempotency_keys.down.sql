-- WP-KERNEL-009 MT-062 rollback (MT-063 RollbackAndRepairMigrations).

DROP TABLE IF EXISTS knowledge_idempotency_keys;
DELETE FROM knowledge_schema_registry WHERE family_key = 'idempotency_keys';
