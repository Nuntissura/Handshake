-- Down: WP-KERNEL-009 authority-hardening #1/#7 promoted-fact span evidence guard.
DROP TRIGGER IF EXISTS trg_knowledge_crdt_promoted_fact_span_evidence
    ON knowledge_crdt_promoted_facts;
DROP FUNCTION IF EXISTS knowledge_crdt_promoted_fact_span_evidence_guard();
