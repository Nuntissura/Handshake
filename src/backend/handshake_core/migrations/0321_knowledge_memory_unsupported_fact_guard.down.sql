DROP TRIGGER IF EXISTS trg_knowledge_memory_fact_authority_transition_guard
    ON knowledge_memory_facts;
DROP FUNCTION IF EXISTS knowledge_memory_fact_authority_transition_guard();

ALTER TABLE knowledge_context_bundle_items
    DROP CONSTRAINT IF EXISTS chk_knowledge_context_bundle_items_support_reason;
ALTER TABLE knowledge_context_bundle_items
    DROP COLUMN IF EXISTS unsupported_reason,
    DROP COLUMN IF EXISTS supported;
