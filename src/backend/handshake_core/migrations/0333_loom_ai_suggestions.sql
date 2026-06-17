-- WP-KERNEL-009 MT-260 UnifiedWorkSurface-260-AILoomJobs (GAP-LM-011).
-- Master Spec anchor: 02-system-architecture.md section 2.3.13.11 — "AI edit
-- proposals, graph mutation proposals, relationship extraction, auto-linking,
-- auto-tagging ... MUST leave actor, source span, state-vector, validation,
-- denial, or promotion receipts."
--
-- AI Loom jobs (auto-tag / auto-caption / batch link-suggestion) run the
-- operator's configured model over LoomBlocks and land EVERY model suggestion
-- as a PENDING proposal row in loom_ai_suggestions. Nothing becomes authority
-- (a real LoomEdge / derived auto_caption|auto_tags field) until an operator or
-- validator confirms (accept => decide+promote). The row carries full model
-- attribution (model, version, token usage, trace_id), prompt/output hashes for
-- provenance, the EventLedger event ids for record/decide/promotion, and the
-- promoted artifact ref. Authority is PostgreSQL + EventLedger; UI is a
-- projection. There is NO parallel store (no SQLite/in-mem).

CREATE TABLE IF NOT EXISTS loom_ai_suggestions (
    suggestion_id TEXT PRIMARY KEY
        CHECK (suggestion_id ~ '^LAIS-[0-9a-f]{32}$'),
    job_id TEXT NOT NULL CHECK (job_id ~ '^LAIJ-[0-9a-f]{32}$'),
    workspace_id TEXT NOT NULL
        REFERENCES workspaces(id) ON UPDATE RESTRICT ON DELETE CASCADE,
    -- auto_tag | auto_caption | link_suggest
    kind TEXT NOT NULL CHECK (kind IN ('auto_tag', 'auto_caption', 'link_suggest')),
    -- The block the suggestion is about (the SOURCE block for link/tag edges,
    -- the captioned block for captions).
    block_id TEXT NOT NULL,
    -- For link_suggest: the suggested TARGET block. NULL for tag/caption.
    target_block_id TEXT,
    -- The model's suggested value as typed JSON:
    --   auto_tag      -> {"tag": "<tag name>"}
    --   auto_caption  -> {"caption": "<text>"}
    --   link_suggest  -> {"reason": "<why>"}
    suggested_value JSONB NOT NULL,
    -- {model, version, prompt_tokens, completion_tokens, total_tokens, trace_id}
    model_attribution JSONB NOT NULL,
    prompt_sha256 TEXT NOT NULL CHECK (prompt_sha256 ~ '^[0-9a-f]{64}$'),
    output_sha256 TEXT NOT NULL CHECK (output_sha256 ~ '^[0-9a-f]{64}$'),
    -- pending -> accepted -> promoted ; pending -> rejected
    review_state TEXT NOT NULL DEFAULT 'pending' CHECK (review_state IN (
        'pending', 'accepted', 'rejected', 'promoted'
    )),
    -- Reviewers are operators or validators; models cannot self-confirm.
    decided_by TEXT
        CHECK (decided_by IS NULL OR decided_by ~ '^(operator|validator):[A-Za-z0-9._-]+$'),
    decided_at_utc TIMESTAMPTZ,
    decision_reason TEXT,
    -- EventLedger provenance: the recorded (AI_EDIT_PROPOSAL_RECORDED) event,
    -- the decision (AI_EDIT_PROPOSAL_DECIDED) event, and the promotion pair
    -- (PROMOTION_REQUESTED + PROMOTION_ACCEPTED).
    recorded_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    decided_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    promotion_requested_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    promotion_accepted_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    -- The authority artifact the accepted suggestion became (edge_id for
    -- link/tag, block_id for caption). NULL until promoted.
    promoted_artifact_ref TEXT,
    -- Idempotency hash over (kind, block, target, value) so re-running a job
    -- never duplicates an existing suggestion.
    value_hash TEXT NOT NULL CHECK (value_hash ~ '^[0-9a-f]{64}$'),
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_loom_ai_suggestions_decision
        CHECK (
            (review_state = 'pending'
                AND decided_by IS NULL AND decided_at_utc IS NULL
                AND decision_reason IS NULL AND decided_event_id IS NULL)
            OR (review_state IN ('accepted', 'rejected', 'promoted')
                AND decided_by IS NOT NULL AND decided_at_utc IS NOT NULL
                AND decision_reason IS NOT NULL AND decided_event_id IS NOT NULL)
        ),
    CONSTRAINT chk_loom_ai_suggestions_promotion
        CHECK (
            (review_state = 'promoted'
                AND promotion_requested_event_id IS NOT NULL
                AND promotion_accepted_event_id IS NOT NULL
                AND promoted_artifact_ref IS NOT NULL)
            OR (review_state <> 'promoted'
                AND promotion_requested_event_id IS NULL
                AND promotion_accepted_event_id IS NULL
                AND promoted_artifact_ref IS NULL)
        ),
    -- link_suggest must carry a target; tag/caption must not.
    CONSTRAINT chk_loom_ai_suggestions_target
        CHECK (
            (kind = 'link_suggest' AND target_block_id IS NOT NULL)
            OR (kind IN ('auto_tag', 'auto_caption') AND target_block_id IS NULL)
        )
);

-- Idempotency: one suggestion per (job, block, target, kind, value).
CREATE UNIQUE INDEX IF NOT EXISTS uq_loom_ai_suggestions_idempotency
    ON loom_ai_suggestions (job_id, block_id, kind, value_hash, COALESCE(target_block_id, ''));

CREATE INDEX IF NOT EXISTS idx_loom_ai_suggestions_job
    ON loom_ai_suggestions (job_id, kind, review_state, created_at_utc);

CREATE INDEX IF NOT EXISTS idx_loom_ai_suggestions_workspace
    ON loom_ai_suggestions (workspace_id, review_state, created_at_utc);

-- Widen the knowledge-CRDT denial-receipt allow-list to admit the AI Loom
-- promotion denial kind (additive superset; mirrors the 0290 pattern). A
-- promote attempt on a pending/rejected suggestion, or by a non-operator,
-- leaves a durable 'loom_ai_promotion_denied' receipt.
ALTER TABLE knowledge_crdt_denial_receipts
    DROP CONSTRAINT IF EXISTS knowledge_crdt_denial_receipts_receipt_kind_check;

ALTER TABLE knowledge_crdt_denial_receipts
    ADD CONSTRAINT knowledge_crdt_denial_receipts_receipt_kind_check
    CHECK (receipt_kind IN (
        'stale_draft_save',
        'concurrent_draft_fork',
        'ahead_of_head_save',
        'update_content_mismatch',
        'sequence_slot_race',
        'lease_write_denied',
        'index_run_slot_rejected',
        'graph_promotion_denied',
        'ai_edit_promotion_denied',
        'ai_edit_applied_mismatch',
        'ai_edit_applied_update_missing',
        'loom_ai_promotion_denied'
    ));

-- NOTE: knowledge_schema_registry is the WP-009 *knowledge namespace* boundary
-- (`knowledge_`-prefixed tables only, per chk_knowledge_schema_registry_prefix).
-- loom_ai_suggestions belongs to the Loom domain (sibling of `loom_blocks`,
-- `loom_edges`, `assets`, `loom_asset_tiers`, which are likewise NOT registered
-- there), so it is intentionally not inserted into knowledge_schema_registry.
-- Authority of these rows is still PostgreSQL + EventLedger; the registry is a
-- namespace boundary record, not the authority surface.
