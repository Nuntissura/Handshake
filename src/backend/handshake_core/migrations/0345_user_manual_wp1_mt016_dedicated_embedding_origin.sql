-- WP-1 MT-016: UserManual seed rows for the dedicated embedding model routing
-- surface use a distinct origin. Keep origin constraints explicit so seeded
-- tool/feature rows stay auditable by work packet.

ALTER TABLE user_manual_tool_entries
    DROP CONSTRAINT IF EXISTS user_manual_tool_entries_origin_check;

ALTER TABLE user_manual_tool_entries
    ADD CONSTRAINT user_manual_tool_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface', 'wp1_model_lane', 'wp1_mt014', 'wp1_mt013_embedded_model', 'wp1_mt012_operator_chat', 'wp1_mt016_dedicated_embedding_model'));

ALTER TABLE user_manual_feature_entries
    DROP CONSTRAINT IF EXISTS user_manual_feature_entries_origin_check;

ALTER TABLE user_manual_feature_entries
    ADD CONSTRAINT user_manual_feature_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface', 'wp1_model_lane', 'wp1_mt014', 'wp1_mt013_embedded_model', 'wp1_mt012_operator_chat', 'wp1_mt016_dedicated_embedding_model'));
