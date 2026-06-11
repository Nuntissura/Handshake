-- WP-KERNEL-009 MT-079 rollback. Removes recovery receipts before their
-- checkpoints (FK order) and both registry rows.

DROP TABLE IF EXISTS knowledge_crdt_recovery_receipts;
DROP TABLE IF EXISTS knowledge_crdt_swarm_checkpoints;
DELETE FROM knowledge_schema_registry WHERE family_key IN ('crdt_swarm_checkpoints', 'crdt_recovery_receipts');
