-- WP-KERNEL-009 MT-091 hardening (#3) rollback (MT-063 RollbackAndRepairMigrations).
-- Reverse-order rollback runs this before 0163.down, so the spans table still
-- exists; drop the additive redaction-marker CHECK constraint.

ALTER TABLE knowledge_ingestion_spans
    DROP CONSTRAINT IF EXISTS chk_knowledge_ingestion_spans_redaction_marker;
