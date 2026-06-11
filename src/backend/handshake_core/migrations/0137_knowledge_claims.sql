-- WP-KERNEL-009 MT-056 PostgresEventLedgerCore-056-KnowledgeClaimTables.
-- Master Spec anchor: 2.3.13.11 KnowledgeClaim ("an assertion about a source,
-- product behavior, task, or operator workflow. Claims MUST carry lifecycle
-- state (proposed, accepted, conflicted, retired), evidence spans, conflict
-- refs, and resolution receipts.").
--
-- MT-056 contract wording maps onto the spec lifecycle as follows
-- (spec states are canonical; contract adjectives are qualifiers):
--   probationary        -> lifecycle_state = 'proposed'
--   stable              -> lifecycle_state = 'accepted'
--   rejected            -> lifecycle_state = 'retired' + retirement_reason 'rejected'
--   superseded          -> lifecycle_state = 'retired' + retirement_reason
--                          'superseded' + superseded_by_claim_id
--   temporal            -> temporal_qualifier JSONB (valid_from/valid_to)
--   granular-qualified  -> granularity_qualifier TEXT
--
-- Lifecycle transitions are guarded in
-- storage/knowledge.rs::transition_knowledge_claim:
--   proposed   -> accepted | conflicted | retired
--   accepted   -> conflicted | retired
--   conflicted -> accepted | retired
--   retired    -> (terminal)
--
-- Evidence: knowledge_claim_spans (REQUIRED, >= 1, commit-time trigger).
-- Conflicts: knowledge_claim_conflicts with resolution receipt refs into the
-- EventLedger.

CREATE TABLE IF NOT EXISTS knowledge_claims (
    claim_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    claim_kind TEXT NOT NULL CHECK (claim_kind IN (
        'source_fact', 'product_behavior', 'task_state', 'operator_workflow'
    )),
    claim_text TEXT NOT NULL,
    subject_entity_id TEXT
        REFERENCES knowledge_entities(entity_id) ON DELETE SET NULL,
    lifecycle_state TEXT NOT NULL DEFAULT 'proposed'
        CHECK (lifecycle_state IN ('proposed', 'accepted', 'conflicted', 'retired')),
    -- Temporal qualifier: {"valid_from": ..., "valid_to": ...} or NULL.
    temporal_qualifier JSONB,
    -- Granularity qualifier: e.g. 'file', 'symbol', 'workspace', free token.
    granularity_qualifier TEXT,
    confidence DOUBLE PRECISION NOT NULL DEFAULT 0.5
        CHECK (confidence >= 0.0 AND confidence <= 1.0),
    retirement_reason TEXT
        CHECK (retirement_reason IS NULL OR retirement_reason IN (
            'rejected', 'superseded', 'stale', 'operator_retired'
        )),
    superseded_by_claim_id TEXT
        REFERENCES knowledge_claims(claim_id) ON DELETE SET NULL,
    proposed_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    resolution_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_claims_id CHECK (claim_id ~ '^KCL-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_claims_text
        CHECK (btrim(claim_text) = claim_text AND claim_text <> ''),
    -- Retired claims must say why; live claims must not carry a reason.
    CONSTRAINT chk_knowledge_claims_retirement_shape
        CHECK (
            (lifecycle_state = 'retired' AND retirement_reason IS NOT NULL)
            OR (lifecycle_state <> 'retired' AND retirement_reason IS NULL)
        ),
    -- Superseded pointer only makes sense on superseded retirement.
    CONSTRAINT chk_knowledge_claims_superseded_shape
        CHECK (superseded_by_claim_id IS NULL OR retirement_reason = 'superseded')
);

CREATE INDEX IF NOT EXISTS idx_knowledge_claims_workspace_state
    ON knowledge_claims (workspace_id, lifecycle_state);

CREATE INDEX IF NOT EXISTS idx_knowledge_claims_subject
    ON knowledge_claims (subject_entity_id);

-- REQUIRED evidence spans for every claim.
CREATE TABLE IF NOT EXISTS knowledge_claim_spans (
    claim_id TEXT NOT NULL
        REFERENCES knowledge_claims(claim_id) ON DELETE CASCADE,
    span_id TEXT NOT NULL
        REFERENCES knowledge_spans(span_id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (claim_id, span_id)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_claim_spans_span
    ON knowledge_claim_spans (span_id);

CREATE OR REPLACE FUNCTION knowledge_claim_requires_span() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM knowledge_claim_spans WHERE claim_id = NEW.claim_id
    ) THEN
        RAISE EXCEPTION
            'knowledge_claims % violates spec 2.3.13.11: claims MUST carry evidence spans (knowledge_claim_spans is empty at commit)',
            NEW.claim_id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END $$;

CREATE CONSTRAINT TRIGGER trg_knowledge_claim_requires_span
    AFTER INSERT ON knowledge_claims
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION knowledge_claim_requires_span();

-- Conflict refs between claims, resolved through EventLedger receipts.
CREATE TABLE IF NOT EXISTS knowledge_claim_conflicts (
    conflict_id TEXT PRIMARY KEY,
    claim_id TEXT NOT NULL
        REFERENCES knowledge_claims(claim_id) ON DELETE CASCADE,
    conflicting_claim_id TEXT NOT NULL
        REFERENCES knowledge_claims(claim_id) ON DELETE CASCADE,
    detected_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    conflict_reason TEXT NOT NULL,
    resolution_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ,
    CONSTRAINT chk_knowledge_claim_conflicts_id
        CHECK (conflict_id ~ '^KCC-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_claim_conflicts_distinct
        CHECK (claim_id <> conflicting_claim_id),
    CONSTRAINT chk_knowledge_claim_conflicts_reason
        CHECK (btrim(conflict_reason) = conflict_reason AND conflict_reason <> ''),
    -- Resolution is receipt-backed: resolved_at requires the receipt ref.
    CONSTRAINT chk_knowledge_claim_conflicts_resolution_shape
        CHECK (resolved_at IS NULL OR resolution_receipt_event_id IS NOT NULL),
    CONSTRAINT uq_knowledge_claim_conflicts_pair
        UNIQUE (claim_id, conflicting_claim_id)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_claim_conflicts_claim
    ON knowledge_claim_conflicts (claim_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_claim_conflicts_conflicting
    ON knowledge_claim_conflicts (conflicting_claim_id);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('claims', 'knowledge_claims', 'KnowledgeClaim',
     'authority', '0137_knowledge_claims.sql', 'MT-056'),
    ('claim_spans', 'knowledge_claim_spans', 'KnowledgeClaim',
     'authority', '0137_knowledge_claims.sql', 'MT-056'),
    ('claim_conflicts', 'knowledge_claim_conflicts', 'KnowledgeClaim',
     'authority', '0137_knowledge_claims.sql', 'MT-056')
ON CONFLICT (family_key) DO NOTHING;
