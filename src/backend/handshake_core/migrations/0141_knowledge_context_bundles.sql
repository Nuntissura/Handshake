-- WP-KERNEL-009 MT-060 PostgresEventLedgerCore-060-ContextBundleTables.
-- Master Spec anchor: 2.3.13.11 RetrievalTrace ("a replayable record of
-- none/direct_load/exact_lookup/graph_traversal/hybrid_rag selection,
-- including why broader retrieval was used or skipped") + the projection rule
-- ("context bundles ... are projections only" — the BUNDLE CONTENT is a
-- projection; these tables are the durable RUN/DECISION evidence, which is
-- authority).
--
-- Kernel V1 compatibility: kernel/context_bundle.rs (ContextBundle V1) keys
-- bundles as 'CTX-' || first 16 hex of the canonical-JSON sha256, with
-- kernel_task_run_id/session_run_id/allowed_context/context_hash. These
-- tables persist exactly that shape (bundle_id, kernel_task_run_id,
-- session_run_id, allowed_context, context_hash) plus the WP-009 retrieval
-- evidence: per-item decisions, token budgets, citations, and the
-- retrieval trace with mode + mode_reason.

CREATE TABLE IF NOT EXISTS knowledge_context_bundles (
    bundle_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    kernel_task_run_id TEXT NOT NULL,
    session_run_id TEXT NOT NULL,
    -- ContextBundle V1 allowed_context payload (projection content).
    allowed_context JSONB NOT NULL,
    -- sha256 over canonical JSON of allowed_context (ContextBundle V1).
    context_hash TEXT NOT NULL,
    query_text TEXT,
    token_budget INTEGER CHECK (token_budget IS NULL OR token_budget >= 0),
    tokens_used INTEGER CHECK (tokens_used IS NULL OR tokens_used >= 0),
    build_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- ContextBundle V1 id shape: CTX- + 16 hex (prefix of context_hash).
    CONSTRAINT chk_knowledge_context_bundles_id
        CHECK (bundle_id ~ '^CTX-[0-9a-f]{16}$'),
    CONSTRAINT chk_knowledge_context_bundles_hash
        CHECK (context_hash ~ '^[0-9a-f]{64}$'),
    -- The V1 invariant: bundle_id is derived from the content hash.
    CONSTRAINT chk_knowledge_context_bundles_id_matches_hash
        CHECK (bundle_id = 'CTX-' || substring(context_hash from 1 for 16)),
    CONSTRAINT chk_knowledge_context_bundles_run_ids
        CHECK (btrim(kernel_task_run_id) = kernel_task_run_id
               AND kernel_task_run_id <> ''
               AND btrim(session_run_id) = session_run_id
               AND session_run_id <> '')
);

CREATE INDEX IF NOT EXISTS idx_knowledge_context_bundles_workspace
    ON knowledge_context_bundles (workspace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_knowledge_context_bundles_session
    ON knowledge_context_bundles (session_run_id);

-- Per-item retrieval decisions inside a bundle build.
CREATE TABLE IF NOT EXISTS knowledge_context_bundle_items (
    bundle_id TEXT NOT NULL
        REFERENCES knowledge_context_bundles(bundle_id) ON DELETE CASCADE,
    item_ordinal INTEGER NOT NULL CHECK (item_ordinal >= 0),
    ref_kind TEXT NOT NULL CHECK (ref_kind IN (
        'source', 'span', 'claim', 'passage', 'entity'
    )),
    ref_id TEXT NOT NULL,
    retrieval_decision TEXT NOT NULL CHECK (retrieval_decision IN (
        'included', 'excluded_budget', 'excluded_relevance', 'excluded_redacted'
    )),
    relevance_score DOUBLE PRECISION
        CHECK (relevance_score IS NULL
               OR (relevance_score >= 0.0 AND relevance_score <= 1.0)),
    token_count INTEGER CHECK (token_count IS NULL OR token_count >= 0),
    -- Human/model-citeable citation string for included items.
    citation TEXT,
    PRIMARY KEY (bundle_id, item_ordinal),
    CONSTRAINT chk_knowledge_context_bundle_items_ref
        CHECK (btrim(ref_id) = ref_id AND ref_id <> '')
);

CREATE INDEX IF NOT EXISTS idx_knowledge_context_bundle_items_ref
    ON knowledge_context_bundle_items (ref_kind, ref_id);

-- Replayable retrieval traces (spec RetrievalTrace family).
CREATE TABLE IF NOT EXISTS knowledge_retrieval_traces (
    trace_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    retrieval_mode TEXT NOT NULL CHECK (retrieval_mode IN (
        'none', 'direct_load', 'exact_lookup', 'graph_traversal', 'hybrid_rag'
    )),
    -- Spec MUST: why broader retrieval was used or skipped.
    mode_reason TEXT NOT NULL,
    query_text TEXT,
    -- The bundle this trace produced, when one was built.
    bundle_id TEXT
        REFERENCES knowledge_context_bundles(bundle_id) ON DELETE SET NULL,
    -- Replayable decision log: [{"step": ..., "candidates": ..., ...}].
    decisions JSONB NOT NULL DEFAULT '[]'::jsonb,
    trace_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_retrieval_traces_id
        CHECK (trace_id ~ '^KRT-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_retrieval_traces_reason
        CHECK (btrim(mode_reason) = mode_reason AND mode_reason <> '')
);

CREATE INDEX IF NOT EXISTS idx_knowledge_retrieval_traces_workspace
    ON knowledge_retrieval_traces (workspace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_knowledge_retrieval_traces_bundle
    ON knowledge_retrieval_traces (bundle_id);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('context_bundles', 'knowledge_context_bundles', 'Support',
     'authority', '0141_knowledge_context_bundles.sql', 'MT-060'),
    ('context_bundle_items', 'knowledge_context_bundle_items', 'Support',
     'authority', '0141_knowledge_context_bundles.sql', 'MT-060'),
    ('retrieval_traces', 'knowledge_retrieval_traces', 'RetrievalTrace',
     'authority', '0141_knowledge_context_bundles.sql', 'MT-060')
ON CONFLICT (family_key) DO NOTHING;
