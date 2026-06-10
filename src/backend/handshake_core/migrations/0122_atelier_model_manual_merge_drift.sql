-- WP-KERNEL-005 MT-183 / MT-185 / MT-186 / MT-187 ModelManual merge + drift guard.
-- Persists executed manual source-row merge runs (Core/Data, Pose/ComfyUI, and
-- Diagnostics-owned rows merged by normalized id with missing rows marked as
-- blockers) and manual drift-guard runs (wired-surface resolution, orphan rows,
-- normalization collisions, and HBR-MAN-001 version-bump checks across runs).

CREATE TABLE IF NOT EXISTS atelier_model_manual_row_merge (
    run_id            UUID PRIMARY KEY,
    source_kind       TEXT NOT NULL,
    manual_version    TEXT NOT NULL,
    merged_row_count  INTEGER NOT NULL,
    blocker_count     INTEGER NOT NULL,
    merged_rows       JSONB NOT NULL,
    blockers          JSONB NOT NULL,
    created_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_mm_row_merge_source_kind
        CHECK (source_kind IN ('core_data', 'pose_comfy', 'diagnostics_owned')),
    CONSTRAINT chk_atelier_mm_row_merge_manual_version
        CHECK (manual_version = btrim(manual_version) AND manual_version <> ''),
    CONSTRAINT chk_atelier_mm_row_merge_merged_rows_array
        CHECK (jsonb_typeof(merged_rows) = 'array'),
    CONSTRAINT chk_atelier_mm_row_merge_blockers_array
        CHECK (jsonb_typeof(blockers) = 'array'),
    CONSTRAINT chk_atelier_mm_row_merge_counts
        CHECK (
            merged_row_count >= 0
            AND blocker_count >= 0
            AND merged_row_count = jsonb_array_length(merged_rows)
            AND blocker_count = jsonb_array_length(blockers)
        ),
    -- A merge that produced neither merged rows nor blockers did not execute.
    CONSTRAINT chk_atelier_mm_row_merge_nonempty
        CHECK (merged_row_count + blocker_count > 0)
);

CREATE INDEX IF NOT EXISTS idx_atelier_mm_row_merge_kind
    ON atelier_model_manual_row_merge(source_kind, created_at_utc DESC);

CREATE TABLE IF NOT EXISTS atelier_model_manual_drift_guard (
    run_id                UUID PRIMARY KEY,
    guard_scope           TEXT NOT NULL,
    manual_version        TEXT NOT NULL,
    wired_surface_sha256  TEXT NOT NULL,
    wired_surface_changed BOOLEAN NOT NULL,
    finding_count         INTEGER NOT NULL,
    findings              JSONB NOT NULL,
    created_at_utc        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_mm_drift_guard_scope
        CHECK (guard_scope = btrim(guard_scope) AND guard_scope <> ''),
    CONSTRAINT chk_atelier_mm_drift_guard_manual_version
        CHECK (manual_version = btrim(manual_version) AND manual_version <> ''),
    CONSTRAINT chk_atelier_mm_drift_guard_sha256
        CHECK (wired_surface_sha256 ~ '^sha256:[0-9a-f]{64}$'),
    CONSTRAINT chk_atelier_mm_drift_guard_findings_array
        CHECK (jsonb_typeof(findings) = 'array'),
    CONSTRAINT chk_atelier_mm_drift_guard_finding_count
        CHECK (finding_count >= 0 AND finding_count = jsonb_array_length(findings))
);

CREATE INDEX IF NOT EXISTS idx_atelier_mm_drift_guard_scope
    ON atelier_model_manual_drift_guard(guard_scope, created_at_utc DESC);
