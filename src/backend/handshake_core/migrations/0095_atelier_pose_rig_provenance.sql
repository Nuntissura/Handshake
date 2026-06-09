-- WP-KERNEL-005 MT-087: complete pose rig detector/source provenance.
-- Existing live databases may already have atelier_pose_rig from 0032, so this
-- migration upgrades the row shape in place.

ALTER TABLE atelier_pose_rig
    ADD COLUMN IF NOT EXISTS detector_model TEXT NOT NULL DEFAULT 'unknown',
    ADD COLUMN IF NOT EXISTS detector_model_version TEXT NOT NULL DEFAULT 'unknown',
    ADD COLUMN IF NOT EXISTS source_asset_version_ref TEXT,
    ADD COLUMN IF NOT EXISTS source_asset_path_ref TEXT,
    ADD COLUMN IF NOT EXISTS confidence_available BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE atelier_pose_rig
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_rig_detector_model,
    ADD CONSTRAINT chk_atelier_pose_rig_detector_model CHECK (
        btrim(detector_model) = detector_model
        AND detector_model <> ''
    );

ALTER TABLE atelier_pose_rig
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_rig_detector_model_version,
    ADD CONSTRAINT chk_atelier_pose_rig_detector_model_version CHECK (
        btrim(detector_model_version) = detector_model_version
        AND detector_model_version <> ''
    );
