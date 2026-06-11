-- Down-migration for 0241_knowledge_memory_facts.sql (MT-114).
DELETE FROM knowledge_schema_registry WHERE family_key = 'memory_facts';
DROP TABLE IF EXISTS knowledge_memory_facts;
