-- WP-KERNEL-005 MT-092: typed pose sidecar artifact registry.
-- Records OpenPose JSON, OpenPose PNG previews, and conditioning PNG sidecars
-- as portable ArtifactStore refs. No local filesystem or .GOV refs.

CREATE TABLE IF NOT EXISTS atelier_pose_sidecar (
    sidecar_id     UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rig_id         UUID NOT NULL REFERENCES atelier_pose_rig(rig_id) ON DELETE CASCADE,
    source_asset_id UUID REFERENCES atelier_media_asset(asset_id) ON DELETE SET NULL,
    source_ref     TEXT NOT NULL DEFAULT '',
    kind           TEXT NOT NULL CHECK (kind IN ('openpose_json','openpose_png','conditioning_png')),
    role           TEXT NOT NULL,
    artifact_ref   TEXT NOT NULL,
    manifest_ref   TEXT NOT NULL,
    content_hash   TEXT NOT NULL,
    byte_len       BIGINT NOT NULL CHECK (byte_len > 0),
    mime           TEXT NOT NULL,
    width          INTEGER NOT NULL DEFAULT 0 CHECK (width > 0),
    height         INTEGER NOT NULL DEFAULT 0 CHECK (height > 0),
    status         TEXT NOT NULL DEFAULT 'rendered' CHECK (status IN ('rendered','failed')),
    error_message  TEXT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (rig_id, kind),
    CONSTRAINT chk_atelier_pose_sidecar_source_ref CHECK (
        btrim(source_ref) = source_ref
        AND source_ref <> ''
        AND source_ref NOT ILIKE '%.GOV%'
        AND source_ref !~ '\s'
        AND source_ref !~ '^[A-Za-z]:'
        AND source_ref NOT LIKE 'file:%'
    ),
    CONSTRAINT chk_atelier_pose_sidecar_artifact_ref CHECK (
        artifact_ref LIKE 'artifact://.handshake/artifacts/%'
        AND artifact_ref LIKE '%/payload'
        AND artifact_ref NOT ILIKE '%.GOV%'
        AND artifact_ref !~ '\\'
        AND artifact_ref !~ '\s'
        AND artifact_ref !~ '^[A-Za-z]:'
        AND artifact_ref NOT LIKE 'file:%'
    ),
    CONSTRAINT chk_atelier_pose_sidecar_manifest_ref CHECK (
        manifest_ref LIKE 'artifact://.handshake/artifacts/%/artifact.json'
        AND manifest_ref NOT ILIKE '%.GOV%'
        AND manifest_ref !~ '\\'
        AND manifest_ref !~ '\s'
        AND manifest_ref !~ '^[A-Za-z]:'
        AND manifest_ref NOT LIKE 'file:%'
    ),
    CONSTRAINT chk_atelier_pose_sidecar_role_kind CHECK (role = kind),
    CONSTRAINT chk_atelier_pose_sidecar_content_hash CHECK (
        btrim(content_hash) = content_hash
        AND content_hash <> ''
        AND content_hash !~ '\s'
    ),
    CONSTRAINT chk_atelier_pose_sidecar_mime CHECK (
        (kind = 'openpose_json' AND mime = 'application/json')
        OR (kind IN ('openpose_png','conditioning_png') AND mime = 'image/png')
    ),
    CONSTRAINT chk_atelier_pose_sidecar_status_error CHECK (
        (status = 'rendered' AND error_message IS NULL)
        OR (status = 'failed' AND error_message IS NOT NULL AND btrim(error_message) <> '')
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_pose_sidecar_rig
    ON atelier_pose_sidecar(rig_id, created_at_utc);
