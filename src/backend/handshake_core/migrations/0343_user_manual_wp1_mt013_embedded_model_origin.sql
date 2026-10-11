-- WP-1 MT-013 (AC#5): allow UserManual seed rows for the embedded-model
-- lifecycle ledger + fail-closed/embedding Flight Recorder proof suites.
--
-- Re-creating this CHECK constraint requires listing every origin the seed
-- corpus currently uses, or the DROP would silently strip a still-valid origin.
-- That set already includes 'wp1_mt014' (used by the MT-014 model-catalog seed
-- rows), for which no migration was ever landed — so this migration also
-- re-admits it. Adds 'wp1_mt013_embedded_model' for the MT-013 rows.

ALTER TABLE user_manual_tool_entries
    DROP CONSTRAINT IF EXISTS user_manual_tool_entries_origin_check;

ALTER TABLE user_manual_tool_entries
    ADD CONSTRAINT user_manual_tool_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface', 'wp1_model_lane', 'wp1_mt014', 'wp1_mt013_embedded_model'));

ALTER TABLE user_manual_feature_entries
    DROP CONSTRAINT IF EXISTS user_manual_feature_entries_origin_check;

ALTER TABLE user_manual_feature_entries
    ADD CONSTRAINT user_manual_feature_entries_origin_check
    CHECK (origin IN ('legacy_model_manual', 'wp009_surface', 'wp1_model_lane', 'wp1_mt014', 'wp1_mt013_embedded_model'));
