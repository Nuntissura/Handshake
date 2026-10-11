-- WP-1 MT-012: allow UserManual seed rows for the operator chat/launch surface
-- capture proof suite.
--
-- Re-creating this CHECK constraint requires listing every origin the seed
-- corpus currently uses, or the DROP would silently strip a still-valid origin.
-- Carries forward every previously-admitted origin and adds
-- 'wp1_mt012_operator_chat' for the MT-012 operator chat/launch tool entry.

ALTER TABLE user_manual_tool_entries
    DROP CONSTRAINT IF EXISTS user_manual_tool_entries_origin_check;

ALTER TABLE user_manual_tool_entries
    ADD CONSTRAINT user_manual_tool_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface', 'wp1_model_lane', 'wp1_mt014', 'wp1_mt013_embedded_model', 'wp1_mt012_operator_chat'));

ALTER TABLE user_manual_feature_entries
    DROP CONSTRAINT IF EXISTS user_manual_feature_entries_origin_check;

ALTER TABLE user_manual_feature_entries
    ADD CONSTRAINT user_manual_feature_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface', 'wp1_model_lane', 'wp1_mt014', 'wp1_mt013_embedded_model', 'wp1_mt012_operator_chat'));
