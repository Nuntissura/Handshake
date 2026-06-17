-- WP-KERNEL-009 MT-262 BlockCollectionViews — down migration (replay-safe).

-- Drop the view-definition presence constraint first (it references the column).
ALTER TABLE loom_blocks
    DROP CONSTRAINT IF EXISTS chk_loom_blocks_view_definition;

-- Restore the 0334 content_type allow-list (drops 'view_def').
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
        'canvas'
    ));

ALTER TABLE loom_blocks
    DROP COLUMN IF EXISTS view_definition_json;
