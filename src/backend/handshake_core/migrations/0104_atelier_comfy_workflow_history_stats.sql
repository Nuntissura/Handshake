-- WP-KERNEL-005 MT-103: workflow history/stats query projection.
-- Character is projected from durable receipt evidence so history and stats can
-- filter by character without parsing free-form JSON on every read.

ALTER TABLE atelier_comfy_workflow_receipt
    ADD COLUMN IF NOT EXISTS character_ref TEXT;

UPDATE atelier_comfy_workflow_receipt
SET character_ref = COALESCE(
    evidence->>'character_ref',
    receipt_json #>> '{evidence,character_ref}'
)
WHERE character_ref IS NULL
  AND COALESCE(evidence->>'character_ref', receipt_json #>> '{evidence,character_ref}') IS NOT NULL;

ALTER TABLE atelier_comfy_workflow_receipt
    DROP CONSTRAINT IF EXISTS chk_atelier_comfy_workflow_receipt_character_ref,
    ADD CONSTRAINT chk_atelier_comfy_workflow_receipt_character_ref
        CHECK (
            character_ref IS NULL
            OR (
                btrim(character_ref) = character_ref
                AND character_ref <> ''
            )
        );

CREATE INDEX IF NOT EXISTS idx_atelier_comfy_workflow_receipt_history
    ON atelier_comfy_workflow_receipt(character_ref, workflow_spec_ref, status, created_at_utc);
