-- WP-KERNEL-005 MT-155/MT-157: durable visual-diff baselines, diff requests,
-- and computed comparison results for the kernel visual debugging loop.
--
-- MT-155 (Visual Diff Baseline Contract): screenshot baselines and the
-- standalone diff-request schema (threshold + metadata) persist in PostgreSQL
-- instead of living only as embedded fields of an in-memory loop projection.
-- A diff request binds EITHER a registered baseline row (baseline_id) OR the
-- previous screenshot artifact ref (previous_screenshot_ref) -- the
-- "baseline-or-previous" comparison contract.
--
-- MT-157 (Pixel Versus Structural Comparison): a computed comparison result
-- row carries the RESULT fields (units compared/differing, mismatch basis
-- points, threshold verdict, outcome) for the pixel_diff / structural_dom /
-- manual strategy enum. Manual comparisons persist in the
-- 'manual_review_required' outcome until an operator verdict is recorded.
--
-- Storage authority is PostgreSQL only; refs are portable artifact:// tokens
-- (never .GOV, SQLite, or machine-local paths).

CREATE TABLE IF NOT EXISTS kernel_visual_diff_baseline (
    baseline_id       UUID PRIMARY KEY,
    surface_id        TEXT NOT NULL,
    baseline_ref      TEXT NOT NULL,
    content_sha256    TEXT NOT NULL,
    captured_by       TEXT NOT NULL,
    captured_at_utc   TIMESTAMPTZ NOT NULL,
    created_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_kernel_vd_baseline_surface_id
        CHECK (btrim(surface_id) = surface_id AND surface_id <> ''),
    CONSTRAINT chk_kernel_vd_baseline_ref
        CHECK (baseline_ref LIKE 'artifact://baselines/%'),
    CONSTRAINT chk_kernel_vd_baseline_sha256
        CHECK (content_sha256 ~ '^sha256:[0-9a-f]{64}$'),
    CONSTRAINT chk_kernel_vd_baseline_captured_by
        CHECK (btrim(captured_by) = captured_by AND captured_by <> ''),
    CONSTRAINT uq_kernel_vd_baseline_surface_ref
        UNIQUE (surface_id, baseline_ref)
);

CREATE INDEX IF NOT EXISTS idx_kernel_vd_baseline_surface_latest
    ON kernel_visual_diff_baseline (surface_id, captured_at_utc DESC, baseline_id DESC);

CREATE TABLE IF NOT EXISTS kernel_visual_diff_request (
    request_id                    UUID PRIMARY KEY,
    surface_id                    TEXT NOT NULL,
    baseline_id                   UUID REFERENCES kernel_visual_diff_baseline(baseline_id),
    previous_screenshot_ref       TEXT,
    candidate_screenshot_ref      TEXT NOT NULL,
    comparison_mode               TEXT NOT NULL,
    threshold_config_ref          TEXT NOT NULL,
    max_pixel_diff_basis_points   INTEGER NOT NULL,
    max_layout_shift_basis_points INTEGER NOT NULL,
    structural_mismatch_limit     INTEGER NOT NULL,
    metadata_json                 JSONB NOT NULL DEFAULT '{}'::jsonb,
    requested_by                  TEXT NOT NULL,
    created_at_utc                TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_kernel_vd_request_surface_id
        CHECK (btrim(surface_id) = surface_id AND surface_id <> ''),
    -- baseline-or-previous: exactly one comparison reference source.
    CONSTRAINT chk_kernel_vd_request_baseline_or_previous
        CHECK (
            (baseline_id IS NOT NULL AND previous_screenshot_ref IS NULL)
            OR (baseline_id IS NULL AND previous_screenshot_ref IS NOT NULL)
        ),
    CONSTRAINT chk_kernel_vd_request_previous_ref
        CHECK (
            previous_screenshot_ref IS NULL
            OR previous_screenshot_ref LIKE 'artifact://screenshots/%'
        ),
    CONSTRAINT chk_kernel_vd_request_candidate_ref
        CHECK (candidate_screenshot_ref LIKE 'artifact://screenshots/%'),
    CONSTRAINT chk_kernel_vd_request_comparison_mode
        CHECK (comparison_mode IN ('pixel_diff', 'structural_dom', 'manual')),
    CONSTRAINT chk_kernel_vd_request_threshold_config_ref
        CHECK (threshold_config_ref LIKE 'packet://%'),
    CONSTRAINT chk_kernel_vd_request_pixel_threshold
        CHECK (max_pixel_diff_basis_points > 0),
    CONSTRAINT chk_kernel_vd_request_layout_threshold
        CHECK (max_layout_shift_basis_points > 0),
    CONSTRAINT chk_kernel_vd_request_structural_limit
        CHECK (structural_mismatch_limit >= 0),
    CONSTRAINT chk_kernel_vd_request_metadata_json
        CHECK (jsonb_typeof(metadata_json) = 'object'),
    CONSTRAINT chk_kernel_vd_request_requested_by
        CHECK (btrim(requested_by) = requested_by AND requested_by <> '')
);

CREATE INDEX IF NOT EXISTS idx_kernel_vd_request_surface
    ON kernel_visual_diff_request (surface_id, created_at_utc DESC);

CREATE TABLE IF NOT EXISTS kernel_visual_diff_result (
    result_id              UUID PRIMARY KEY,
    request_id             UUID NOT NULL REFERENCES kernel_visual_diff_request(request_id),
    comparison_mode        TEXT NOT NULL,
    units_compared         BIGINT NOT NULL,
    units_differing        BIGINT NOT NULL,
    mismatch_basis_points  INTEGER NOT NULL,
    threshold_exceeded     BOOLEAN NOT NULL,
    outcome                TEXT NOT NULL,
    computed_at_utc        TIMESTAMPTZ NOT NULL,
    created_at_utc         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_kernel_vd_result_comparison_mode
        CHECK (comparison_mode IN ('pixel_diff', 'structural_dom', 'manual')),
    CONSTRAINT chk_kernel_vd_result_units
        CHECK (units_compared >= 0 AND units_differing >= 0),
    CONSTRAINT chk_kernel_vd_result_basis_points
        CHECK (mismatch_basis_points BETWEEN 0 AND 10000),
    CONSTRAINT chk_kernel_vd_result_outcome
        CHECK (outcome IN ('pass', 'fail', 'manual_review_required')),
    -- Manual strategy never auto-passes/fails; computed strategies never
    -- park in manual review.
    CONSTRAINT chk_kernel_vd_result_manual_outcome
        CHECK (
            (comparison_mode = 'manual' AND outcome = 'manual_review_required')
            OR (comparison_mode <> 'manual' AND outcome IN ('pass', 'fail'))
        )
);

CREATE INDEX IF NOT EXISTS idx_kernel_vd_result_request
    ON kernel_visual_diff_result (request_id, created_at_utc DESC);
