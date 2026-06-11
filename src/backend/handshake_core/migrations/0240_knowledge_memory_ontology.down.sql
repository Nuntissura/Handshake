-- Down-migration for 0240_knowledge_memory_ontology.sql (MT-113).
DELETE FROM knowledge_schema_registry
    WHERE family_key IN ('memory_ontology_terms', 'memory_ontology_aliases');

DROP TRIGGER IF EXISTS trg_knowledge_memory_ontology_transition_guard
    ON knowledge_memory_ontology_terms;
DROP TRIGGER IF EXISTS trg_knowledge_memory_ontology_stable_requires_receipt
    ON knowledge_memory_ontology_terms;
DROP FUNCTION IF EXISTS knowledge_memory_ontology_transition_guard();
DROP FUNCTION IF EXISTS knowledge_memory_ontology_stable_requires_receipt();

DROP TABLE IF EXISTS knowledge_memory_ontology_aliases;
DROP TABLE IF EXISTS knowledge_memory_ontology_terms;
