-- WP-KERNEL-009 MT-056 hardening rollback (MT-063 RollbackAndRepairMigrations).
-- In the reverse-order rollback chain this runs before 0137.down, so
-- knowledge_claims still exists; drop the trigger explicitly, then the routine.
-- Dropping the FUNCTION is what keeps information_schema.routines clean after
-- rollback (the leftover the MT-063 routines assertion guards against).

DROP TRIGGER IF EXISTS trg_knowledge_claim_transition_guard ON knowledge_claims;
DROP FUNCTION IF EXISTS knowledge_claim_transition_guard();
