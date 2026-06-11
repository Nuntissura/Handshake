-- WP-KERNEL-005 MT-138 hardening for existing state-probe catalog tables.
-- Constraints are NOT VALID so interrupted/bootstrap test rows do not block
-- the migration, while all new writes are guarded.

ALTER TABLE atelier_state_probe_catalog_entry
    DROP CONSTRAINT IF EXISTS chk_atelier_state_probe_status;

ALTER TABLE atelier_state_probe_catalog_entry
    ADD CONSTRAINT chk_atelier_state_probe_status
    CHECK (status = 'ready')
    NOT VALID;

ALTER TABLE atelier_state_probe_catalog_entry
    DROP CONSTRAINT IF EXISTS chk_atelier_state_probe_read_model;

ALTER TABLE atelier_state_probe_catalog_entry
    ADD CONSTRAINT chk_atelier_state_probe_read_model
    CHECK (read_model = 'postgres_event_ledger_projection')
    NOT VALID;

ALTER TABLE atelier_state_probe_catalog_entry
    DROP CONSTRAINT IF EXISTS chk_atelier_state_probe_required_pre_visual;

ALTER TABLE atelier_state_probe_catalog_entry
    ADD CONSTRAINT chk_atelier_state_probe_required_pre_visual
    CHECK (required_before_visual_inspection IS TRUE)
    NOT VALID;

ALTER TABLE atelier_state_probe_catalog_entry
    DROP CONSTRAINT IF EXISTS chk_atelier_state_probe_fields_object;

ALTER TABLE atelier_state_probe_catalog_entry
    ADD CONSTRAINT chk_atelier_state_probe_fields_object
    CHECK (
        jsonb_typeof(probe_fields) = 'object'
        AND jsonb_typeof(probe_fields->'fields') = 'array'
        AND jsonb_array_length(probe_fields->'fields') > 0
        AND probe_fields->>'schema' = 'hsk.atelier.state_probe.fields@1'
        AND probe_fields->>'state_authority' = 'postgres'
        AND probe_fields->>'event_authority' = 'kernel_event_ledger'
    )
    NOT VALID;

ALTER TABLE atelier_state_probe_catalog_entry
    DROP CONSTRAINT IF EXISTS chk_atelier_state_probe_evidence_refs_array;

ALTER TABLE atelier_state_probe_catalog_entry
    ADD CONSTRAINT chk_atelier_state_probe_evidence_refs_array
    CHECK (
        jsonb_typeof(evidence_refs) = 'array'
        AND jsonb_array_length(evidence_refs) > 0
    )
    NOT VALID;
