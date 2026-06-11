-- WP-KERNEL-009 MT-052 PostgresEventLedgerCore-052-IndexRunLifecycleTables.
-- Master Spec anchor: 2.3.13.11 ("last-index receipt", index runs feed
-- KnowledgeSource/Span/Entity/Edge provenance) + first-slice proof item
-- "source registration ... EventLedger replay".
--
-- knowledge_index_runs persists every indexing run as durable lifecycle
-- state: started/completed/failed/cancelled, the actor that ran it, the
-- worktree it scanned, a restart checkpoint for resumable runs, result
-- counts, typed error capture, and EventLedger receipt refs for start and
-- finish. Lifecycle transitions are guarded in
-- storage/knowledge.rs::transition_knowledge_index_run (terminal states are
-- terminal; only 'started' rows can move).

CREATE TABLE IF NOT EXISTS knowledge_index_runs (
    index_run_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    root_id TEXT REFERENCES knowledge_source_roots(root_id) ON DELETE SET NULL,
    run_state TEXT NOT NULL DEFAULT 'started'
        CHECK (run_state IN ('started', 'completed', 'failed', 'cancelled')),
    -- Scope description: {"mode": "full"|"incremental", "globs": [...], ...}.
    scope JSONB NOT NULL DEFAULT '{}'::jsonb,
    actor_kind TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    -- Worktree the run scanned (portable token, not a machine path).
    worktree_id TEXT,
    -- Resumable runs persist their cursor here; NULL once finished.
    restart_checkpoint JSONB,
    sources_seen INTEGER NOT NULL DEFAULT 0 CHECK (sources_seen >= 0),
    sources_indexed INTEGER NOT NULL DEFAULT 0 CHECK (sources_indexed >= 0),
    spans_extracted INTEGER NOT NULL DEFAULT 0 CHECK (spans_extracted >= 0),
    entities_detected INTEGER NOT NULL DEFAULT 0 CHECK (entities_detected >= 0),
    edges_written INTEGER NOT NULL DEFAULT 0 CHECK (edges_written >= 0),
    claims_written INTEGER NOT NULL DEFAULT 0 CHECK (claims_written >= 0),
    -- Typed error capture for failed runs: {"taxonomy": ..., "message": ...}.
    error_capture JSONB,
    start_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    finish_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMPTZ,
    CONSTRAINT chk_knowledge_index_runs_id
        CHECK (index_run_id ~ '^KIR-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_index_runs_actor
        CHECK (btrim(actor_kind) = actor_kind AND actor_kind <> ''
               AND btrim(actor_id) = actor_id AND actor_id <> ''),
    -- Terminal rows must carry a finish time; running rows must not.
    CONSTRAINT chk_knowledge_index_runs_finished_shape
        CHECK (
            (run_state = 'started' AND finished_at IS NULL)
            OR (run_state <> 'started' AND finished_at IS NOT NULL)
        ),
    -- Failed runs must capture the error.
    CONSTRAINT chk_knowledge_index_runs_error_shape
        CHECK (run_state <> 'failed' OR error_capture IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_index_runs_workspace
    ON knowledge_index_runs (workspace_id, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_knowledge_index_runs_state
    ON knowledge_index_runs (run_state);

CREATE INDEX IF NOT EXISTS idx_knowledge_index_runs_root
    ON knowledge_index_runs (root_id);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('index_runs', 'knowledge_index_runs', 'Support',
     'authority', '0133_knowledge_index_runs.sql', 'MT-052')
ON CONFLICT (family_key) DO NOTHING;
