-- WP-KERNEL-009 MT-085 SourceIngestionAndEvidence-085-ExtractionReceiptModel.
-- Master Spec anchor: 2.3.13.11 KnowledgeSource ("last-index receipt") + WP
-- constraint "PDF/media extraction failures must produce receipts and
-- repairable metadata rather than silently indexing partial content".
--
-- knowledge_ingestion_receipts persists ONE row per extraction ATTEMPT
-- (success, partial, failed, deferred, skipped, blocked) — the per-attempt
-- evidence trail behind the per-source rollup in knowledge_sources
-- (parser_status/extraction_status/last_index_receipt_event_id). Receipts
-- carry the extractor identity+version, the typed error class, span counts,
-- redaction counts, the content hash AT EXTRACTION TIME (raw fidelity hash,
-- MT-084), wall-clock duration, and an EventLedger receipt ref so every
-- attempt is replayable kernel evidence.
--
-- Status vocabulary (MT-085 contract: success, partial success, unsupported
-- format, missing text layer, OCR needed, operator repair):
--   * status: success | partial | failed | deferred | skipped | blocked
--   * error_class (required for non-success): UNSUPPORTED_FORMAT,
--     NO_TEXT_LAYER, OCR_NEEDED, ENCRYPTED, MALFORMED_CUE, PARSE_ERROR,
--     OVERSIZE, SECRET_BLOCKED, IO_ERROR, INTERNAL
--   * operator repair linkage is owned by the repair queue (0164):
--     knowledge_ingestion_repair_queue.receipt_id / resolved_receipt_id
--     reference rows of THIS table; receipts carry no repair FK themselves.

CREATE TABLE IF NOT EXISTS knowledge_ingestion_receipts (
    receipt_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    source_id TEXT NOT NULL REFERENCES knowledge_sources(source_id) ON DELETE CASCADE,
    -- Groups every receipt of one ingestion pass (engine-generated token).
    ingestion_run_token TEXT,
    extractor_id TEXT NOT NULL,
    extractor_version TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN (
        'success', 'partial', 'failed', 'deferred', 'skipped', 'blocked'
    )),
    error_class TEXT CHECK (error_class IS NULL OR error_class IN (
        'UNSUPPORTED_FORMAT', 'NO_TEXT_LAYER', 'OCR_NEEDED', 'ENCRYPTED',
        'MALFORMED_CUE', 'PARSE_ERROR', 'OVERSIZE', 'SECRET_BLOCKED',
        'IO_ERROR', 'INTERNAL'
    )),
    error_detail JSONB,
    spans_produced INTEGER NOT NULL DEFAULT 0 CHECK (spans_produced >= 0),
    spans_failed INTEGER NOT NULL DEFAULT 0 CHECK (spans_failed >= 0),
    redaction_count INTEGER NOT NULL DEFAULT 0 CHECK (redaction_count >= 0),
    -- Raw fidelity hash of the source content at extraction time (MT-084).
    content_hash TEXT NOT NULL,
    duration_ms BIGINT NOT NULL DEFAULT 0 CHECK (duration_ms >= 0),
    receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_ingestion_receipts_id
        CHECK (receipt_id ~ '^KIRC-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_ingestion_receipts_hash
        CHECK (content_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_knowledge_ingestion_receipts_extractor
        CHECK (btrim(extractor_id) = extractor_id AND extractor_id <> ''
               AND btrim(extractor_version) = extractor_version AND extractor_version <> ''),
    -- Non-success attempts must say WHY (typed, never silent).
    CONSTRAINT chk_knowledge_ingestion_receipts_error_shape
        CHECK (status IN ('success') OR error_class IS NOT NULL),
    -- Success carries no error class.
    CONSTRAINT chk_knowledge_ingestion_receipts_success_shape
        CHECK (status <> 'success' OR error_class IS NULL)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_ingestion_receipts_source
    ON knowledge_ingestion_receipts (source_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_knowledge_ingestion_receipts_workspace
    ON knowledge_ingestion_receipts (workspace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_knowledge_ingestion_receipts_status
    ON knowledge_ingestion_receipts (status);

CREATE INDEX IF NOT EXISTS idx_knowledge_ingestion_receipts_run_token
    ON knowledge_ingestion_receipts (ingestion_run_token);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('ingestion_receipts', 'knowledge_ingestion_receipts',
     'KnowledgeSource', 'authority', '0162_knowledge_ingestion_receipts.sql', 'MT-085')
ON CONFLICT (family_key) DO NOTHING;
