-- WP-KERNEL-005 MT-100: preserve optional identity landmarks,
-- measurements, and pose metadata as workflow receipt inputs on Comfy intake
-- outputs. Absence is represented explicitly as {"identity": {}}.

ALTER TABLE atelier_comfy_intake_output
    ADD COLUMN IF NOT EXISTS workflow_input_metadata JSONB NOT NULL DEFAULT '{"identity": {}}'::jsonb;

UPDATE atelier_comfy_intake_output
SET workflow_input_metadata = '{"identity": {}}'::jsonb
WHERE workflow_input_metadata IS NULL
   OR workflow_input_metadata = '{}'::jsonb;

ALTER TABLE atelier_comfy_intake_output
    DROP CONSTRAINT IF EXISTS chk_atelier_comfy_intake_output_workflow_input_metadata_json,
    ADD CONSTRAINT chk_atelier_comfy_intake_output_workflow_input_metadata_json
        CHECK (jsonb_typeof(workflow_input_metadata) = 'object');
