-- WP-KERNEL-005 MT-101: durable ComfyUI workflow receipt schema.
-- Records the workflow spec/json/prompt refs, every materialized output ref,
-- status, optional error ref, structured evidence, and the receipt projection.

CREATE TABLE IF NOT EXISTS atelier_comfy_workflow_receipt (
    receipt_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    system_id TEXT NOT NULL,
    workflow_run_id UUID NOT NULL UNIQUE,
    workflow_spec_ref TEXT NOT NULL,
    workflow_json_ref TEXT NOT NULL,
    prompt_ref TEXT NOT NULL,
    all_refs JSONB NOT NULL DEFAULT '{}'::jsonb,
    outputs JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed', 'cancelled')),
    error_ref TEXT,
    evidence JSONB NOT NULL DEFAULT '{}'::jsonb,
    receipt_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_comfy_workflow_receipt_system_id
        CHECK (
            btrim(system_id) = system_id
            AND system_id <> ''
            AND system_id ~ '^[A-Za-z0-9._-]+$'
        ),
    CONSTRAINT chk_atelier_comfy_workflow_receipt_refs
        CHECK (
            btrim(workflow_spec_ref) = workflow_spec_ref
            AND workflow_spec_ref <> ''
            AND btrim(workflow_json_ref) = workflow_json_ref
            AND workflow_json_ref <> ''
            AND btrim(prompt_ref) = prompt_ref
            AND prompt_ref <> ''
        ),
    CONSTRAINT chk_atelier_comfy_workflow_receipt_error
        CHECK (
            (
                status = 'failed'
                AND error_ref IS NOT NULL
                AND btrim(error_ref) = error_ref
                AND error_ref <> ''
            )
            OR (
                status <> 'failed'
                AND (
                    error_ref IS NULL
                    OR (
                        btrim(error_ref) = error_ref
                        AND error_ref <> ''
                    )
                )
            )
        ),
    CONSTRAINT chk_atelier_comfy_workflow_receipt_refs_json
        CHECK (jsonb_typeof(all_refs) = 'object'),
    CONSTRAINT chk_atelier_comfy_workflow_receipt_outputs_json
        CHECK (jsonb_typeof(outputs) = 'array'),
    CONSTRAINT chk_atelier_comfy_workflow_receipt_evidence_json
        CHECK (jsonb_typeof(evidence) = 'object'),
    CONSTRAINT chk_atelier_comfy_workflow_receipt_receipt_json
        CHECK (
            jsonb_typeof(receipt_json) = 'object'
            AND receipt_json->>'schema' = 'hsk.atelier.comfy.workflow_receipt@1'
        )
);

CREATE INDEX IF NOT EXISTS idx_atelier_comfy_workflow_receipt_status
    ON atelier_comfy_workflow_receipt(status, updated_at_utc);
