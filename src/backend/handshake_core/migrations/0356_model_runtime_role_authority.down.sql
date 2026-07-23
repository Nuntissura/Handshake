ALTER TABLE model_runtime_registry
    DROP CONSTRAINT IF EXISTS chk_model_runtime_registry_runtime_role,
    DROP CONSTRAINT IF EXISTS chk_model_runtime_registry_schema_id;

UPDATE model_runtime_registry
SET schema_id = 'hsk.model_runtime_registry.row@1';

ALTER TABLE model_runtime_registry
    ADD CONSTRAINT chk_model_runtime_registry_schema_id
        CHECK (schema_id = 'hsk.model_runtime_registry.row@1'),
    DROP COLUMN IF EXISTS runtime_role;
