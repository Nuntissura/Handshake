ALTER TABLE model_lane_recovery_events
    DROP CONSTRAINT IF EXISTS uq_model_lane_recovery_events_run_replay_order;

ALTER TABLE model_lane_recovery_events
    ADD CONSTRAINT uq_model_lane_recovery_events_replay_order
    UNIQUE (run_id, replay_order_seq);

DELETE FROM model_lane_schema_registry
WHERE schema_id = 'hsk.model_lane_recovery_event@2';
