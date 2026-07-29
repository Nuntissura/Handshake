-- WP-KERNEL-012 MT-027: unpublished audit intent must survive block deletion.

ALTER TABLE loom_block_view_fr_outbox
    DROP CONSTRAINT IF EXISTS fk_loom_block_view_fr_outbox_block;

-- A rollback cannot re-add the block FK while retained rows refer to deleted
-- blocks. The down migration parks those rows in this archive; a later forward
-- migration restores them after the FK is removed.
DO $$
BEGIN
    IF to_regclass('loom_block_view_fr_outbox_retention_archive') IS NOT NULL THEN
        INSERT INTO loom_block_view_fr_outbox
        SELECT *
        FROM loom_block_view_fr_outbox_retention_archive
        ON CONFLICT (event_id) DO NOTHING;

        DROP TABLE loom_block_view_fr_outbox_retention_archive;
    END IF;
END
$$;
