-- WP-KERNEL-009 MT-054 rollback (MT-063 RollbackAndRepairMigrations).
-- Triggers and functions are dropped with their tables.

DROP TABLE IF EXISTS knowledge_edge_spans;
DROP TABLE IF EXISTS knowledge_edges;
DROP FUNCTION IF EXISTS knowledge_edge_requires_span();
DROP FUNCTION IF EXISTS knowledge_edge_span_delete_guard();
DELETE FROM knowledge_schema_registry WHERE family_key IN ('edges', 'edge_spans');
