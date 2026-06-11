-- Down-migration for 0243_knowledge_memory_bridge_edges.sql (MT-124).
DELETE FROM knowledge_schema_registry WHERE family_key = 'memory_bridge_decisions';
DROP TABLE IF EXISTS knowledge_memory_bridge_decisions;
