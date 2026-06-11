-- WP-KERNEL-005 MT-091: complete OpenPose sidecar artifact metadata.
-- Adds role and ArtifactStore manifest linkage to existing typed pose sidecars.

ALTER TABLE atelier_pose_sidecar
    ADD COLUMN IF NOT EXISTS role TEXT,
    ADD COLUMN IF NOT EXISTS manifest_ref TEXT;

UPDATE atelier_pose_sidecar
SET role = kind
WHERE role IS NULL OR btrim(role) = '';

UPDATE atelier_pose_sidecar
SET manifest_ref = CASE
    WHEN artifact_ref LIKE '%/payload' THEN regexp_replace(artifact_ref, '/payload$', '/artifact.json')
    ELSE artifact_ref || '/artifact.json'
END
WHERE manifest_ref IS NULL OR btrim(manifest_ref) = '';

ALTER TABLE atelier_pose_sidecar
    ALTER COLUMN role SET NOT NULL,
    ALTER COLUMN manifest_ref SET NOT NULL;

ALTER TABLE atelier_pose_sidecar
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_sidecar_artifact_ref,
    ADD CONSTRAINT chk_atelier_pose_sidecar_artifact_ref CHECK (
        artifact_ref LIKE 'artifact://.handshake/artifacts/%'
        AND artifact_ref LIKE '%/payload'
        AND artifact_ref NOT ILIKE '%.GOV%'
        AND artifact_ref !~ '\\'
        AND artifact_ref !~ '\s'
        AND artifact_ref !~ '^[A-Za-z]:'
        AND artifact_ref NOT LIKE 'file:%'
    );

ALTER TABLE atelier_pose_sidecar
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_sidecar_manifest_ref,
    ADD CONSTRAINT chk_atelier_pose_sidecar_manifest_ref CHECK (
        manifest_ref LIKE 'artifact://.handshake/artifacts/%/artifact.json'
        AND manifest_ref NOT ILIKE '%.GOV%'
        AND manifest_ref !~ '\\'
        AND manifest_ref !~ '\s'
        AND manifest_ref !~ '^[A-Za-z]:'
        AND manifest_ref NOT LIKE 'file:%'
    );

ALTER TABLE atelier_pose_sidecar
    DROP CONSTRAINT IF EXISTS chk_atelier_pose_sidecar_role_kind,
    ADD CONSTRAINT chk_atelier_pose_sidecar_role_kind CHECK (role = kind);
