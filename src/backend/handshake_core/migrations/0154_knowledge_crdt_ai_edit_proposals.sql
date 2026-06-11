-- WP-KERNEL-009 MT-074 CRDTAndConcurrencyCore-074-AiEditProposalReviewFlow.
-- Master Spec anchor: 02-system-architecture.md section 2.3.13.11 — "AI edit
-- proposals ... MUST leave actor, source span, state-vector, validation,
-- denial, or promotion receipts."
--
-- knowledge_crdt_ai_edit_proposals stores model-proposed rich-document edits
-- as reviewable drafts: the proposed diff against a pinned document
-- revision, full actor/session provenance, and source span citations.
-- Review state machine: proposed -> approved | rejected; approved ->
-- promoted (EventLedger promotion pair). Rejection leaves the decision on
-- the row AND a durable denial path when promotion is attempted anyway.

CREATE TABLE IF NOT EXISTS knowledge_crdt_ai_edit_proposals (
    proposal_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL CHECK (btrim(workspace_id) <> ''),
    document_id TEXT NOT NULL CHECK (btrim(document_id) <> ''),
    crdt_document_id TEXT NOT NULL CHECK (btrim(crdt_document_id) <> ''),
    -- Pinned base revision the diff applies to.
    base_update_seq BIGINT NOT NULL CHECK (base_update_seq >= 0),
    base_state_vector TEXT NOT NULL CHECK (base_state_vector LIKE 'hsk-sv1:%'),
    -- The proposed edit as a typed JSON diff payload (ProseMirror steps or
    -- replacement node JSON; schema stamped inside the payload).
    proposed_diff JSONB NOT NULL,
    diff_sha256 TEXT NOT NULL CHECK (diff_sha256 ~ '^[0-9a-f]{64}$'),
    -- Source span citations backing the proposal (spec MUST; >= 1).
    source_span_citations JSONB NOT NULL CHECK (
        jsonb_typeof(source_span_citations) = 'array'
        AND jsonb_array_length(source_span_citations) >= 1
    ),
    -- Proposing MODEL actor (AI proposals come from model lanes only).
    actor_id TEXT NOT NULL CHECK (actor_id ~ '^(local_model|cloud_model):[A-Za-z0-9._-]+$'),
    actor_kind TEXT NOT NULL CHECK (actor_kind IN ('local_model', 'cloud_model')),
    session_id TEXT NOT NULL CHECK (btrim(session_id) <> ''),
    correlation_id TEXT NOT NULL CHECK (btrim(correlation_id) <> ''),
    lease_id TEXT REFERENCES knowledge_crdt_agent_lane_leases(lease_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    review_state TEXT NOT NULL DEFAULT 'proposed' CHECK (review_state IN (
        'proposed', 'approved', 'rejected', 'promoted'
    )),
    -- Reviewers are operators or validators; models cannot self-approve.
    decided_by TEXT,
    decided_at_utc TIMESTAMPTZ,
    decision_reason TEXT,
    recorded_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    decided_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    promotion_requested_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    promotion_accepted_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_crdt_ai_edit_proposals_id
        CHECK (proposal_id ~ '^[0-9A-HJKMNP-TV-Z]{26}$'),
    CONSTRAINT chk_knowledge_crdt_ai_edit_proposals_decision
        CHECK (
            (review_state = 'proposed'
                AND decided_by IS NULL AND decided_at_utc IS NULL
                AND decision_reason IS NULL AND decided_event_id IS NULL)
            OR (review_state IN ('approved', 'rejected', 'promoted')
                AND decided_by IS NOT NULL AND decided_at_utc IS NOT NULL
                AND decision_reason IS NOT NULL AND decided_event_id IS NOT NULL)
        ),
    CONSTRAINT chk_knowledge_crdt_ai_edit_proposals_reviewer
        CHECK (
            decided_by IS NULL
            OR decided_by ~ '^(operator|validator):[A-Za-z0-9._-]+$'
        ),
    -- Promotion receipts only on promoted rows, and always as a pair.
    CONSTRAINT chk_knowledge_crdt_ai_edit_proposals_promotion
        CHECK (
            (review_state = 'promoted'
                AND promotion_requested_event_id IS NOT NULL
                AND promotion_accepted_event_id IS NOT NULL)
            OR (review_state <> 'promoted'
                AND promotion_requested_event_id IS NULL
                AND promotion_accepted_event_id IS NULL)
        )
);

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_ai_edit_proposals_document
    ON knowledge_crdt_ai_edit_proposals (crdt_document_id, review_state, created_at_utc);

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_ai_edit_proposals_actor
    ON knowledge_crdt_ai_edit_proposals (actor_id, created_at_utc);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('crdt_ai_edit_proposals', 'knowledge_crdt_ai_edit_proposals', 'AiEditProposal',
     'support', '0154_knowledge_crdt_ai_edit_proposals.sql', 'MT-074')
ON CONFLICT (family_key) DO NOTHING;
