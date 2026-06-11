-- WP-KERNEL-009 MT-108 CodeIndexPartialFailureHandler — code-index repair queue.
-- Master Spec anchor: 2.3.13.11 + the MT-108 contract: "a file the grammar
-- cannot parse produces a typed receipt and (when repairable) a repair-queue
-- entry, never silent partial indexing."
--
-- Adversarial-review finding closed by this migration: mod.rs/engine.rs claimed
-- "repair-queue integration" for code-index parse failures, but
-- record_parse_failure had NO queue integration. The SourceIngestionAndEvidence
-- group already owns its own repair queue (knowledge_ingestion_repair_queue,
-- 0164) for ingestion/extraction failures; that queue is owned by a different
-- module the code-index lane must not touch. This is the CODE-INDEX equivalent:
-- a durable, backend-visible repair surface for files whose PARSE failed
-- (grammar init / no tree / FFI panic) or whose READ failed (binary / non-UTF8
-- / unreadable), so a no-context model can see the open code-index repair work
-- and re-run the parse pass after the cause is fixed.
--
-- Range note: the code-index group's own bookkeeping tables live at 0170-0171,
-- but this remediation migration is authored in the operator-assigned
-- 0230-0239 hardening band; it belongs to the same module
-- (storage::knowledge code-index section) and the same MT family (MT-108).
--
-- Lifecycle mirrors knowledge_ingestion_repair_queue (0164):
--   queued -> retrying -> resolved        (re-parse succeeded)
--   queued -> retrying -> queued          (re-parse failed, attempts < max)
--   queued -> retrying -> dead_letter     (attempts exhausted)
--   queued|retrying -> resolved|dead_letter (operator decision)
-- Terminal states (resolved, dead_letter) never transition again.

CREATE TABLE IF NOT EXISTS knowledge_code_repair_queue (
    code_repair_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- The shared KnowledgeSource whose code-index parse/read failed.
    source_id TEXT NOT NULL
        REFERENCES knowledge_sources(source_id) ON DELETE CASCADE,
    -- Repo-relative path (denormalised for operator-visible queues).
    relative_path TEXT NOT NULL,
    -- Why the file sits in the code-index repair queue.
    reason_class TEXT NOT NULL CHECK (reason_class IN (
        'PARSE_ERROR', 'READ_ERROR', 'PANIC', 'CONFIG_PARSE_ERROR'
    )),
    -- Operator-visible reason payload (the failing reason string, language...).
    reason_detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    state TEXT NOT NULL DEFAULT 'queued' CHECK (state IN (
        'queued', 'retrying', 'resolved', 'dead_letter'
    )),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    max_attempts INTEGER NOT NULL DEFAULT 3 CHECK (max_attempts >= 1),
    last_attempt_at TIMESTAMPTZ,
    -- The code-index file-parse receipt that enqueued this entry.
    enqueue_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    -- The receipt of the attempt that resolved the entry.
    resolved_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_code_repair_queue_id
        CHECK (code_repair_id ~ '^KCRQ-[0-9a-f]{32}$'),
    -- Resolved entries must name the receipt that resolved them.
    CONSTRAINT chk_knowledge_code_repair_queue_resolved_shape
        CHECK (state <> 'resolved' OR resolved_receipt_event_id IS NOT NULL)
);

-- One OPEN repair entry per source: re-failing while queued updates the
-- existing entry instead of multiplying rows.
CREATE UNIQUE INDEX IF NOT EXISTS uq_knowledge_code_repair_open_source
    ON knowledge_code_repair_queue (source_id)
    WHERE state IN ('queued', 'retrying');

CREATE INDEX IF NOT EXISTS idx_knowledge_code_repair_workspace_state
    ON knowledge_code_repair_queue (workspace_id, state, created_at DESC);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('code_repair_queue', 'knowledge_code_repair_queue', 'KnowledgeSource',
     'support', '0230_knowledge_code_repair_queue.sql', 'MT-108')
ON CONFLICT (family_key) DO NOTHING;
