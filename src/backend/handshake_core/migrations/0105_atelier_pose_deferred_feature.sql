-- WP-KERNEL-005 MT-115 / MT-116 / MT-117: typed pose deferred-feature registry.
--
-- Records CKC WP-0133 pose-workspace items and Pose tab polish features that are
-- intentionally PLANNED / DEFERRED / BLOCKED, with a mandatory machine-readable
-- deferral_reason. This is the typed runtime surface that prevents false parity
-- claims: a deferred/blocked pose feature is a real Postgres row + EventLedger
-- event, never governance markdown. No local filesystem, no .GOV refs, no SQLite.

CREATE TABLE IF NOT EXISTS atelier_pose_deferred_feature (
    feature_id      TEXT PRIMARY KEY,
    feature_kind    TEXT NOT NULL,
    status          TEXT NOT NULL CHECK (status IN ('PLANNED','DEFERRED','BLOCKED')),
    feature_label   TEXT NOT NULL,
    deferral_reason TEXT NOT NULL,
    carry_forward   BOOLEAN NOT NULL DEFAULT FALSE,
    source_ref      TEXT,
    created_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_pose_deferred_feature_id CHECK (
        btrim(feature_id) = feature_id
        AND feature_id <> ''
    ),
    CONSTRAINT chk_atelier_pose_deferred_feature_kind CHECK (
        btrim(feature_kind) = feature_kind
        AND feature_kind <> ''
    ),
    CONSTRAINT chk_atelier_pose_deferred_feature_label CHECK (
        btrim(feature_label) = feature_label
        AND feature_label <> ''
    ),
    CONSTRAINT chk_atelier_pose_deferred_feature_reason CHECK (
        btrim(deferral_reason) = deferral_reason
        AND deferral_reason <> ''
    ),
    CONSTRAINT chk_atelier_pose_deferred_feature_source_ref CHECK (
        source_ref IS NULL
        OR (
            btrim(source_ref) = source_ref
            AND source_ref <> ''
            AND source_ref NOT ILIKE '%.GOV%'
            AND source_ref !~ '\s'
            AND source_ref !~ '^[A-Za-z]:'
            AND source_ref NOT LIKE 'file:%'
        )
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_pose_deferred_feature_kind
    ON atelier_pose_deferred_feature(feature_kind, status, created_at_utc);
