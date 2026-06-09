-- WP-KERNEL-005 MT-098: complete identity profile records with name,
-- description, source/crop/artifact refs, versioning, and soft-delete state.

ALTER TABLE atelier_identity_profile
    ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS name TEXT NOT NULL DEFAULT 'identity_profile',
    ADD COLUMN IF NOT EXISTS description TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS source_ref TEXT,
    ADD COLUMN IF NOT EXISTS crop_ref TEXT,
    ADD COLUMN IF NOT EXISTS artifact_ref TEXT,
    ADD COLUMN IF NOT EXISTS updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ADD COLUMN IF NOT EXISTS deleted_at_utc TIMESTAMPTZ;

ALTER TABLE atelier_identity_profile
    DROP CONSTRAINT IF EXISTS chk_atelier_identity_profile_version,
    ADD CONSTRAINT chk_atelier_identity_profile_version CHECK (version >= 1);

CREATE INDEX IF NOT EXISTS idx_atelier_identity_profile_active
    ON atelier_identity_profile(character_internal_id, kind, seq DESC)
    WHERE deleted_at_utc IS NULL;
