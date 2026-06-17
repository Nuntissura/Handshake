-- Down-migration for 0336_loom_search_v2.sql (WP-KERNEL-009 MT-264).
-- Replay-safe: drops the derived search-index projection and its indexes. The
-- pg_trgm / vector extensions are intentionally NOT dropped here -- other
-- objects may depend on them and they are cheap to leave installed; dropping an
-- extension other migrations rely on would not be replay-safe.

DROP INDEX IF EXISTS idx_loom_block_search_ws_type;
DROP INDEX IF EXISTS idx_loom_block_search_embedding;
DROP INDEX IF EXISTS idx_loom_block_search_trgm;
DROP INDEX IF EXISTS idx_loom_block_search_tsv;
DROP TABLE IF EXISTS loom_block_search_index;
