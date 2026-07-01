-- WP-1 MT-002: allow UserManual seed rows for Dexterity model-lane proof.

ALTER TABLE user_manual_tool_entries
    DROP CONSTRAINT IF EXISTS user_manual_tool_entries_origin_check;

ALTER TABLE user_manual_tool_entries
    ADD CONSTRAINT user_manual_tool_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface', 'wp1_model_lane'));

ALTER TABLE user_manual_feature_entries
    DROP CONSTRAINT IF EXISTS user_manual_feature_entries_origin_check;

ALTER TABLE user_manual_feature_entries
    ADD CONSTRAINT user_manual_feature_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface', 'wp1_model_lane'));
