-- WP-KERNEL-009 MT-183 PinsFavoritesAndUnlinked (down).
DROP INDEX IF EXISTS idx_loom_blocks_pins_order;
ALTER TABLE loom_blocks DROP COLUMN IF EXISTS pin_order;
