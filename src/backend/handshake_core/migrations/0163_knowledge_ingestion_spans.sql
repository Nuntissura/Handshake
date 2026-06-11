-- WP-KERNEL-009 MT-087 SourceIngestionAndEvidence-087-PdfTranscriptImportPath
-- (span persistence surface shared by MT-087 PDF pages, MT-088 media-time
-- cues, MT-089 JSON-pointer structures, MT-090 heading/paragraph blocks,
-- MT-091 redaction markers).
--
-- Master Spec anchor: 2.3.13.11 KnowledgeSpan ("a byte, text, AST,
-- media-time, page, cell, or rich-document range anchored to a
-- KnowledgeSource. A span is the minimum citeable evidence unit").
--
-- knowledge_ingestion_spans stores the INGESTION-side extraction output:
-- each row is one citable span produced by one extraction attempt (FK to its
-- receipt, so every span is traceable to extractor id+version, run token,
-- and EventLedger evidence). Anchors are typed per kind; content is the
-- POST-REDACTION text (MT-091: raw secret bytes never land here) with a
-- verifiability hash of exactly what is stored. Wikilink/link candidates
-- (MT-090) ride along as JSONB for later graph work.
--
-- Re-extraction model: spans of a previous attempt for the same source are
-- deleted when a NEW successful/partial attempt persists its spans (the
-- receipt trail of prior attempts remains; span rows always reflect the
-- newest extraction of the source content).

CREATE TABLE IF NOT EXISTS knowledge_ingestion_spans (
    span_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    source_id TEXT NOT NULL REFERENCES knowledge_sources(source_id) ON DELETE CASCADE,
    receipt_id TEXT NOT NULL REFERENCES knowledge_ingestion_receipts(receipt_id) ON DELETE CASCADE,
    span_index INTEGER NOT NULL CHECK (span_index >= 0),
    anchor_kind TEXT NOT NULL CHECK (anchor_kind IN (
        'byte_range', 'line_range', 'pdf_page', 'media_time',
        'json_pointer', 'heading_path'
    )),
    -- Typed anchor payload (serde shape of knowledge_ingestion::spans::SpanAnchor).
    anchor JSONB NOT NULL,
    byte_start BIGINT CHECK (byte_start IS NULL OR byte_start >= 0),
    byte_end BIGINT CHECK (byte_end IS NULL OR byte_end >= 0),
    -- Post-redaction span text. Never raw secret bytes (MT-091).
    content TEXT NOT NULL,
    -- SHA-256 of exactly the stored content (verifiability hash).
    content_hash TEXT NOT NULL,
    redaction_state TEXT NOT NULL DEFAULT 'none'
        CHECK (redaction_state IN ('none', 'redacted')),
    link_candidates JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_ingestion_spans_id
        CHECK (span_id ~ '^KISP-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_ingestion_spans_hash
        CHECK (content_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_knowledge_ingestion_spans_anchor_shape
        CHECK (jsonb_typeof(anchor) = 'object'),
    CONSTRAINT chk_knowledge_ingestion_spans_links_shape
        CHECK (jsonb_typeof(link_candidates) = 'array'),
    CONSTRAINT chk_knowledge_ingestion_spans_byte_order
        CHECK (byte_start IS NULL OR byte_end IS NULL OR byte_end >= byte_start),
    CONSTRAINT uq_knowledge_ingestion_spans_receipt_index
        UNIQUE (receipt_id, span_index)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_ingestion_spans_source
    ON knowledge_ingestion_spans (source_id, span_index);

CREATE INDEX IF NOT EXISTS idx_knowledge_ingestion_spans_workspace
    ON knowledge_ingestion_spans (workspace_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_ingestion_spans_anchor_kind
    ON knowledge_ingestion_spans (anchor_kind);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('ingestion_spans', 'knowledge_ingestion_spans',
     'KnowledgeSpan', 'authority', '0163_knowledge_ingestion_spans.sql', 'MT-087')
ON CONFLICT (family_key) DO NOTHING;
