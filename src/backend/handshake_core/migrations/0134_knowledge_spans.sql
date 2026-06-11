-- WP-KERNEL-009 MT-055 PostgresEventLedgerCore-055-KnowledgeSpanTables.
-- Master Spec anchor: 2.3.13.11 KnowledgeSpan ("a byte, text, AST,
-- media-time, page, cell, or rich-document range anchored to a
-- KnowledgeSource. A span is the minimum citeable evidence unit for claims
-- and graph edges.").
--
-- NOTE ON MIGRATION ORDER: spans land at 0134, BEFORE entities (0135) and
-- edges (0136), because both carry REQUIRED span evidence refs. The MT
-- numbering (053 entities, 054 edges, 055 spans) is contract grouping, not a
-- dependency order; the FK graph dictates spans first.
--
-- Range semantics by span_kind (range_start/range_end are unit-agnostic):
--   byte       -> byte offsets into the source content
--   text       -> UTF-8 character offsets
--   ast        -> byte offsets of the AST node; section_path holds the node path
--   media_time -> milliseconds into the media timeline
--   page       -> page numbers (start/end page)
--   cell       -> row index range; section_path holds the sheet/column address
--   rich_doc   -> block ordinal range; section_path holds the block path

CREATE TABLE IF NOT EXISTS knowledge_spans (
    span_id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL
        REFERENCES knowledge_sources(source_id) ON DELETE CASCADE,
    span_kind TEXT NOT NULL CHECK (span_kind IN (
        'byte', 'text', 'ast', 'media_time', 'page', 'cell', 'rich_doc'
    )),
    range_start BIGINT NOT NULL CHECK (range_start >= 0),
    range_end BIGINT NOT NULL,
    line_start INTEGER,
    line_end INTEGER,
    -- AST node path, heading path, sheet/cell address, or rich-doc block path.
    section_path TEXT,
    -- SHA-256 of the exact span content: re-anchoring evidence after edits.
    content_sha256 TEXT NOT NULL,
    parser_version TEXT NOT NULL,
    extraction_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    -- Provenance: which index run extracted this span.
    index_run_id TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    display_snippet TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_spans_id CHECK (span_id ~ '^KSP-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_spans_range CHECK (range_end >= range_start),
    CONSTRAINT chk_knowledge_spans_lines
        CHECK (
            (line_start IS NULL AND line_end IS NULL)
            OR (line_start IS NOT NULL AND line_end IS NOT NULL
                AND line_start >= 1 AND line_end >= line_start)
        ),
    CONSTRAINT chk_knowledge_spans_content_sha256
        CHECK (content_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_knowledge_spans_parser_version
        CHECK (btrim(parser_version) = parser_version AND parser_version <> '')
);

CREATE INDEX IF NOT EXISTS idx_knowledge_spans_source
    ON knowledge_spans (source_id, range_start);

CREATE INDEX IF NOT EXISTS idx_knowledge_spans_run
    ON knowledge_spans (index_run_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_spans_content
    ON knowledge_spans (content_sha256);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('spans', 'knowledge_spans', 'KnowledgeSpan',
     'authority', '0134_knowledge_spans.sql', 'MT-055')
ON CONFLICT (family_key) DO NOTHING;
