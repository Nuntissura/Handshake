CREATE TABLE IF NOT EXISTS loom_block_view_fr_outbox_retention_archive
AS TABLE loom_block_view_fr_outbox WITH NO DATA;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_loom_block_view_fr_outbox_archive_workspace'
          AND conrelid = 'loom_block_view_fr_outbox_retention_archive'::regclass
    ) THEN
        ALTER TABLE loom_block_view_fr_outbox_retention_archive
            ADD CONSTRAINT fk_loom_block_view_fr_outbox_archive_workspace
            FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE;
    END IF;
END
$$;

TRUNCATE TABLE loom_block_view_fr_outbox_retention_archive;

INSERT INTO loom_block_view_fr_outbox_retention_archive
SELECT outbox.*
FROM loom_block_view_fr_outbox AS outbox
LEFT JOIN loom_blocks AS block
  ON block.block_id = outbox.block_id
WHERE block.block_id IS NULL;

DELETE FROM loom_block_view_fr_outbox AS outbox
WHERE NOT EXISTS (
    SELECT 1
    FROM loom_blocks AS block
    WHERE block.block_id = outbox.block_id
);

ALTER TABLE loom_block_view_fr_outbox
    ADD CONSTRAINT fk_loom_block_view_fr_outbox_block
    FOREIGN KEY (block_id) REFERENCES loom_blocks(block_id) ON DELETE CASCADE;
