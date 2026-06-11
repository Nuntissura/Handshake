-- WP-KERNEL-005 MT-090: preserve typed pose calibration data.
-- Existing calibration rows keep their state/block reason and receive empty
-- typed defaults until a Workflow-Engine calibration job writes real data.

ALTER TABLE atelier_pose_calibration
    ADD COLUMN IF NOT EXISTS head_pose_ref TEXT,
    ADD COLUMN IF NOT EXISTS marker_visibility JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS marker_colors JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS hand_rows JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS history_refs JSONB NOT NULL DEFAULT '[]'::jsonb;

ALTER TABLE atelier_pose_calibration
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_calibration_marker_visibility_json,
    ADD CONSTRAINT chk_atelier_pose_calibration_marker_visibility_json CHECK (jsonb_typeof(marker_visibility) = 'object');

ALTER TABLE atelier_pose_calibration
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_calibration_marker_colors_json,
    ADD CONSTRAINT chk_atelier_pose_calibration_marker_colors_json CHECK (jsonb_typeof(marker_colors) = 'object');

ALTER TABLE atelier_pose_calibration
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_calibration_hand_rows_json,
    ADD CONSTRAINT chk_atelier_pose_calibration_hand_rows_json CHECK (jsonb_typeof(hand_rows) = 'array');

ALTER TABLE atelier_pose_calibration
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_calibration_history_refs_json,
    ADD CONSTRAINT chk_atelier_pose_calibration_history_refs_json CHECK (jsonb_typeof(history_refs) = 'array');
