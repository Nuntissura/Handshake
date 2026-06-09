-- WP-KERNEL-005 MT-029 follow-up: rejection-audit rows must reference the
-- same batch as their item. The original table had independent FKs for
-- item_id and batch_id, which allowed direct SQL to pair a valid item with a
-- different valid batch.

DELETE FROM atelier_intake_item_rejection_audit audit
WHERE NOT EXISTS (
    SELECT 1
    FROM atelier_intake_item item
    WHERE item.item_id = audit.item_id
      AND item.batch_id = audit.batch_id
);

ALTER TABLE atelier_intake_item_rejection_audit
    DROP CONSTRAINT IF EXISTS fk_atelier_intake_rejection_audit_item_batch;

ALTER TABLE atelier_intake_item
    DROP CONSTRAINT IF EXISTS uq_atelier_intake_item_item_batch;

ALTER TABLE atelier_intake_item
    ADD CONSTRAINT uq_atelier_intake_item_item_batch UNIQUE (item_id, batch_id);

ALTER TABLE atelier_intake_item_rejection_audit
    ADD CONSTRAINT fk_atelier_intake_rejection_audit_item_batch
    FOREIGN KEY (item_id, batch_id)
    REFERENCES atelier_intake_item(item_id, batch_id)
    ON DELETE CASCADE;
