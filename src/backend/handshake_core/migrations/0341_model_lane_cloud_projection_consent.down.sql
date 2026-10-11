-- Down-migration for 0341_model_lane_cloud_projection_consent.sql (WP-1 MT-006).

DROP INDEX IF EXISTS idx_model_lane_cloud_consent_receipts_record_gin;
DROP INDEX IF EXISTS idx_model_lane_cloud_consent_receipts_locus;
DROP INDEX IF EXISTS idx_model_lane_cloud_consent_receipts_projection;
DROP INDEX IF EXISTS idx_model_lane_cloud_consent_receipts_run_replay;
DROP INDEX IF EXISTS idx_model_lane_cloud_consent_receipts_stream_replay;
DROP INDEX IF EXISTS idx_model_lane_cloud_consent_receipts_event_seq;
DROP TABLE IF EXISTS model_lane_cloud_consent_receipts;

DROP INDEX IF EXISTS idx_model_lane_cloud_projection_plans_record_gin;
DROP INDEX IF EXISTS idx_model_lane_cloud_projection_plans_locus;
DROP INDEX IF EXISTS idx_model_lane_cloud_projection_plans_run_replay;
DROP INDEX IF EXISTS idx_model_lane_cloud_projection_plans_stream_replay;
DROP INDEX IF EXISTS idx_model_lane_cloud_projection_plans_event_seq;
DROP TABLE IF EXISTS model_lane_cloud_projection_plans;

DELETE FROM model_lane_schema_registry
WHERE schema_id IN (
    'hsk.model_lane_cloud_projection_plan@1',
    'hsk.model_lane_cloud_consent_receipt@1',
    'hsk.model_lane_cloud_consent_denial@1'
);
