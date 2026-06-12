-- Revert WP-KERNEL-009 MT-241/242/243 project wiki compile layer columns.
DROP INDEX IF EXISTS idx_knowledge_wiki_projections_cited_sources;
DROP INDEX IF EXISTS idx_knowledge_wiki_projections_page_type;
ALTER TABLE knowledge_wiki_projections
    DROP CONSTRAINT IF EXISTS chk_knowledge_wiki_projections_stamp_shape;
ALTER TABLE knowledge_wiki_projections
    DROP CONSTRAINT IF EXISTS chk_knowledge_wiki_projections_stamp_guard;
ALTER TABLE knowledge_wiki_projections
    DROP CONSTRAINT IF EXISTS chk_knowledge_wiki_projections_page_type;
ALTER TABLE knowledge_wiki_projections
    DROP COLUMN IF EXISTS page_links,
    DROP COLUMN IF EXISTS compile_recipe,
    DROP COLUMN IF EXISTS compile_stamp,
    DROP COLUMN IF EXISTS page_type;
