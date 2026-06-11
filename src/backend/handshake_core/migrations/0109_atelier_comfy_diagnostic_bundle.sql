-- WP-KERNEL-005 MT-112: Pose Diagnostic Bundle Hook.
-- Structured diagnostic bundle inputs for a FAILED pose/ComfyUI operation, so a
-- no-context model can triage without re-running the operation. Captures:
--   * request_json    -- the captured (scrubbed) request that failed,
--   * refs_json       -- the portable refs involved (object of named refs;
--                        every reachable string ref is rejected at record time
--                        via the canonical reject_legacy_runtime_ref boundary),
--   * versions_json   -- the pinned versions in effect at failure,
--   * logs_ref        -- a portable Handshake-native ref to the failure logs,
--   * artifacts_json  -- the artifacts involved in the failed operation,
--   * error_taxonomy  -- a stable token classifying the failure.
-- One bundle per run (idempotent upsert keyed on workflow_run_id). No raw bytes,
-- no credentials, no socket: governed DATA + RECEIPT model only
-- (LAW-COMFY-INTAKE-001/005). Storage authority is PostgreSQL only
-- (LAW-COMFY-INTAKE-004); SQLite is forbidden. The refs_json / logs_ref CHECK
-- constraints provide a DB-level safety net rejecting .GOV / SQLite / localhost /
-- machine-local refs in addition to the application-level boundary.

CREATE TABLE IF NOT EXISTS atelier_comfy_diagnostic_bundle (
    bundle_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_run_id UUID NOT NULL UNIQUE,
    -- The captured (scrubbed) request that failed.
    request_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- The portable refs involved (object of named refs).
    refs_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- The pinned versions in effect at failure.
    versions_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Portable Handshake-native ref to the failure logs (never inline bytes).
    logs_ref TEXT NOT NULL,
    -- The artifacts involved in the failed operation.
    artifacts_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Stable error-taxonomy token classifying the failure.
    error_taxonomy TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_comfy_diagnostic_bundle_request_json
        CHECK (jsonb_typeof(request_json) = 'object'),
    CONSTRAINT chk_atelier_comfy_diagnostic_bundle_versions_json
        CHECK (jsonb_typeof(versions_json) = 'object'),
    CONSTRAINT chk_atelier_comfy_diagnostic_bundle_artifacts_json
        CHECK (jsonb_typeof(artifacts_json) = 'object'),
    -- refs is an object; reject .GOV / SQLite / localhost anywhere in its text.
    CONSTRAINT chk_atelier_comfy_diagnostic_bundle_refs_json
        CHECK (
            jsonb_typeof(refs_json) = 'object'
            AND refs_json::text !~* '(\.gov|sqlite|localhost|127\.0\.0\.1|::1|file:)'
        ),
    -- logs_ref: non-empty, non-padded, portable (reject machine-local/legacy).
    CONSTRAINT chk_atelier_comfy_diagnostic_bundle_logs_ref
        CHECK (
            btrim(logs_ref) = logs_ref
            AND logs_ref <> ''
            AND logs_ref !~ '[[:space:]]'
            AND logs_ref !~* '(^[a-z]:|\.gov|file:|localhost|127\.0\.0\.1|::1|sqlite|\\|/\.\./)'
        ),
    -- error_taxonomy: non-empty, non-padded token.
    CONSTRAINT chk_atelier_comfy_diagnostic_bundle_error_taxonomy
        CHECK (
            btrim(error_taxonomy) = error_taxonomy
            AND error_taxonomy <> ''
        )
);
