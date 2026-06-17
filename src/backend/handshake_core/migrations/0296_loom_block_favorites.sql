-- WP-KERNEL-009 MT-183 PinsFavoritesAndUnlinked.
--
-- Favorites are separate from pins: pins drive the reorderable grid, while
-- favorites are a durable operator bookmark/favorites view over Loom blocks.
-- Additive + nullable-safe integer flag, matching the existing pinned shape.

ALTER TABLE loom_blocks
    ADD COLUMN IF NOT EXISTS favorite INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_loom_blocks_favorites
    ON loom_blocks (workspace_id, updated_at, block_id)
    WHERE favorite <> 0;
