-- WP-KERNEL-012 MT-080 FIX A (CARD-ORIGIN) — down migration (replay-safe).
-- Drops the text-card origin flag. Placements revert to carrying no card-kind
-- hint (the pre-fix behaviour where text cards only survived same-session).

ALTER TABLE loom_canvas_placements
    DROP COLUMN IF EXISTS is_text_card;
