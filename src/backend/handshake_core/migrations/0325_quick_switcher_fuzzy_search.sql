-- WP-KERNEL-009 MT-256 QuickSwitcher fuzzy title search.
-- pg_trgm backs typo-tolerant candidate filtering for the same
-- title/identifier expressions used by the Loom graph quick-open backend.
-- Policy: pg_trgm is normalized into public because WP-KERNEL-009 tests and
-- runtime connections pin search_path to per-test/per-runtime schemas while
-- Rust search SQL intentionally calls public.similarity and OPERATOR(public.%).
-- The down migration removes this migration's indexes but does not move/drop
-- pg_trgm; extension ownership/location is database-level shared state.

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
        ALTER EXTENSION pg_trgm SET SCHEMA public;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_loom_blocks_quick_switcher_fuzzy_block_id
    ON loom_blocks
    USING GIN (
        lower(block_id) public.gin_trgm_ops
    );

CREATE INDEX IF NOT EXISTS idx_loom_blocks_quick_switcher_fuzzy_title
    ON loom_blocks
    USING GIN (
        lower(COALESCE(title, '')) public.gin_trgm_ops
    );

CREATE INDEX IF NOT EXISTS idx_loom_blocks_quick_switcher_fuzzy_filename
    ON loom_blocks
    USING GIN (
        lower(COALESCE(original_filename, '')) public.gin_trgm_ops
    );

CREATE INDEX IF NOT EXISTS idx_knowledge_entities_quick_switcher_fuzzy_entity_id
    ON knowledge_entities
    USING GIN (
        lower(entity_id) public.gin_trgm_ops
    );

CREATE INDEX IF NOT EXISTS idx_knowledge_entities_quick_switcher_fuzzy_entity_key
    ON knowledge_entities
    USING GIN (
        lower(entity_key) public.gin_trgm_ops
    );

CREATE INDEX IF NOT EXISTS idx_knowledge_entities_quick_switcher_fuzzy_display_name
    ON knowledge_entities
    USING GIN (
        lower(display_name) public.gin_trgm_ops
    );

CREATE INDEX IF NOT EXISTS idx_user_manual_pages_quick_switcher_fuzzy_slug
    ON user_manual_pages
    USING GIN (
        lower(slug) public.gin_trgm_ops
    );

CREATE INDEX IF NOT EXISTS idx_user_manual_pages_quick_switcher_fuzzy_title
    ON user_manual_pages
    USING GIN (
        lower(title) public.gin_trgm_ops
    );

CREATE INDEX IF NOT EXISTS idx_user_manual_sections_quick_switcher_fuzzy_title
    ON user_manual_sections
    USING GIN (
        lower(title) public.gin_trgm_ops
    );

CREATE INDEX IF NOT EXISTS idx_knowledge_wiki_projections_quick_switcher_fuzzy_projection_id
    ON knowledge_wiki_projections
    USING GIN (
        lower(projection_id) public.gin_trgm_ops
    );

CREATE INDEX IF NOT EXISTS idx_knowledge_wiki_projections_quick_switcher_fuzzy_title
    ON knowledge_wiki_projections
    USING GIN (
        lower(title) public.gin_trgm_ops
    );
