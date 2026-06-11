-- WP-KERNEL-009 MT-069 CRDTAndConcurrencyCore-069-ClaimPromotionBridge.
-- Master Spec anchor: 02-system-architecture.md section 2.3.13.11 —
-- "authority changes MUST flow through WriteBoxV1 and EventLedger promotion".
--
-- knowledge_crdt_promoted_facts is the EventLedger-backed authority landing
-- zone for ACCEPTED graph/claim proposals (MT-068): one row per promoted
-- proposal, written ONLY by the MT-069 promotion bridge after the
-- PROMOTION_REQUESTED/PROMOTION_ACCEPTED event pair. Idempotent: the unique
-- proposal_id makes re-promotion return the existing fact.
--
-- Relationship to knowledge_claims (migration 0137, committed by MT-056):
-- promotion lands here, in the WP-009 CRDT namespace, instead of
-- double-writing knowledge_claims rows. knowledge_claims requires commit-time
-- KSP-* span evidence (FK + trigger); a DRAFT proposal (0152) may still cite
-- 'pending:<source>:<range>' markers for spans not extracted yet. Authority-
-- hardening #1 (2026-06-11): such soft markers DO NOT reach this authority
-- table. The MT-069 promotion gate resolves+validates every cited ref and
-- freezes ONLY the validated, de-duplicated canonical KSP- ids into
-- source_span_refs (a 'pending:' marker, a missing/foreign/retired span ->
-- promotion DENIED). Migration 0190 adds a BEFORE INSERT trigger here that
-- re-checks each KSP- ref exists, is same-workspace, and is not stale, so a
-- direct INSERT cannot create a fact with dangling evidence either. The frozen
-- KSP- ids let the knowledge-claims lane map promoted add_claim facts into
-- knowledge_claims without data loss.

CREATE TABLE IF NOT EXISTS knowledge_crdt_promoted_facts (
    fact_id TEXT PRIMARY KEY,
    proposal_id TEXT NOT NULL UNIQUE
        REFERENCES knowledge_crdt_graph_proposals(proposal_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    workspace_id TEXT NOT NULL CHECK (btrim(workspace_id) <> ''),
    mutation_kind TEXT NOT NULL CHECK (mutation_kind IN (
        'add_entity', 'retire_entity',
        'add_edge', 'retire_edge',
        'add_claim', 'retire_claim'
    )),
    -- Frozen copy of the approved mutation payload at promotion time.
    fact_payload JSONB NOT NULL,
    source_span_refs JSONB NOT NULL CHECK (
        jsonb_typeof(source_span_refs) = 'array'
        AND jsonb_array_length(source_span_refs) >= 1
    ),
    confidence DOUBLE PRECISION NOT NULL
        CHECK (confidence >= 0.0 AND confidence <= 1.0),
    -- Original proposer and the promotion gate actor.
    proposed_by TEXT NOT NULL,
    promoted_by TEXT NOT NULL CHECK (promoted_by ~ '^(operator|validator|system):[A-Za-z0-9._-]+$'),
    -- EventLedger promotion receipts (request + acceptance).
    promotion_requested_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    promotion_accepted_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    promoted_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_crdt_promoted_facts_id
        CHECK (fact_id ~ '^[0-9A-HJKMNP-TV-Z]{26}$')
);

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_promoted_facts_workspace
    ON knowledge_crdt_promoted_facts (workspace_id, promoted_at_utc);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('crdt_promoted_facts', 'knowledge_crdt_promoted_facts', 'KnowledgeClaim',
     'authority', '0153_knowledge_crdt_promoted_facts.sql', 'MT-069')
ON CONFLICT (family_key) DO NOTHING;
