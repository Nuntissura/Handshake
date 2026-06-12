-- WP-KERNEL-009 MT-183 PinsFavoritesAndUnlinked.
--
-- Master Spec anchor: §10.12 §7.1 / §7.1.4.3 / [LM-VIEW-004] — the Pins view is
-- a "pinned blocks; user-reorderable grid". The Loom MVP (migration 0013)
-- carries a `pinned` boolean but no user-defined order, so the Pins grid cannot
-- be reordered. This migration adds a nullable `pin_order` to loom_blocks: a
-- stable integer ordinal the operator controls. NULL pin_order sorts after
-- explicitly-ordered pins (newly pinned blocks land at the end until ordered),
-- then by updated_at DESC for a deterministic, stable Pins feed.
--
-- Additive + nullable: safe on the existing table; no backfill required.

ALTER TABLE loom_blocks
    ADD COLUMN IF NOT EXISTS pin_order INTEGER;

-- Index the Pins ordering (pinned rows, by pin_order then recency). Partial
-- index keeps it small (only pinned blocks participate in the Pins view).
CREATE INDEX IF NOT EXISTS idx_loom_blocks_pins_order
    ON loom_blocks (workspace_id, pin_order, updated_at)
    WHERE pinned <> 0;
