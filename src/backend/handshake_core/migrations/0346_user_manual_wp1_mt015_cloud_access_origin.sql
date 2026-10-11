-- Admit the MT-015 Cloud Model Access UserManual tool-entry origin.
-- Keep origin constraints explicit so seeded authority rows cannot silently
-- invent new provenance labels.

ALTER TABLE user_manual_tool_entries
    DROP CONSTRAINT IF EXISTS user_manual_tool_entries_origin_check;

ALTER TABLE user_manual_tool_entries
    ADD CONSTRAINT user_manual_tool_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface', 'wp1_model_lane', 'wp1_mt014', 'wp1_mt013_embedded_model', 'wp1_mt012_operator_chat', 'wp1_mt016_dedicated_embedding_model', 'wp1_mt015_cloud_model_access'));

ALTER TABLE user_manual_feature_entries
    DROP CONSTRAINT IF EXISTS user_manual_feature_entries_origin_check;

ALTER TABLE user_manual_feature_entries
    ADD CONSTRAINT user_manual_feature_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface', 'wp1_model_lane', 'wp1_mt014', 'wp1_mt013_embedded_model', 'wp1_mt012_operator_chat', 'wp1_mt016_dedicated_embedding_model', 'wp1_mt015_cloud_model_access'));
