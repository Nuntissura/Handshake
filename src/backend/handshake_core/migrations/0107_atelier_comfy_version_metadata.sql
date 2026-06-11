-- WP-KERNEL-005 MT-110: external tool and model version policy.
-- Captures version discovery/pinning for the three named provenance versions so
-- a workflow's provenance is reproducible:
--   * pose_model_asset_version -- the pinned pose model-asset version,
--   * image_tool_version       -- the pinned image-tool version (blur/sharpen/etc),
--   * comfy_model_version      -- the pinned ComfyUI model version.
-- Linked to a workflow run (one metadata row per run, idempotent upsert keyed on
-- workflow_run_id) and optionally to a registered workflow spec for spec-level
-- provenance. Storage authority is PostgreSQL only (LAW-COMFY-INTAKE-004);
-- SQLite is forbidden.

CREATE TABLE IF NOT EXISTS atelier_comfy_version_metadata (
    version_metadata_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_run_id UUID NOT NULL UNIQUE,
    -- Optional FK to the registered workflow spec this run executed.
    spec_id UUID REFERENCES atelier_comfy_workflow_spec(spec_id),
    -- The three pinned provenance versions (named by the MT-110 contract).
    pose_model_asset_version TEXT NOT NULL,
    image_tool_version TEXT NOT NULL,
    comfy_model_version TEXT NOT NULL,
    -- Optional preflight discovery evidence (what was probed, what was found).
    preflight_evidence JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_comfy_version_metadata_pose
        CHECK (
            btrim(pose_model_asset_version) = pose_model_asset_version
            AND pose_model_asset_version <> ''
        ),
    CONSTRAINT chk_atelier_comfy_version_metadata_image_tool
        CHECK (
            btrim(image_tool_version) = image_tool_version
            AND image_tool_version <> ''
        ),
    CONSTRAINT chk_atelier_comfy_version_metadata_comfy
        CHECK (
            btrim(comfy_model_version) = comfy_model_version
            AND comfy_model_version <> ''
        ),
    CONSTRAINT chk_atelier_comfy_version_metadata_evidence_json
        CHECK (jsonb_typeof(preflight_evidence) = 'object')
);

CREATE INDEX IF NOT EXISTS idx_atelier_comfy_version_metadata_spec
    ON atelier_comfy_version_metadata(spec_id);
