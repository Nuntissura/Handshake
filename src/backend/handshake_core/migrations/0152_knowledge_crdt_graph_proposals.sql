-- WP-KERNEL-009 MT-068 CRDTAndConcurrencyCore-068-GraphMutationProposalModel.
-- Master Spec anchor: 02-system-architecture.md section 2.3.13.11 — "graph
-- mutation proposals ... MUST leave actor, source span, state-vector,
-- validation, denial, or promotion receipts" and "A direct write to
-- ProjectKnowledgeIndex authority records outside the catalog/write-box/
-- promotion path is invalid."
--
-- knowledge_crdt_graph_proposals stores agent knowledge-graph writes as
-- reviewable DRAFT proposals (authority_class 'support', NEVER authority).
-- Promotion into EventLedger-backed facts is MT-069's bridge; rejection and
-- denial receipts are durable (0150 + EventLedger).
--
-- Span refs: JSONB array of span ref strings ('KSP-<32hex>' ids from
-- knowledge_spans, migration 0134, or 'pending:<source>:<range>' markers for
-- spans not yet extracted). Soft refs by design: a proposal may cite spans
-- that a later re-index retires; the promotion gate (MT-069) re-validates.

CREATE TABLE IF NOT EXISTS knowledge_crdt_graph_proposals (
    proposal_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL CHECK (btrim(workspace_id) <> ''),
    mutation_kind TEXT NOT NULL CHECK (mutation_kind IN (
        'add_entity', 'retire_entity',
        'add_edge', 'retire_edge',
        'add_claim', 'retire_claim'
    )),
    -- The proposed mutation payload (typed JSON; entity/edge/claim draft).
    mutation_payload JSONB NOT NULL,
    -- Evidence: at least one source span ref (spec MUST).
    source_span_refs JSONB NOT NULL,
    confidence DOUBLE PRECISION NOT NULL
        CHECK (confidence >= 0.0 AND confidence <= 1.0),
    -- Proposing actor (canonical MT-065 form) + provenance.
    actor_id TEXT NOT NULL CHECK (actor_id ~ '^(operator|local_model|cloud_model|validator|system):[A-Za-z0-9._-]+$'),
    actor_kind TEXT NOT NULL CHECK (actor_kind IN (
        'operator', 'local_model', 'cloud_model', 'validator', 'system'
    )),
    session_id TEXT NOT NULL CHECK (btrim(session_id) <> ''),
    correlation_id TEXT NOT NULL CHECK (btrim(correlation_id) <> ''),
    -- Optional lane lease under which the proposal was written (MT-076).
    lease_id TEXT REFERENCES knowledge_crdt_agent_lane_leases(lease_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    review_state TEXT NOT NULL DEFAULT 'proposed' CHECK (review_state IN (
        'proposed', 'approved', 'rejected', 'promoted'
    )),
    decided_by TEXT,
    decided_at_utc TIMESTAMPTZ,
    decision_reason TEXT,
    -- EventLedger receipts.
    recorded_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    decided_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_crdt_graph_proposals_id
        CHECK (proposal_id ~ '^[0-9A-HJKMNP-TV-Z]{26}$'),
    -- Span evidence MUST be a non-empty JSON array of non-empty strings.
    CONSTRAINT chk_knowledge_crdt_graph_proposals_spans
        CHECK (
            jsonb_typeof(source_span_refs) = 'array'
            AND jsonb_array_length(source_span_refs) >= 1
        ),
    -- Decision fields travel together with a decided/promoted state.
    CONSTRAINT chk_knowledge_crdt_graph_proposals_decision
        CHECK (
            (review_state = 'proposed'
                AND decided_by IS NULL AND decided_at_utc IS NULL
                AND decision_reason IS NULL AND decided_event_id IS NULL)
            OR (review_state IN ('approved', 'rejected', 'promoted')
                AND decided_by IS NOT NULL AND decided_at_utc IS NOT NULL
                AND decision_reason IS NOT NULL AND decided_event_id IS NOT NULL)
        ),
    -- Reviewers are operators or validators (models cannot self-approve).
    CONSTRAINT chk_knowledge_crdt_graph_proposals_reviewer
        CHECK (
            decided_by IS NULL
            OR decided_by ~ '^(operator|validator):[A-Za-z0-9._-]+$'
        )
);

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_graph_proposals_workspace_state
    ON knowledge_crdt_graph_proposals (workspace_id, review_state, created_at_utc);

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_graph_proposals_actor
    ON knowledge_crdt_graph_proposals (actor_id, created_at_utc);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('crdt_graph_proposals', 'knowledge_crdt_graph_proposals', 'GraphMutationProposal',
     'support', '0152_knowledge_crdt_graph_proposals.sql', 'MT-068')
ON CONFLICT (family_key) DO NOTHING;
