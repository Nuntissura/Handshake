-- Down-migration for 0343_user_manual_wp1_mt013_embedded_model_origin.sql.
-- Removes only the MT-013 origin; 'wp1_mt014' predates this migration and is
-- kept so the reversal does not fail against existing MT-014 seed rows.

DELETE FROM user_manual_feature_entries WHERE origin = 'wp1_mt013_embedded_model';
DELETE FROM user_manual_tool_entries WHERE origin = 'wp1_mt013_embedded_model';

ALTER TABLE user_manual_feature_entries
    DROP CONSTRAINT IF EXISTS user_manual_feature_entries_origin_check;

ALTER TABLE user_manual_feature_entries
    ADD CONSTRAINT user_manual_feature_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface', 'wp1_model_lane', 'wp1_mt014'));

ALTER TABLE user_manual_tool_entries
    DROP CONSTRAINT IF EXISTS user_manual_tool_entries_origin_check;

ALTER TABLE user_manual_tool_entries
    ADD CONSTRAINT user_manual_tool_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface', 'wp1_model_lane', 'wp1_mt014'));
