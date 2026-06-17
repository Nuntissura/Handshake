DROP INDEX IF EXISTS idx_loom_blocks_favorites;
ALTER TABLE loom_blocks DROP COLUMN IF EXISTS favorite;
