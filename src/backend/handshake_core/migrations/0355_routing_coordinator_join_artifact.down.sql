ALTER TABLE model_lane_routing_stage_attempts
    DROP CONSTRAINT model_lane_routing_stage_attempts_output_lineage_check;

ALTER TABLE model_lane_routing_stage_attempts
    ADD CONSTRAINT model_lane_routing_stage_attempts_check1
    CHECK ((output_ref IS NULL) = (output_message_ref IS NULL));
