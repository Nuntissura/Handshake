-- Down-migration for 0342_model_lane_recovery.sql (WP-1 MT-007).

DROP INDEX IF EXISTS idx_model_lane_mt_runtime_statuses_locus;
DROP INDEX IF EXISTS idx_model_lane_mt_runtime_statuses_run;
DROP INDEX IF EXISTS idx_model_lane_mt_runtime_statuses_event_seq;
DROP TABLE IF EXISTS model_lane_mt_runtime_statuses;

DROP INDEX IF EXISTS idx_model_lane_diagnostic_tiers_behavior;
DROP INDEX IF EXISTS idx_model_lane_diagnostic_tiers_event_seq;
DROP TABLE IF EXISTS model_lane_diagnostic_tier_statuses;

DROP INDEX IF EXISTS idx_model_lane_leases_locus;
DROP INDEX IF EXISTS idx_model_lane_leases_run;
DROP INDEX IF EXISTS idx_model_lane_leases_event_seq;
DROP TABLE IF EXISTS model_lane_leases;

DROP INDEX IF EXISTS idx_model_lane_recovery_events_locus;
DROP INDEX IF EXISTS idx_model_lane_recovery_events_run_replay;
DROP INDEX IF EXISTS idx_model_lane_recovery_events_event_seq;
DROP TABLE IF EXISTS model_lane_recovery_events;

DROP INDEX IF EXISTS idx_model_lane_recovery_checkpoints_locus;
DROP INDEX IF EXISTS idx_model_lane_recovery_checkpoints_run;
DROP INDEX IF EXISTS idx_model_lane_recovery_checkpoints_event_seq;
DROP TABLE IF EXISTS model_lane_recovery_checkpoints;

DELETE FROM model_lane_schema_registry
WHERE schema_id IN (
    'hsk.model_lane_recovery_checkpoint@1',
    'hsk.model_lane_recovery_event@1',
    'hsk.model_lane_lease@1',
    'hsk.model_lane_diagnostic_tier@1',
    'hsk.model_lane_mt_runtime_status@1'
);

DROP INDEX IF EXISTS idx_model_lanes_run_lane_unique;
