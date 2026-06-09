-- WP-KERNEL-005 MT-092 upgrade: source, dimensions, status/error contract
-- for pose sidecars already created by 0090 in local worktrees.

ALTER TABLE atelier_pose_sidecar
    ADD COLUMN IF NOT EXISTS source_asset_id UUID REFERENCES atelier_media_asset(asset_id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS source_ref TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS width INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS height INTEGER NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'rendered',
    ADD COLUMN IF NOT EXISTS error_message TEXT;

UPDATE atelier_pose_sidecar sidecar
SET source_asset_id = rig.source_asset_id,
    source_ref = rig.source_ref,
    width = GREATEST(rig.canvas_width, 1),
    height = GREATEST(rig.canvas_height, 1)
FROM atelier_pose_rig rig
WHERE sidecar.rig_id = rig.rig_id
  AND sidecar.source_ref = '';

ALTER TABLE atelier_pose_sidecar
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_sidecar_source_ref,
    ADD CONSTRAINT chk_atelier_pose_sidecar_source_ref CHECK (
        btrim(source_ref) = source_ref
        AND source_ref <> ''
        AND source_ref NOT ILIKE '%.GOV%'
        AND source_ref !~ '\s'
        AND source_ref !~ '^[A-Za-z]:'
        AND source_ref NOT LIKE 'file:%'
    );

ALTER TABLE atelier_pose_sidecar
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_sidecar_dimensions,
    ADD CONSTRAINT chk_atelier_pose_sidecar_dimensions CHECK (width > 0 AND height > 0);

ALTER TABLE atelier_pose_sidecar
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_sidecar_status,
    ADD CONSTRAINT chk_atelier_pose_sidecar_status CHECK (status IN ('rendered','failed'));

ALTER TABLE atelier_pose_sidecar
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_sidecar_status_error,
    ADD CONSTRAINT chk_atelier_pose_sidecar_status_error CHECK (
        (status = 'rendered' AND error_message IS NULL)
        OR (status = 'failed' AND error_message IS NOT NULL AND btrim(error_message) <> '')
    );
