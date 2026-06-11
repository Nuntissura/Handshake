-- WP-KERNEL-005 MT-020: similarity projection rebuild jobs.
-- PostgreSQL authority only; image bytes stay in ArtifactStore/callers.

CREATE TABLE IF NOT EXISTS atelier_similarity_rebuild_job (
    job_id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_internal_id UUID NOT NULL REFERENCES atelier_media_asset(asset_id) ON DELETE CASCADE,
    status            TEXT NOT NULL CHECK (status IN ('running','completed','failed')),
    requested_by      TEXT NOT NULL,
    processed_count   BIGINT NOT NULL DEFAULT 0,
    failed_count      BIGINT NOT NULL DEFAULT 0,
    dhash_hex         TEXT,
    palette_json      JSONB,
    error_ref         TEXT,
    created_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_atelier_similarity_rebuild_job_asset
    ON atelier_similarity_rebuild_job(asset_internal_id, updated_at_utc DESC);
CREATE INDEX IF NOT EXISTS idx_atelier_similarity_rebuild_job_status
    ON atelier_similarity_rebuild_job(status, updated_at_utc DESC);
