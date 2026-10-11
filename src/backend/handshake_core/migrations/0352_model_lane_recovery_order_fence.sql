-- AC-10: recovery-event ordering is allocated under the per-run advisory fence.
-- Version 2 records the stronger write contract while retaining @1 for history.
INSERT INTO model_lane_schema_registry
    (schema_id, schema_version, record_kind, table_name)
VALUES
    ('hsk.model_lane_recovery_event@2', 2, 'ModelLaneRecoveryEvent', 'model_lane_recovery_events')
ON CONFLICT (schema_id) DO UPDATE SET
    schema_version = EXCLUDED.schema_version,
    record_kind = EXCLUDED.record_kind,
    table_name = EXCLUDED.table_name,
    source_component = EXCLUDED.source_component;

ALTER TABLE model_lane_recovery_events
    DROP CONSTRAINT IF EXISTS uq_model_lane_recovery_events_replay_order;

ALTER TABLE model_lane_recovery_events
    ADD CONSTRAINT uq_model_lane_recovery_events_run_replay_order
    UNIQUE (run_id, replay_order_seq);
