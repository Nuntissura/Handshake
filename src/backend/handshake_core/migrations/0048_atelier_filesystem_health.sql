-- WP-KERNEL-005 MT-023 filesystem health diagnostics.
-- Checks are read-only snapshots over governed atelier state. They record
-- findings but never resync, repair, or delete source rows.

CREATE TABLE IF NOT EXISTS atelier_filesystem_health_check (
    check_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    requested_by    TEXT NOT NULL,
    scope_label     TEXT,
    summary         JSONB NOT NULL,
    created_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS atelier_filesystem_health_finding (
    finding_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    check_id        UUID NOT NULL REFERENCES atelier_filesystem_health_check(check_id) ON DELETE CASCADE,
    finding_kind    TEXT NOT NULL CHECK (
        finding_kind IN (
            'missing_original',
            'missing_thumbnail',
            'inbox_pending',
            'untracked_original',
            'sidecar_visibility_anomaly'
        )
    ),
    target_type     TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    details         JSONB NOT NULL,
    created_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_atelier_filesystem_health_finding_check
    ON atelier_filesystem_health_finding(check_id, finding_kind);

CREATE INDEX IF NOT EXISTS idx_atelier_filesystem_health_finding_target
    ON atelier_filesystem_health_finding(target_type, target_id);
