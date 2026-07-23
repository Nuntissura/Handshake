DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM model_lane_cloud_projection_plans)
       OR EXISTS (SELECT 1 FROM model_lane_cloud_consent_receipts)
       OR EXISTS (
            SELECT 1
            FROM kernel_event_ledger
            WHERE aggregate_type IN (
                'model_lane_cloud_projection_plan',
                'model_lane_cloud_consent_receipt'
            )
              AND payload->>'schema_id' IN (
                'hsk.model_lane_cloud_projection_plan@2',
                'hsk.model_lane_cloud_consent_receipt@2'
              )
       ) THEN
        RAISE EXCEPTION 'cannot downgrade 0353 while any cloud plan/receipt authority exists; transform and re-ledger authority first';
    END IF;
END $$;

DROP INDEX IF EXISTS idx_model_lane_cloud_consent_receipts_targets_v2;
DROP INDEX IF EXISTS idx_model_lane_cloud_projection_plans_targets_v2;
DROP INDEX IF EXISTS uq_model_lane_cloud_projection_plans_single_run_legacy_v1;
DROP INDEX IF EXISTS uq_model_lane_cloud_projection_plans_single_run_v2;
DROP INDEX IF EXISTS uq_model_lane_cloud_projection_plans_single_lane_v2;

ALTER TABLE model_lane_cloud_consent_receipts
    DROP CONSTRAINT IF EXISTS chk_model_lane_cloud_consent_receipts_binding_shape_v2,
    DROP CONSTRAINT IF EXISTS chk_model_lane_cloud_consent_receipts_revocation_input_hash_v2,
    DROP CONSTRAINT IF EXISTS chk_model_lane_cloud_consent_receipts_target_hash_v2,
    DROP CONSTRAINT IF EXISTS chk_model_lane_cloud_consent_receipts_targets_v2,
    DROP CONSTRAINT IF EXISTS chk_model_lane_cloud_consent_receipts_scope_v2,
    DROP COLUMN revocation_input_hash,
    DROP COLUMN target_bindings_hash,
    DROP COLUMN target_bindings,
    ALTER COLUMN requested_model_id SET NOT NULL,
    ALTER COLUMN provider_kind SET NOT NULL,
    ALTER COLUMN model_session_id SET NOT NULL,
    ALTER COLUMN lane_id SET NOT NULL;

ALTER TABLE model_lane_cloud_projection_plans
    DROP CONSTRAINT IF EXISTS chk_model_lane_cloud_projection_plans_binding_shape_v2,
    DROP CONSTRAINT IF EXISTS chk_model_lane_cloud_projection_plans_target_hash_v2,
    DROP CONSTRAINT IF EXISTS chk_model_lane_cloud_projection_plans_targets_v2,
    DROP CONSTRAINT IF EXISTS chk_model_lane_cloud_projection_plans_scope_v2,
    DROP COLUMN target_bindings_hash,
    DROP COLUMN target_bindings,
    ALTER COLUMN requested_model_id SET NOT NULL,
    ALTER COLUMN provider_kind SET NOT NULL,
    ALTER COLUMN model_session_id SET NOT NULL,
    ALTER COLUMN lane_id SET NOT NULL,
    ADD CONSTRAINT uq_model_lane_cloud_projection_plans_lane
        UNIQUE (run_id, lane_id, model_session_id, provider_kind);

DELETE FROM model_lane_schema_registry
WHERE schema_id IN (
    'hsk.model_lane_cloud_projection_plan@2',
    'hsk.model_lane_cloud_consent_receipt@2'
);
