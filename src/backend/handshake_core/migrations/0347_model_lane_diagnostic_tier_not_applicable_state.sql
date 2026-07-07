ALTER TABLE model_lane_diagnostic_tier_statuses
    DROP CONSTRAINT IF EXISTS chk_model_lane_diag_state;

ALTER TABLE model_lane_diagnostic_tier_statuses
    ADD CONSTRAINT chk_model_lane_diag_state
        CHECK (state IN ('wired','not_applicable_with_reason','deferred_with_reason','missing'));
