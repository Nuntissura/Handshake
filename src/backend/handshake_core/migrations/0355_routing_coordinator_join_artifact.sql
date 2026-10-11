ALTER TABLE model_lane_routing_stage_attempts
    DROP CONSTRAINT model_lane_routing_stage_attempts_check1;

ALTER TABLE model_lane_routing_stage_attempts
    ADD CONSTRAINT model_lane_routing_stage_attempts_output_lineage_check
    CHECK (
        (output_ref IS NULL AND output_message_ref IS NULL)
        OR (
            output_ref IS NOT NULL
            AND (
                (dispatch_target = 'coordinator_join' AND output_message_ref IS NULL)
                OR (dispatch_target <> 'coordinator_join' AND output_message_ref IS NOT NULL)
            )
        )
    );
