-- WP-KERNEL-005 MT-099: identity crop artifacts linked to identity profile
-- versions with native ArtifactStore refs, hash, crop geometry, landmarks,
-- and a durable manifest.

CREATE TABLE IF NOT EXISTS atelier_identity_crop_artifact (
    crop_id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    profile_id            UUID NOT NULL REFERENCES atelier_identity_profile(profile_id) ON DELETE CASCADE,
    profile_version       BIGINT NOT NULL,
    character_internal_id UUID NOT NULL REFERENCES atelier_character(internal_id) ON DELETE CASCADE,
    source_ref            TEXT NOT NULL,
    crop_box              JSONB NOT NULL,
    landmarks             JSONB NOT NULL DEFAULT '[]'::jsonb,
    artifact_ref          TEXT NOT NULL,
    manifest_ref          TEXT NOT NULL,
    content_hash          TEXT NOT NULL,
    byte_len              BIGINT NOT NULL,
    mime                  TEXT NOT NULL,
    width                 INTEGER NOT NULL,
    height                INTEGER NOT NULL,
    manifest              JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_by            TEXT NOT NULL,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_atelier_identity_crop_artifact_profile_version_hash UNIQUE (profile_id, profile_version, content_hash),
    CONSTRAINT chk_atelier_identity_crop_artifact_profile_version CHECK (profile_version >= 1),
    CONSTRAINT chk_atelier_identity_crop_artifact_byte_len CHECK (byte_len > 0),
    CONSTRAINT chk_atelier_identity_crop_artifact_mime CHECK (mime = 'image/png'),
    CONSTRAINT chk_atelier_identity_crop_artifact_size CHECK (width = 512 AND height = 512),
    CONSTRAINT chk_atelier_identity_crop_artifact_crop_box_json CHECK (jsonb_typeof(crop_box) = 'object'),
    CONSTRAINT chk_atelier_identity_crop_artifact_landmarks_json CHECK (jsonb_typeof(landmarks) = 'array'),
    CONSTRAINT chk_atelier_identity_crop_artifact_manifest_json CHECK (jsonb_typeof(manifest) = 'object')
);

CREATE INDEX IF NOT EXISTS idx_atelier_identity_crop_artifact_profile
    ON atelier_identity_crop_artifact(profile_id, profile_version, created_at_utc);
