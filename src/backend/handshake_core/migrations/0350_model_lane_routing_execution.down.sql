DELETE FROM model_lane_schema_registry
WHERE schema_id IN (
    'hsk.model_lane_routing_execution@5',
    'hsk.model_lane_routing_stage_attempt@4',
    'hsk.model_lane_routing_outbox@4',
    'hsk.model_lane_run_extension@1'
);

DROP TABLE IF EXISTS model_lane_routing_outbox;
DROP TABLE IF EXISTS model_lane_routing_stage_attempts;
DROP TABLE IF EXISTS model_lane_routing_executions;
