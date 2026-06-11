-- WP-KERNEL-009 MT-056 rollback (MT-063 RollbackAndRepairMigrations).

DROP TABLE IF EXISTS knowledge_claim_conflicts;
DROP TABLE IF EXISTS knowledge_claim_spans;
DROP TABLE IF EXISTS knowledge_claims;
DROP FUNCTION IF EXISTS knowledge_claim_requires_span();
DELETE FROM knowledge_schema_registry
WHERE family_key IN ('claims', 'claim_spans', 'claim_conflicts');
