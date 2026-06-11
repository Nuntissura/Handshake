-- WP-KERNEL-009 MT-094 SourceIngestionAndEvidence-094-SourceRepairQueue.
-- Master Spec anchor: 2.3.13.11 + WP constraint "PDF/media extraction
-- failures must produce receipts and repairable metadata rather than
-- silently indexing partial content"; MT-094 contract: "backend-visible
-- repair queue for failed extraction, missing assets, denied paths, and
-- stale hashes".
--
-- knowledge_ingestion_repair_queue is the durable repair surface: one OPEN
-- entry per source at a time (partial unique index), with a typed reason
-- class (mirrors the receipt error vocabulary plus queue-specific reasons),
-- operator-visible reason detail, a retry budget, and a terminal
-- dead-letter state. Lifecycle:
--
--   queued -> retrying -> resolved            (retry succeeded)
--   queued -> retrying -> queued              (retry failed, attempts < max)
--   queued -> retrying -> dead_letter         (attempts exhausted)
--   queued|retrying -> resolved|dead_letter   (operator decision)
--
-- Terminal states (resolved, dead_letter) never transition again; the
-- storeguards transitions with optimistic WHERE clauses exactly like the
-- knowledge_index_runs lifecycle.

CREATE TABLE IF NOT EXISTS knowledge_ingestion_repair_queue (
    repair_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    source_id TEXT NOT NULL REFERENCES knowledge_sources(source_id) ON DELETE CASCADE,
    -- The failing receipt that enqueued this entry.
    receipt_id TEXT REFERENCES knowledge_ingestion_receipts(receipt_id) ON DELETE SET NULL,
    reason_class TEXT NOT NULL CHECK (reason_class IN (
        'UNSUPPORTED_FORMAT', 'NO_TEXT_LAYER', 'OCR_NEEDED', 'ENCRYPTED',
        'MALFORMED_CUE', 'PARSE_ERROR', 'OVERSIZE', 'SECRET_BLOCKED',
        'IO_ERROR', 'INTERNAL', 'MISSING_ASSET', 'DENIED_PATH', 'STALE_HASH'
    )),
    -- Operator-visible reason payload (limits hit, failed pages, cues lost...).
    reason_detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    state TEXT NOT NULL DEFAULT 'queued' CHECK (state IN (
        'queued', 'retrying', 'resolved', 'dead_letter'
    )),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    max_attempts INTEGER NOT NULL DEFAULT 3 CHECK (max_attempts >= 1),
    last_attempt_at TIMESTAMPTZ,
    -- The receipt of the attempt that resolved the entry.
    resolved_receipt_id TEXT REFERENCES knowledge_ingestion_receipts(receipt_id) ON DELETE SET NULL,
    enqueue_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_ingestion_repair_queue_id
        CHECK (repair_id ~ '^KIRQ-[0-9a-f]{32}$'),
    -- Resolved entries must name the receipt that resolved them.
    CONSTRAINT chk_knowledge_ingestion_repair_queue_resolved_shape
        CHECK (state <> 'resolved' OR resolved_receipt_id IS NOT NULL)
);

-- One OPEN repair entry per source: re-failing while queued updates the
-- existing entry instead of multiplying rows.
CREATE UNIQUE INDEX IF NOT EXISTS uq_knowledge_ingestion_repair_open_source
    ON knowledge_ingestion_repair_queue (source_id)
    WHERE state IN ('queued', 'retrying');

CREATE INDEX IF NOT EXISTS idx_knowledge_ingestion_repair_workspace_state
    ON knowledge_ingestion_repair_queue (workspace_id, state, created_at DESC);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('ingestion_repair_queue', 'knowledge_ingestion_repair_queue',
     'KnowledgeSource', 'authority', '0164_knowledge_ingestion_repair_queue.sql', 'MT-094')
ON CONFLICT (family_key) DO NOTHING;
