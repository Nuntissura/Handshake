-- WP-KERNEL-005 MT-088: persist fallback/failed detector error reasons.
-- Existing databases may have pose rigs from 0032/0095; upgrade in place.

ALTER TABLE atelier_pose_rig
    ADD COLUMN IF NOT EXISTS error_reason TEXT;

UPDATE atelier_pose_rig
SET error_reason = 'legacy detector status without recorded reason'
WHERE detector_status IN ('fallback','failed')
  AND (error_reason IS NULL OR btrim(error_reason) = '');

UPDATE atelier_pose_rig
SET error_reason = NULL
WHERE detector_status = 'detected';

ALTER TABLE atelier_pose_rig
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_rig_status_error,
    ADD CONSTRAINT chk_atelier_pose_rig_status_error CHECK (
        (detector_status = 'detected' AND error_reason IS NULL)
        OR (
            detector_status IN ('fallback','failed')
            AND error_reason IS NOT NULL
            AND btrim(error_reason) = error_reason
            AND error_reason <> ''
        )
    );
