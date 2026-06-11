-- WP-KERNEL-005 MT-141: Kernel Diagnostic Bundle Manifest.
-- The kernel-level diagnostic bundle manifest a no-context model uses to
-- isolate a failure without re-running it. This is the KERNEL surface; it is
-- distinct from MT-112's pose/ComfyUI failure bundle
-- (atelier_comfy_diagnostic_bundle), which is scoped to a single workflow run.
-- A manifest row captures:
--   * subject_kind / subject_ref -- what failed (a portable, machine-citable
--     subject token; never a machine-local path, .GOV ref, or SQLite ref),
--   * failure_summary            -- one-line human/model readable summary,
--   * error_taxonomy             -- stable token classifying the failure,
--   * severity                   -- canonical diagnostics severity token,
--   * sections_json              -- ordered evidence sections (diagnostics,
--                                   event-ledger, state-probe, logs, env,
--                                   artifacts), each with a portable
--                                   content_ref and/or inline content_json,
--   * reproduction_json          -- deterministic reproduction steps,
--   * isolation_json             -- ordered isolation hints (check-first list).
-- Storage authority is PostgreSQL only; SQLite is forbidden. The sections_json
-- CHECK is a DB-level safety net rejecting .GOV / SQLite / localhost /
-- machine-local refs in addition to the application-level boundary
-- (reject_legacy_runtime_ref).

CREATE TABLE IF NOT EXISTS kernel_diagnostic_bundle_manifest (
    manifest_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    schema_id TEXT NOT NULL,
    subject_kind TEXT NOT NULL,
    subject_ref TEXT NOT NULL,
    failure_summary TEXT NOT NULL,
    error_taxonomy TEXT NOT NULL,
    severity TEXT NOT NULL,
    created_by TEXT NOT NULL,
    sections_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    reproduction_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    isolation_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_kernel_diag_bundle_manifest_schema_id
        CHECK (btrim(schema_id) = schema_id AND schema_id <> ''),
    CONSTRAINT chk_kernel_diag_bundle_manifest_subject_kind
        CHECK (btrim(subject_kind) = subject_kind AND subject_kind <> ''),
    -- subject_ref: non-empty, non-padded, portable (reject machine-local/legacy).
    CONSTRAINT chk_kernel_diag_bundle_manifest_subject_ref
        CHECK (
            btrim(subject_ref) = subject_ref
            AND subject_ref <> ''
            AND subject_ref !~ '[[:space:]]'
            AND subject_ref !~* '(^[a-z]:|\.gov|file:|localhost|127\.0\.0\.1|::1|sqlite|\\|/\.\./)'
        ),
    CONSTRAINT chk_kernel_diag_bundle_manifest_failure_summary
        CHECK (btrim(failure_summary) = failure_summary AND failure_summary <> ''),
    CONSTRAINT chk_kernel_diag_bundle_manifest_error_taxonomy
        CHECK (btrim(error_taxonomy) = error_taxonomy AND error_taxonomy <> ''),
    CONSTRAINT chk_kernel_diag_bundle_manifest_severity
        CHECK (severity IN ('fatal', 'error', 'warning', 'info', 'hint')),
    CONSTRAINT chk_kernel_diag_bundle_manifest_created_by
        CHECK (btrim(created_by) = created_by AND created_by <> ''),
    -- sections: non-empty array; reject .GOV / SQLite / localhost anywhere in
    -- its text (evidence bodies belong behind portable content_refs, not inline
    -- machine-local payloads).
    CONSTRAINT chk_kernel_diag_bundle_manifest_sections_json
        CHECK (
            jsonb_typeof(sections_json) = 'array'
            AND jsonb_array_length(sections_json) > 0
            AND sections_json::text !~* '(\.gov|sqlite|localhost|127\.0\.0\.1|file:)'
        ),
    CONSTRAINT chk_kernel_diag_bundle_manifest_reproduction_json
        CHECK (
            jsonb_typeof(reproduction_json) = 'array'
            AND jsonb_array_length(reproduction_json) > 0
        ),
    CONSTRAINT chk_kernel_diag_bundle_manifest_isolation_json
        CHECK (
            jsonb_typeof(isolation_json) = 'array'
            AND jsonb_array_length(isolation_json) > 0
        )
);

CREATE INDEX IF NOT EXISTS idx_kernel_diag_bundle_manifest_subject
    ON kernel_diagnostic_bundle_manifest (subject_kind, subject_ref, created_at_utc DESC);
