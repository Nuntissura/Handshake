-- WP-KERNEL-005 MT-017 media derivative skeleton.
-- Thumbnail/proxy/Photo Studio skeleton work is tracked as durable derivative
-- state with retry/error evidence; bytes stay in ArtifactStore.

CREATE TABLE IF NOT EXISTS atelier_media_derivative (
    derivative_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_id              UUID NOT NULL REFERENCES atelier_media_asset(asset_id) ON DELETE CASCADE,
    derivative_kind       TEXT NOT NULL CHECK (derivative_kind IN ('thumbnail', 'proxy', 'photo_studio_skeleton')),
    target_width          INTEGER NOT NULL CHECK (target_width > 0 AND target_width <= 16384),
    target_height         INTEGER NOT NULL CHECK (target_height > 0 AND target_height <= 16384),
    format                TEXT NOT NULL CHECK (format IN ('png', 'jpeg')),
    status                TEXT NOT NULL DEFAULT 'pending'
                          CHECK (status IN ('pending', 'generating', 'generated', 'retryable_error', 'failed')),
    artifact_ref          TEXT,
    artifact_manifest_ref TEXT,
    mime                  TEXT,
    byte_len              BIGINT CHECK (byte_len IS NULL OR byte_len > 0),
    requested_by          TEXT NOT NULL,
    updated_by            TEXT NOT NULL,
    attempt_count         BIGINT NOT NULL DEFAULT 0,
    retry_count           BIGINT NOT NULL DEFAULT 0,
    last_error_code       TEXT,
    last_error_ref        TEXT,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (asset_id, derivative_kind, target_width, target_height, format)
);

CREATE INDEX IF NOT EXISTS idx_atelier_media_derivative_asset
    ON atelier_media_derivative(asset_id, derivative_kind, target_width, target_height);

CREATE INDEX IF NOT EXISTS idx_atelier_media_derivative_status
    ON atelier_media_derivative(status, updated_at_utc DESC);
