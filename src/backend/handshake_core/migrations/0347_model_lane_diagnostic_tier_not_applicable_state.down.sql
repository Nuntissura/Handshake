ALTER TABLE model_lane_diagnostic_tier_statuses
    DROP CONSTRAINT IF EXISTS chk_model_lane_diag_state;

UPDATE model_lane_diagnostic_tier_statuses
SET state = 'deferred_with_reason',
    follow_up_ref = COALESCE(follow_up_ref, 'hbr-int-009://not-applicable-state-downgrade'),
    record_json = jsonb_set(
        jsonb_set(record_json, '{state}', '"deferred_with_reason"'::jsonb, false),
        '{follow_up_ref}',
        to_jsonb(COALESCE(follow_up_ref, 'hbr-int-009://not-applicable-state-downgrade')),
        false
    )
WHERE state = 'not_applicable_with_reason';

ALTER TABLE model_lane_diagnostic_tier_statuses
    ADD CONSTRAINT chk_model_lane_diag_state
        CHECK (state IN ('wired','deferred_with_reason','missing'));
