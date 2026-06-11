-- WP-KERNEL-009 MT-113 MemoryGraphAndClaims-113-MemoryOntologySchema.
-- Master Spec anchor: 02-system-architecture.md section 2.3.13.11 (Project
-- Knowledge Index and Rich Document Authority) and the WP-009 contract field
-- `translated_memory_system_spec.layers[MemoryOntology]`:
--   "Stable schema memory for entity classes, relation classes, aliases,
--    normalized terms, and promoted extraction patterns. Schema terms start
--    probationary and become stable only after evidence thresholds, conflict
--    checks, or operator/spec approval."
--
-- This is the MemoryGraph's ontology layer. It EXTENDS the committed knowledge
-- substrate (entities 0135, edges 0136, claims 0137, spans 0134, passages
-- 0138); it does not duplicate it. An ontology term names a *class* (e.g. the
-- `symbol` entity kind, the `depends_on` relation) or a *promoted extraction
-- pattern*, with a lifecycle that mirrors the claim lifecycle discipline:
-- probationary terms cannot become stable retrieval ontology without evidence,
-- conflict checks, and an EventLedger promotion receipt (MT-119/MT-120).
--
-- Migration band: 0240-0259 is the MemoryGraphAndClaims band (MT-113..MT-128).
-- The PostgresEventLedgerCore chain (0130-0142) and the CRDT band (0150-0159)
-- are frozen; this ships additively.
--
-- Identity model: an ontology term's stable natural identity is
-- (workspace_id, term_kind, term_key). Re-derivation by any later extraction
-- run upserts on that identity, so term_id stays stable. Aliases map alternate
-- surface spellings onto a canonical term.

CREATE TABLE IF NOT EXISTS knowledge_memory_ontology_terms (
    term_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- What class of ontology object this term names.
    term_kind TEXT NOT NULL CHECK (term_kind IN (
        'entity_class', 'relation_class', 'attribute', 'extraction_pattern'
    )),
    -- Stable natural key within (workspace, kind): the normalized class name,
    -- relation type, or extraction pattern signature.
    term_key TEXT NOT NULL,
    -- The canonical normalized display label.
    normalized_label TEXT NOT NULL,
    -- For relation_class terms: the edge_type this ontology relation maps onto
    -- in the knowledge_edges substrate (NULL for non-relation terms). Kept as a
    -- soft mapping (TEXT, validated by CHECK) rather than an FK because
    -- knowledge_edges has no edge_type lookup table; the edge_type vocabulary
    -- is the 0136 CHECK set.
    maps_to_edge_type TEXT
        CHECK (maps_to_edge_type IS NULL OR maps_to_edge_type IN (
            'defines', 'references', 'contains', 'depends_on', 'implements',
            'documents', 'validates', 'derived_from', 'mentions', 'links_to',
            'supersedes', 'relates_to'
        )),
    -- For entity_class terms: the entity_kind this ontology class maps onto.
    maps_to_entity_kind TEXT
        CHECK (maps_to_entity_kind IS NULL OR maps_to_entity_kind IN (
            'symbol', 'concept', 'file', 'folder', 'project', 'person', 'role',
            'task', 'api', 'schema', 'command', 'media', 'manual_entry',
            'product_primitive', 'spec_topic', 'work_packet', 'micro_task',
            'taskboard_row', 'rich_document', 'loom_block', 'user_manual_page'
        )),
    -- Lifecycle: probationary terms are not yet stable retrieval ontology.
    -- Mirrors the claim lifecycle discipline (proposed/accepted/retired) under
    -- ontology-specific names; promotion is receipt-backed (MT-120).
    lifecycle_state TEXT NOT NULL DEFAULT 'probationary'
        CHECK (lifecycle_state IN ('probationary', 'stable', 'retired')),
    retirement_reason TEXT
        CHECK (retirement_reason IS NULL OR retirement_reason IN (
            'rejected', 'superseded', 'stale', 'operator_retired'
        )),
    superseded_by_term_id TEXT
        REFERENCES knowledge_memory_ontology_terms(term_id) ON DELETE SET NULL,
    -- Promotion accounting (MT-119/MT-120): how many independent observations
    -- back this term, and the threshold it must clear to be promotable.
    observation_count INTEGER NOT NULL DEFAULT 0 CHECK (observation_count >= 0),
    promotion_threshold INTEGER NOT NULL DEFAULT 3 CHECK (promotion_threshold >= 1),
    -- Whether an operator/spec explicitly approved this term (a promotion path
    -- that bypasses the frequency threshold per the translated spec).
    operator_approved BOOLEAN NOT NULL DEFAULT FALSE,
    -- The EventLedger receipt that promoted this term to stable (MT-120). NULL
    -- while probationary; required when stable (enforced by trigger below).
    promotion_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    detection_provenance JSONB NOT NULL DEFAULT '{}'::jsonb,
    first_seen_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    last_seen_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_memory_ontology_terms_id
        CHECK (term_id ~ '^KMO-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_memory_ontology_terms_key
        CHECK (btrim(term_key) = term_key AND term_key <> ''),
    CONSTRAINT chk_knowledge_memory_ontology_terms_label
        CHECK (btrim(normalized_label) = normalized_label AND normalized_label <> ''),
    -- Retired terms must say why; live terms must not carry a reason.
    CONSTRAINT chk_knowledge_memory_ontology_terms_retirement_shape
        CHECK (
            (lifecycle_state = 'retired' AND retirement_reason IS NOT NULL)
            OR (lifecycle_state <> 'retired' AND retirement_reason IS NULL)
        ),
    -- Superseded pointer only makes sense on superseded retirement.
    CONSTRAINT chk_knowledge_memory_ontology_terms_superseded_shape
        CHECK (superseded_by_term_id IS NULL OR retirement_reason = 'superseded'),
    -- relation_class maps to an edge type; entity_class maps to an entity kind.
    -- A term must not claim both mappings.
    CONSTRAINT chk_knowledge_memory_ontology_terms_mapping_exclusive
        CHECK (NOT (maps_to_edge_type IS NOT NULL AND maps_to_entity_kind IS NOT NULL)),
    CONSTRAINT uq_knowledge_memory_ontology_terms_identity
        UNIQUE (workspace_id, term_kind, term_key)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_memory_ontology_terms_ws_kind_state
    ON knowledge_memory_ontology_terms (workspace_id, term_kind, lifecycle_state);

-- Aliases: alternate surface spellings that normalize onto a canonical term.
CREATE TABLE IF NOT EXISTS knowledge_memory_ontology_aliases (
    alias_id TEXT PRIMARY KEY,
    term_id TEXT NOT NULL
        REFERENCES knowledge_memory_ontology_terms(term_id) ON DELETE CASCADE,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- The raw alias surface form (case/spacing preserved for display).
    alias_surface TEXT NOT NULL,
    -- The case-folded/normalized key the alias resolves by.
    alias_norm_key TEXT NOT NULL,
    -- Where the alias came from: extraction, operator, spec, import.
    alias_source TEXT NOT NULL DEFAULT 'extraction'
        CHECK (alias_source IN ('extraction', 'operator', 'spec', 'import')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_memory_ontology_aliases_id
        CHECK (alias_id ~ '^KMA-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_memory_ontology_aliases_surface
        CHECK (btrim(alias_surface) = alias_surface AND alias_surface <> ''),
    CONSTRAINT chk_knowledge_memory_ontology_aliases_norm_key
        CHECK (btrim(alias_norm_key) = alias_norm_key AND alias_norm_key <> ''),
    -- An alias key is unique per workspace: one normalized spelling resolves to
    -- exactly one canonical term (prevents ambiguous alias graphs).
    CONSTRAINT uq_knowledge_memory_ontology_aliases_norm
        UNIQUE (workspace_id, alias_norm_key)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_memory_ontology_aliases_term
    ON knowledge_memory_ontology_aliases (term_id);

-- Promotion invariant: a stable term MUST carry a promotion receipt. This is
-- the ontology mirror of the claim evidence-and-receipt discipline: a term may
-- only enter stable retrieval ontology with an EventLedger promotion receipt.
CREATE OR REPLACE FUNCTION knowledge_memory_ontology_stable_requires_receipt()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.lifecycle_state = 'stable'
       AND NEW.promotion_receipt_event_id IS NULL THEN
        RAISE EXCEPTION
            'knowledge_memory_ontology_terms % violates MT-120: a stable ontology term MUST carry a promotion_receipt_event_id',
            NEW.term_id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER trg_knowledge_memory_ontology_stable_requires_receipt
    BEFORE INSERT OR UPDATE ON knowledge_memory_ontology_terms
    FOR EACH ROW EXECUTE FUNCTION knowledge_memory_ontology_stable_requires_receipt();

-- Lifecycle transition guard: probationary -> stable | retired,
-- stable -> retired, retired terminal. Mirrors the claim transition guard
-- (0200) so every writer (app method or raw SQL) obeys the same lifecycle.
CREATE OR REPLACE FUNCTION knowledge_memory_ontology_transition_guard()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.lifecycle_state = OLD.lifecycle_state THEN
        RETURN NEW;
    END IF;
    IF OLD.lifecycle_state = 'retired' THEN
        RAISE EXCEPTION
            'knowledge_memory_ontology_terms % violates MT-119: retired is terminal (attempted % -> %)',
            NEW.term_id, OLD.lifecycle_state, NEW.lifecycle_state
            USING ERRCODE = 'check_violation';
    END IF;
    IF (OLD.lifecycle_state, NEW.lifecycle_state) IN (
        ('probationary', 'stable'),
        ('probationary', 'retired'),
        ('stable',       'retired')
    ) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION
        'knowledge_memory_ontology_terms % violates MT-119: illegal lifecycle transition % -> %',
        NEW.term_id, OLD.lifecycle_state, NEW.lifecycle_state
        USING ERRCODE = 'check_violation';
END $$;

CREATE TRIGGER trg_knowledge_memory_ontology_transition_guard
    BEFORE UPDATE OF lifecycle_state ON knowledge_memory_ontology_terms
    FOR EACH ROW EXECUTE FUNCTION knowledge_memory_ontology_transition_guard();

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('memory_ontology_terms', 'knowledge_memory_ontology_terms', 'MemoryOntology',
     'authority', '0240_knowledge_memory_ontology.sql', 'MT-113'),
    ('memory_ontology_aliases', 'knowledge_memory_ontology_aliases', 'MemoryOntology',
     'authority', '0240_knowledge_memory_ontology.sql', 'MT-113')
ON CONFLICT (family_key) DO NOTHING;
