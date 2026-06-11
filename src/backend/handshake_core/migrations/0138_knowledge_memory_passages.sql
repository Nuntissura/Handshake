-- WP-KERNEL-009 MT-057 PostgresEventLedgerCore-057-PassageEvidenceTables.
-- Master Spec anchor: 2.3.13.11 MemoryPassage ("a bounded passage eligible
-- for model context. It is derived from sources and claims and MUST record
-- ranking features, retrieval mode, freshness, and compaction policy.").
--
-- MT-057 contract fields: source passages, extracted text, OCR/transcript
-- metadata, extraction confidence, and failure receipts.
--
-- knowledge_passage_evidence records the derivation lineage: every passage
-- names the sources/claims/spans it was derived from with a typed ref kind.
-- A passage with zero evidence rows is rejected at commit (derived-from is a
-- spec MUST, same trigger discipline as edges/claims).

CREATE TABLE IF NOT EXISTS knowledge_memory_passages (
    passage_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- The bounded, model-context-eligible text.
    passage_text TEXT NOT NULL,
    token_count INTEGER CHECK (token_count IS NULL OR token_count >= 0),
    -- OCR / transcript extraction metadata for media-derived passages:
    -- {"ocr_engine": ..., "transcript_lang": ..., "segments": [...]}.
    ocr_transcript_metadata JSONB,
    extraction_confidence DOUBLE PRECISION NOT NULL DEFAULT 1.0
        CHECK (extraction_confidence >= 0.0 AND extraction_confidence <= 1.0),
    -- Ranking features for retrieval: {"bm25_terms": ..., "embedding_ref": ...,
    -- "recency_score": ..., "pin_weight": ...}.
    ranking_features JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- The cheapest authoritative retrieval mode this passage serves
    -- (spec 2.3.14.1.4 / RetrievalTrace mode vocabulary).
    retrieval_mode TEXT NOT NULL DEFAULT 'hybrid_rag'
        CHECK (retrieval_mode IN (
            'none', 'direct_load', 'exact_lookup', 'graph_traversal', 'hybrid_rag'
        )),
    -- Freshness: when the underlying evidence was last verified current.
    freshness_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    compaction_policy TEXT NOT NULL DEFAULT 'keep'
        CHECK (compaction_policy IN ('keep', 'compactable', 'expired')),
    -- Extraction failure receipt (EventLedger) for passages whose
    -- OCR/transcript/parse pipeline degraded.
    failure_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    derived_in_run TEXT
        REFERENCES knowledge_index_runs(index_run_id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_memory_passages_id
        CHECK (passage_id ~ '^KMP-[0-9a-f]{32}$'),
    CONSTRAINT chk_knowledge_memory_passages_text
        CHECK (passage_text <> '')
);

CREATE INDEX IF NOT EXISTS idx_knowledge_memory_passages_workspace
    ON knowledge_memory_passages (workspace_id, compaction_policy);

CREATE INDEX IF NOT EXISTS idx_knowledge_memory_passages_freshness
    ON knowledge_memory_passages (freshness_at);

-- Derivation lineage: passages are derived from sources, claims, and spans.
CREATE TABLE IF NOT EXISTS knowledge_passage_evidence (
    passage_id TEXT NOT NULL
        REFERENCES knowledge_memory_passages(passage_id) ON DELETE CASCADE,
    ref_kind TEXT NOT NULL CHECK (ref_kind IN ('source', 'claim', 'span')),
    source_id TEXT REFERENCES knowledge_sources(source_id) ON DELETE CASCADE,
    claim_id TEXT REFERENCES knowledge_claims(claim_id) ON DELETE CASCADE,
    span_id TEXT REFERENCES knowledge_spans(span_id) ON DELETE RESTRICT,
    ordinal INTEGER NOT NULL DEFAULT 0 CHECK (ordinal >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Exactly the ref column matching ref_kind must be set.
    CONSTRAINT chk_knowledge_passage_evidence_shape CHECK (
        (ref_kind = 'source' AND source_id IS NOT NULL
            AND claim_id IS NULL AND span_id IS NULL)
        OR (ref_kind = 'claim' AND claim_id IS NOT NULL
            AND source_id IS NULL AND span_id IS NULL)
        OR (ref_kind = 'span' AND span_id IS NOT NULL
            AND source_id IS NULL AND claim_id IS NULL)
    ),
    PRIMARY KEY (passage_id, ref_kind, ordinal)
);

CREATE INDEX IF NOT EXISTS idx_knowledge_passage_evidence_source
    ON knowledge_passage_evidence (source_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_passage_evidence_claim
    ON knowledge_passage_evidence (claim_id);

CREATE OR REPLACE FUNCTION knowledge_passage_requires_evidence() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM knowledge_passage_evidence WHERE passage_id = NEW.passage_id
    ) THEN
        RAISE EXCEPTION
            'knowledge_memory_passages % violates spec 2.3.13.11: passages are derived from sources and claims (knowledge_passage_evidence is empty at commit)',
            NEW.passage_id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END $$;

CREATE CONSTRAINT TRIGGER trg_knowledge_passage_requires_evidence
    AFTER INSERT ON knowledge_memory_passages
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION knowledge_passage_requires_evidence();

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('memory_passages', 'knowledge_memory_passages', 'MemoryPassage',
     'authority', '0138_knowledge_memory_passages.sql', 'MT-057'),
    ('passage_evidence', 'knowledge_passage_evidence', 'MemoryPassage',
     'authority', '0138_knowledge_memory_passages.sql', 'MT-057')
ON CONFLICT (family_key) DO NOTHING;
