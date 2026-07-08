-- WP-KERNEL-012 MT-046 (CKC content types) — down migration (replay-safe).
-- Restores the pre-CKC 0335 content_type allow-list (the 7-value set, without
-- the two CKC surfaces). Only safe to replay when no loom_blocks rows still
-- carry a CKC content type; otherwise the re-added CHECK rejects them, which is
-- the intended guard.

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
        'view_def'
    ));
