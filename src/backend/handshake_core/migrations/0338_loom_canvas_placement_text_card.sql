-- WP-KERNEL-012 MT-080 FIX A completion (CARD-ORIGIN): mark canvas placements
-- that were created by the inline text-card editor (create_canvas_card) so the
-- frontend can restore an inline-editable text card across sessions. Without
-- this flag the placement payload carries no card-kind, so a text card only
-- survives inline editing within the same session (the frontend cannot tell a
-- reloaded text-card placement apart from a generic block reference).
--
-- Additive, back-compatible: every existing placement (all generic block
-- references) defaults to FALSE. Only placements created via the text-card
-- endpoint set it TRUE. Authority is PostgreSQL + EventLedger; the React canvas
-- is a projection only.

ALTER TABLE loom_canvas_placements
    ADD COLUMN IF NOT EXISTS is_text_card BOOLEAN NOT NULL DEFAULT FALSE;
