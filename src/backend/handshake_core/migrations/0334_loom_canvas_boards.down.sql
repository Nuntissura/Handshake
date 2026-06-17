-- WP-KERNEL-009 MT-261 CanvasBoard — down migration (replay-safe).

DROP TABLE IF EXISTS loom_canvas_visual_edges;
DROP TABLE IF EXISTS loom_canvas_placements;
DROP TABLE IF EXISTS loom_canvas_boards;

-- Restore the original loom_blocks content_type allow-list (drops 'canvas').
ALTER TABLE loom_blocks
    DROP CONSTRAINT IF EXISTS loom_blocks_content_type_check;

ALTER TABLE loom_blocks
    ADD CONSTRAINT loom_blocks_content_type_check
    CHECK (content_type IN (
        'note',
        'file',
        'annotated_file',
        'tag_hub',
        'journal'
    ));
