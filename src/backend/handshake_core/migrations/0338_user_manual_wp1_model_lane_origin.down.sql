-- Down-migration for 0338_user_manual_wp1_model_lane_origin.sql.

DELETE FROM user_manual_feature_entries WHERE origin = 'wp1_model_lane';
DELETE FROM user_manual_tool_entries WHERE origin = 'wp1_model_lane';

ALTER TABLE user_manual_feature_entries
    DROP CONSTRAINT IF EXISTS user_manual_feature_entries_origin_check;

ALTER TABLE user_manual_feature_entries
    ADD CONSTRAINT user_manual_feature_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface'));

ALTER TABLE user_manual_tool_entries
    DROP CONSTRAINT IF EXISTS user_manual_tool_entries_origin_check;

ALTER TABLE user_manual_tool_entries
    ADD CONSTRAINT user_manual_tool_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface'));
