-- Intentionally leave pg_trgm in public. Migration 0325 normalizes the shared
-- database extension location so schema-pinned connections can call
-- public.similarity and OPERATOR(public.%); rollback only removes the indexes
-- owned by this migration.

DROP INDEX IF EXISTS idx_knowledge_wiki_projections_quick_switcher_fuzzy_title;
DROP INDEX IF EXISTS idx_knowledge_wiki_projections_quick_switcher_fuzzy_projection_id;
DROP INDEX IF EXISTS idx_user_manual_sections_quick_switcher_fuzzy_title;
DROP INDEX IF EXISTS idx_user_manual_pages_quick_switcher_fuzzy_title;
DROP INDEX IF EXISTS idx_user_manual_pages_quick_switcher_fuzzy_slug;
DROP INDEX IF EXISTS idx_knowledge_entities_quick_switcher_fuzzy_display_name;
DROP INDEX IF EXISTS idx_knowledge_entities_quick_switcher_fuzzy_entity_key;
DROP INDEX IF EXISTS idx_knowledge_entities_quick_switcher_fuzzy_entity_id;
DROP INDEX IF EXISTS idx_loom_blocks_quick_switcher_fuzzy_filename;
DROP INDEX IF EXISTS idx_loom_blocks_quick_switcher_fuzzy_title;
DROP INDEX IF EXISTS idx_loom_blocks_quick_switcher_fuzzy_block_id;
