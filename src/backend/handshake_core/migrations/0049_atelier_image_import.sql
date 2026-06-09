-- WP-KERNEL-005 MT-025 clipboard and URL image import.
-- Clipboard import materializes already-captured ArtifactStore payloads. URL
-- import records a governed fetch request after SSRF/capability preflight; the
-- repository layer does not open sockets.

CREATE TABLE IF NOT EXISTS atelier_image_import_request (
    import_id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    idempotency_key        TEXT NOT NULL UNIQUE,
    source_kind            TEXT NOT NULL CHECK (source_kind IN ('clipboard', 'url')),
    status                 TEXT NOT NULL CHECK (status IN ('materialized', 'queued')),
    requested_by           TEXT NOT NULL,
    normalized_url         TEXT,
    source_url_hash        TEXT,
    source_host            TEXT,
    source_label           TEXT,
    expected_mime          TEXT,
    capability_profile_id  TEXT,
    capability_grant_ref   TEXT,
    required_capabilities  JSONB NOT NULL DEFAULT '[]'::jsonb,
    asset_id               UUID REFERENCES atelier_media_asset(asset_id),
    artifact_ref           TEXT,
    source_provenance      TEXT NOT NULL,
    preflight              JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at_utc         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (
        (source_kind = 'clipboard' AND asset_id IS NOT NULL AND artifact_ref IS NOT NULL)
        OR
        (
            source_kind = 'url'
            AND normalized_url IS NOT NULL
            AND source_url_hash IS NOT NULL
            AND source_host IS NOT NULL
            AND capability_profile_id IS NOT NULL
            AND capability_grant_ref IS NOT NULL
        )
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_image_import_source
    ON atelier_image_import_request(source_kind, created_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_atelier_image_import_url_hash
    ON atelier_image_import_request(source_url_hash);

CREATE INDEX IF NOT EXISTS idx_atelier_image_import_asset
    ON atelier_image_import_request(asset_id);
