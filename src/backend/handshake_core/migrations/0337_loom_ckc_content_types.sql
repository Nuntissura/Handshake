-- WP-KERNEL-012 MT-046 (CKC native editors): widen the loom_blocks content_type
-- allow-list to admit the two CKC surfaces as first-class typed LoomBlocks
-- ('ckc_moodboard', 'ckc_character'). Without these the frontend CKC editors
-- (MT-046 IC-03/04) fall back to a plain note, losing the moodboard/character
-- surface. The CKC blocks are ordinary typed LoomBlocks: they do NOT set
-- view_definition_json, so the 0335 chk_loom_blocks_view_definition constraint
-- (view_definition_json IS NULL for every non-view_def block) is satisfied with
-- the column left NULL. Authority is PostgreSQL + EventLedger; the React CKC
-- editors are projections only. Additive; mirrors the 0334/0335 widening.

ALTER TABLE loom_blocks
    DROP CONSTRAINT IF EXISTS loom_blocks_content_type_check;

ALTER TABLE loom_blocks
    ADD CONSTRAINT loom_blocks_content_type_check
    CHECK (content_type IN (
        'note',
        'file',
        'annotated_file',
        'tag_hub',
        'journal',
        'canvas',
        'view_def',
        'ckc_moodboard',
        'ckc_character'
    ));
