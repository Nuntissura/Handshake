-- MT-014 V2: explicit completion/default-selection authority.
--
-- Existing rows predate the typed role column. The original boot producer used
-- a distinct observer id for dedicated embedding registrations, so the data
-- migration preserves that producer-authored distinction without inferring a
-- role from supports_embedding.

ALTER TABLE model_runtime_registry
    ADD COLUMN runtime_role TEXT;

UPDATE model_runtime_registry
SET runtime_role = CASE
    WHEN last_observed_by = 'handshake-embedded-embedding' THEN 'embedding'
    ELSE 'completion'
END;

ALTER TABLE model_runtime_registry
    DROP CONSTRAINT chk_model_runtime_registry_schema_id;

UPDATE model_runtime_registry
SET schema_id = 'hsk.model_runtime_registry.row@2';

ALTER TABLE model_runtime_registry
    ALTER COLUMN runtime_role SET NOT NULL,
    ADD CONSTRAINT chk_model_runtime_registry_schema_id
        CHECK (schema_id = 'hsk.model_runtime_registry.row@2'),
    ADD CONSTRAINT chk_model_runtime_registry_runtime_role
        CHECK (runtime_role IN ('completion', 'embedding'));
