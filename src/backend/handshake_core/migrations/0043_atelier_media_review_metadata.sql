-- WP-KERNEL-005 MT-018 media review metadata.
-- Review flags/status/rating are stored in PostgreSQL and mutated through
-- EventLedger-backed bulk operations; media bytes remain in ArtifactStore.

CREATE TABLE IF NOT EXISTS atelier_media_review_metadata (
    asset_id        UUID PRIMARY KEY REFERENCES atelier_media_asset(asset_id) ON DELETE CASCADE,
    favorite        BOOLEAN NOT NULL DEFAULT FALSE,
    rating          SMALLINT NOT NULL DEFAULT 0 CHECK (rating BETWEEN 0 AND 5),
    frontpage       BOOLEAN NOT NULL DEFAULT FALSE,
    carousel        BOOLEAN NOT NULL DEFAULT FALSE,
    notes           TEXT,
    review_status   TEXT NOT NULL DEFAULT 'unreviewed'
                    CHECK (review_status IN ('unreviewed', 'review', 'approved', 'rejected', 'deferred')),
    updated_by      TEXT NOT NULL,
    updated_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_atelier_media_review_metadata_status
    ON atelier_media_review_metadata(review_status, updated_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_atelier_media_review_metadata_flags
    ON atelier_media_review_metadata(favorite, frontpage, carousel);
