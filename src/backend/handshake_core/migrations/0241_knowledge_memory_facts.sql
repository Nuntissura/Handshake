-- WP-KERNEL-009 MT-114 MemoryGraphAndClaims-114-MemoryFactSchema.
-- Master Spec anchor: 02-system-architecture.md section 2.3.13.11 (KnowledgeClaim
-- MUST carry lifecycle state, evidence spans, conflict refs, resolution receipts)
-- and the WP-009 contract field
-- `translated_memory_system_spec.layers[MemoryFact]`:
--   "Evidence-backed factual claims and relationships with lifecycle state."
--   states: probationary, stable, conflicted, rejected, superseded,
--           temporal_qualified, granularity_qualified, unsupported.
--
-- A MemoryFact is a STRUCTURED subject/predicate/object view that is BACKED BY a
-- knowledge_claims row (1:1). It does NOT re-implement the claim lifecycle: the
-- backing claim carries the lifecycle_state, the REQUIRED evidence spans
-- (0137 trigger), the transition guard (0200), and the conflict machinery
-- (knowledge_claim_conflicts + EventLedger resolution). The fact row adds the
-- S/P/O decomposition, the predicate ontology link, and a fact-level authority
-- label (MT-125). Deleting the claim cascades the fact away; the claim is the
-- evidence-and-lifecycle authority, the fact is its structured projection-with-
-- identity.
--
-- The translated-spec fact states map onto the backing claim like so (the claim
-- lifecycle_state is canonical; the extra fact states are qualifiers carried on
-- the claim row already):
--   probationary           -> claim.lifecycle_state = 'proposed'
--   stable                 -> claim.lifecycle_state = 'accepted'
--   conflicted             -> claim.lifecycle_state = 'conflicted'
--   rejected               -> claim.lifecycle_state = 'retired' (reason rejected)
--   superseded             -> claim.lifecycle_state = 'retired' (reason superseded)
--   temporal_qualified     -> claim.temporal_qualifier IS NOT NULL
--   granularity_qualified  -> claim.granularity_qualifier IS NOT NULL
--   unsupported            -> fact.authority_label = 'unsupported' (MT-125)

CREATE TABLE IF NOT EXISTS knowledge_memory_facts (
    fact_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- 1:1 backing claim: the lifecycle + evidence + conflict authority.
    claim_id TEXT NOT NULL UNIQUE
        REFERENCES knowledge_claims(claim_id) ON DELETE CASCADE,
    -- Subject is a knowledge entity (the thing the fact is about).
    subject_entity_id TEXT NOT NULL
        REFERENCES knowledge_entities(entity_id) ON DELETE CASCADE,
    -- Predicate: the relation, optionally linked to a relation_class ontology
    -- term (MT-113). Free text under a non-empty CHECK so extraction can record
    -- a predicate before its ontology term is promoted.
    predicate_key TEXT NOT NULL,
    predicate_term_id TEXT
        REFERENCES knowledge_memory_ontology_terms(term_id) ON DELETE SET NULL,
    -- Object is EITHER another entity (relationship fact) OR a literal value
    -- (attribute fact). Exactly one must be set.
    object_entity_id TEXT
        REFERENCES knowledge_entities(entity_id) ON DELETE CASCADE,
    object_literal TEXT,
    -- Optional structured qualifiers (units, source-system, scope), distinct
    -- from the claim's temporal/granularity qualifiers.
    qualifiers JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Fact-level authority label (MT-125). The full vocabulary is enforced by
    -- CHECK; the default mirrors a freshly extracted, model-suggested fact.
    authority_label TEXT NOT NULL DEFAULT 'model_suggested'
        CHECK (authority_label IN (
            'source', 'derived', 'model_suggested', 'operator_approved',
            'deprecated', 'superseded', 'unsupported'
        )),
    extractor_version TEXT NOT NULL,
    created_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_memory_facts_id
        CHECK (fact_id ~ '^KMF-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_memory_facts_predicate
        CHECK (btrim(predicate_key) = predicate_key AND predicate_key <> ''),
    CONSTRAINT chk_knowledge_memory_facts_extractor_version
        CHECK (btrim(extractor_version) = extractor_version AND extractor_version <> ''),
    -- Exactly one object form: entity (relationship) XOR literal (attribute).
    CONSTRAINT chk_knowledge_memory_facts_object_shape
        CHECK (
            (object_entity_id IS NOT NULL AND object_literal IS NULL)
            OR (object_entity_id IS NULL AND object_literal IS NOT NULL)
        )
);

CREATE INDEX IF NOT EXISTS idx_knowledge_memory_facts_workspace
    ON knowledge_memory_facts (workspace_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_memory_facts_subject
    ON knowledge_memory_facts (subject_entity_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_memory_facts_object
    ON knowledge_memory_facts (object_entity_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_memory_facts_predicate
    ON knowledge_memory_facts (workspace_id, predicate_key);

-- Symbolic conflict key: subject + predicate identify the assertion target.
-- Two facts sharing this key with different objects are conflict candidates
-- (MT-121). Indexed for the candidate search.
CREATE INDEX IF NOT EXISTS idx_knowledge_memory_facts_symbolic_key
    ON knowledge_memory_facts (workspace_id, subject_entity_id, predicate_key);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('memory_facts', 'knowledge_memory_facts', 'MemoryFact',
     'authority', '0241_knowledge_memory_facts.sql', 'MT-114')
ON CONFLICT (family_key) DO NOTHING;
