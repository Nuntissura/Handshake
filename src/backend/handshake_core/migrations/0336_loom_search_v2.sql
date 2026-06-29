-- WP-KERNEL-009 MT-264 UnifiedWorkSurface-264-LoomSearchV2 (DEC-008).
-- Master Spec §10.12: ES-class search over the whole Loom corpus, built
-- Postgres-NATIVE (explicitly NOT Elasticsearch / OpenSearch / Solr / any
-- external search daemon) and blended with the Loom graph.
--
-- This migration adds the DERIVED search projection that backs the three
-- search modalities over canonical Loom content:
--   (1) full-text  -> a tsvector GENERATED column + GIN index (ts_rank /
--                     ts_headline relevance + highlight),
--   (2) fuzzy/typo  -> a pg_trgm GIN index over the raw search text,
--   (3) semantic    -> a pgvector column + HNSW kNN over REAL embeddings
--                     produced by the operator's configured model runtime.
--
-- The index is a PROJECTION, never a second source of truth:
--   * loom_block_search_index.block_id is FK -> loom_blocks(block_id) ON DELETE
--     CASCADE, so deleting a block transactionally removes its search row. A
--     deleted block can NEVER surface a stale hit (negative test in
--     loom_search_v2_tests.rs).
--   * search_text is refreshed inside the authority write path (reindex on
--     create/update), EventLedger-receipted.
--
-- pg_trgm normally ships with PostgreSQL contrib; pgvector must be present in
-- the Handshake-managed PostgreSQL. Both are ensured-present below with
-- CREATE EXTENSION IF NOT EXISTS. If pgvector is NOT installed in the managed
-- PostgreSQL binary, this statement FAILS LOUDLY (typed Postgres error) and the
-- whole migration aborts -- we do NOT silently drop the semantic modality.
--
-- Loom-domain table (sibling of loom_blocks / loom_edges / loom_ai_suggestions),
-- so it is intentionally NOT registered in knowledge_schema_registry, whose
-- chk_knowledge_schema_registry_prefix admits `knowledge_`-prefixed tables only
-- (same rationale as 0333_loom_ai_suggestions.sql). Authority remains
-- PostgreSQL + EventLedger.

-- Extensions are pinned to the `public` schema (same convention as the
-- pgcrypto setup in the test harness). Migrations run with a per-schema
-- search_path that does NOT include public, so every extension object below is
-- referenced fully-qualified (public.vector, public.gin_trgm_ops,
-- public.vector_cosine_ops). If a database already installed either extension
-- elsewhere, fail early with a clear invariant instead of later failing on a
-- missing public type/operator.
DO $$
DECLARE
    existing_schema TEXT;
BEGIN
    SELECT n.nspname
      INTO existing_schema
      FROM pg_extension e
      JOIN pg_namespace n ON n.oid = e.extnamespace
     WHERE e.extname = 'pg_trgm';

    IF existing_schema IS NULL THEN
        CREATE EXTENSION pg_trgm WITH SCHEMA public;
    ELSIF existing_schema <> 'public' THEN
        RAISE EXCEPTION
            'Handshake requires pg_trgm extension in schema public; found schema %',
            existing_schema
            USING ERRCODE = 'invalid_schema_name';
    END IF;
END $$;

DO $$
DECLARE
    existing_schema TEXT;
BEGIN
    SELECT n.nspname
      INTO existing_schema
      FROM pg_extension e
      JOIN pg_namespace n ON n.oid = e.extnamespace
     WHERE e.extname = 'vector';

    IF existing_schema IS NULL THEN
        CREATE EXTENSION vector WITH SCHEMA public;
    ELSIF existing_schema <> 'public' THEN
        RAISE EXCEPTION
            'Handshake requires vector extension in schema public; found schema %',
            existing_schema
            USING ERRCODE = 'invalid_schema_name';
    END IF;
END $$;

-- The canonical embedding dimensionality for LoomSearchV2. 768 matches common
-- local embedding models (e.g. nomic-embed-text). The HNSW index requires a
-- fixed dim; the reindex layer validates the model output dim against this.
CREATE TABLE IF NOT EXISTS loom_block_search_index (
    block_id TEXT PRIMARY KEY
        REFERENCES loom_blocks(block_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    workspace_id TEXT NOT NULL
        REFERENCES workspaces(id) ON UPDATE RESTRICT ON DELETE CASCADE,
    content_type TEXT NOT NULL,
    -- The flattened, human-readable text the row indexes (title +
    -- original_filename + derived full_text_index). Refreshed on every block
    -- write inside the authority transaction.
    search_text TEXT NOT NULL DEFAULT '',
    -- Generated full-text vector (english config). STORED so the GIN index and
    -- ts_rank/ts_headline read it without recomputation. Always consistent with
    -- search_text because it is GENERATED.
    search_tsv tsvector GENERATED ALWAYS AS (to_tsvector('english', search_text)) STORED,
    -- Real dense embedding of search_text (pgvector). NULL when no embedding
    -- model is configured (the semantic modality degrades to keyword/trigram --
    -- it is NEVER fabricated).
    embedding public.vector(768),
    -- Provenance of the embedding (model id) for receipts; NULL when none.
    embedding_model TEXT,
    indexed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- (1) Full-text GIN over the generated tsvector.
CREATE INDEX IF NOT EXISTS idx_loom_block_search_tsv
    ON loom_block_search_index USING GIN (search_tsv);

-- (2) Fuzzy/substring trigram GIN over the raw search text.
CREATE INDEX IF NOT EXISTS idx_loom_block_search_trgm
    ON loom_block_search_index USING GIN (search_text public.gin_trgm_ops);

-- (3) Semantic HNSW kNN over the embedding (cosine distance, vector_cosine_ops).
CREATE INDEX IF NOT EXISTS idx_loom_block_search_embedding
    ON loom_block_search_index USING hnsw (embedding public.vector_cosine_ops);

-- Facet/scope helper: workspace + content_type filtering.
CREATE INDEX IF NOT EXISTS idx_loom_block_search_ws_type
    ON loom_block_search_index (workspace_id, content_type);
