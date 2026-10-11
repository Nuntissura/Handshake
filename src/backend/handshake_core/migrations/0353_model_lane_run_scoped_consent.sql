-- MT-017 AC-1: literal run-scoped SingleRun cloud consent plus revocation idempotency.
INSERT INTO model_lane_schema_registry
    (schema_id, schema_version, record_kind, table_name)
VALUES
    ('hsk.model_lane_cloud_projection_plan@2', 2, 'ModelLaneCloudProjectionPlan', 'model_lane_cloud_projection_plans'),
    ('hsk.model_lane_cloud_consent_receipt@2', 2, 'ModelLaneCloudConsentReceipt', 'model_lane_cloud_consent_receipts')
ON CONFLICT (schema_id) DO UPDATE SET
    schema_version = EXCLUDED.schema_version,
    record_kind = EXCLUDED.record_kind,
    table_name = EXCLUDED.table_name,
    source_component = EXCLUDED.source_component;

ALTER TABLE model_lane_cloud_projection_plans
    DROP CONSTRAINT IF EXISTS uq_model_lane_cloud_projection_plans_lane;

ALTER TABLE model_lane_cloud_projection_plans
    ALTER COLUMN lane_id DROP NOT NULL,
    ALTER COLUMN model_session_id DROP NOT NULL,
    ALTER COLUMN provider_kind DROP NOT NULL,
    ALTER COLUMN requested_model_id DROP NOT NULL,
    ADD COLUMN target_bindings JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN target_bindings_hash TEXT;

ALTER TABLE model_lane_cloud_consent_receipts
    ALTER COLUMN lane_id DROP NOT NULL,
    ALTER COLUMN model_session_id DROP NOT NULL,
    ALTER COLUMN provider_kind DROP NOT NULL,
    ALTER COLUMN requested_model_id DROP NOT NULL,
    ADD COLUMN target_bindings JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN target_bindings_hash TEXT,
    ADD COLUMN revocation_input_hash TEXT;

ALTER TABLE model_lane_cloud_projection_plans
    ADD CONSTRAINT chk_model_lane_cloud_projection_plans_scope_v2
        CHECK (consent_scope IN ('single_lane', 'single_run')),
    ADD CONSTRAINT chk_model_lane_cloud_projection_plans_targets_v2
        CHECK (jsonb_typeof(target_bindings) = 'array'),
    ADD CONSTRAINT chk_model_lane_cloud_projection_plans_target_hash_v2
        CHECK (target_bindings_hash IS NULL OR target_bindings_hash ~ '^[0-9a-f]{64}$'),
    ADD CONSTRAINT chk_model_lane_cloud_projection_plans_binding_shape_v2
        CHECK (
            (consent_scope = 'single_lane'
             AND lane_id IS NOT NULL
             AND model_session_id IS NOT NULL
             AND provider_kind IS NOT NULL
             AND requested_model_id IS NOT NULL
             AND jsonb_array_length(target_bindings) = 0
             AND target_bindings_hash IS NULL)
            OR
            (consent_scope = 'single_run'
             AND (
                 -- Native v2 run-wide authority has no lane-bound identity.
                 (lane_id IS NULL
                  AND model_session_id IS NULL
                  AND provider_kind IS NULL
                  AND requested_model_id IS NULL)
                 OR
                 -- Preserve pre-0353 v1 SingleRun authority byte-for-byte.
                 -- The current runtime rejects this lane-bound legacy shape
                 -- for launch, so retaining it cannot widen old consent.
                 (lane_id IS NOT NULL
                  AND model_session_id IS NOT NULL
                  AND provider_kind IS NOT NULL
                  AND requested_model_id IS NOT NULL)
             )
             AND jsonb_array_length(target_bindings) = 0
             AND target_bindings_hash IS NULL)
        );

ALTER TABLE model_lane_cloud_consent_receipts
    ADD CONSTRAINT chk_model_lane_cloud_consent_receipts_scope_v2
        CHECK (consent_scope IN ('single_lane', 'single_run')),
    ADD CONSTRAINT chk_model_lane_cloud_consent_receipts_targets_v2
        CHECK (jsonb_typeof(target_bindings) = 'array'),
    ADD CONSTRAINT chk_model_lane_cloud_consent_receipts_target_hash_v2
        CHECK (target_bindings_hash IS NULL OR target_bindings_hash ~ '^[0-9a-f]{64}$'),
    ADD CONSTRAINT chk_model_lane_cloud_consent_receipts_revocation_input_hash_v2
        CHECK (revocation_input_hash IS NULL OR revocation_input_hash ~ '^[0-9a-f]{64}$'),
    ADD CONSTRAINT chk_model_lane_cloud_consent_receipts_binding_shape_v2
        CHECK (
            (consent_scope = 'single_lane'
             AND lane_id IS NOT NULL
             AND model_session_id IS NOT NULL
             AND provider_kind IS NOT NULL
             AND requested_model_id IS NOT NULL
             AND jsonb_array_length(target_bindings) = 0
             AND target_bindings_hash IS NULL)
            OR
            (consent_scope = 'single_run'
             AND (
                 -- Native v2 run-wide authority has no lane-bound identity.
                 (lane_id IS NULL
                  AND model_session_id IS NULL
                  AND provider_kind IS NULL
                  AND requested_model_id IS NULL)
                 OR
                 -- Preserve pre-0353 v1 SingleRun authority byte-for-byte.
                 -- The current runtime rejects this lane-bound legacy shape
                 -- for launch, so retaining it cannot widen old consent.
                 (lane_id IS NOT NULL
                  AND model_session_id IS NOT NULL
                  AND provider_kind IS NOT NULL
                  AND requested_model_id IS NOT NULL)
             )
             AND jsonb_array_length(target_bindings) = 0
             AND target_bindings_hash IS NULL)
        );

CREATE UNIQUE INDEX uq_model_lane_cloud_projection_plans_single_lane_v2
    ON model_lane_cloud_projection_plans
       (run_id, lane_id, model_session_id, provider_kind)
    WHERE consent_scope = 'single_lane';

CREATE UNIQUE INDEX uq_model_lane_cloud_projection_plans_single_run_v2
    ON model_lane_cloud_projection_plans (run_id)
    WHERE consent_scope = 'single_run'
      AND lane_id IS NULL;

-- Pre-0353 SingleRun rows were lane-bound and used the same uniqueness shape
-- as SingleLane. Keep that historical invariant while excluding those rows
-- from the one-native-run-wide-plan-per-run v2 index above.
CREATE UNIQUE INDEX uq_model_lane_cloud_projection_plans_single_run_legacy_v1
    ON model_lane_cloud_projection_plans
       (run_id, lane_id, model_session_id, provider_kind)
    WHERE consent_scope = 'single_run'
      AND lane_id IS NOT NULL;

CREATE INDEX idx_model_lane_cloud_projection_plans_targets_v2
    ON model_lane_cloud_projection_plans USING GIN (target_bindings);

CREATE INDEX idx_model_lane_cloud_consent_receipts_targets_v2
    ON model_lane_cloud_consent_receipts USING GIN (target_bindings);
