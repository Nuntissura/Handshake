-- WP-KERNEL-005 MT-102: output-first ComfyUI registration failure recovery.
-- Preserve generated output refs when registration fails after save, then let
-- a later retry register the saved image into atelier_comfy_intake_output.

CREATE TABLE IF NOT EXISTS atelier_comfy_output_registration_failure (
    failure_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_run_id UUID NOT NULL,
    node_execution_id TEXT NOT NULL,
    attempted_registration_id UUID,
    source_node_instance_id TEXT NOT NULL,
    source_output_slot TEXT NOT NULL,
    media_kind TEXT NOT NULL CHECK (media_kind IN ('image', 'mask', 'latent_preview', 'video', 'sidecar_json')),
    mime TEXT NOT NULL,
    artifact_ref TEXT NOT NULL,
    artifact_manifest_ref TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    routing_intent TEXT NOT NULL CHECK (routing_intent IN ('artifact', 'sidecar', 'transient')),
    parent_artifact_ref TEXT,
    prompt_json_ref TEXT,
    graph_hash TEXT,
    seed BIGINT,
    workflow_input_metadata JSONB NOT NULL DEFAULT '{"identity": {}}'::jsonb,
    failure_stage TEXT NOT NULL,
    failure_reason TEXT NOT NULL,
    evidence JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'retryable' CHECK (status IN ('retryable', 'registered')),
    retry_count INTEGER NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
    resolved_intake_output_id UUID REFERENCES atelier_comfy_intake_output(intake_output_id),
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_atelier_comfy_output_registration_failure_dedup
        UNIQUE (workflow_run_id, content_hash, failure_stage),
    CONSTRAINT chk_atelier_comfy_output_registration_failure_text
        CHECK (
            btrim(node_execution_id) = node_execution_id
            AND node_execution_id <> ''
            AND btrim(source_node_instance_id) = source_node_instance_id
            AND source_node_instance_id <> ''
            AND btrim(source_output_slot) = source_output_slot
            AND source_output_slot <> ''
            AND btrim(mime) = mime
            AND mime <> ''
            AND btrim(content_hash) = content_hash
            AND content_hash <> ''
            AND btrim(failure_stage) = failure_stage
            AND failure_stage <> ''
            AND btrim(failure_reason) = failure_reason
            AND failure_reason <> ''
        ),
    CONSTRAINT chk_atelier_comfy_output_registration_failure_refs
        CHECK (
            btrim(artifact_ref) = artifact_ref
            AND artifact_ref <> ''
            AND btrim(artifact_manifest_ref) = artifact_manifest_ref
            AND artifact_manifest_ref <> ''
            AND (
                parent_artifact_ref IS NULL
                OR (
                    btrim(parent_artifact_ref) = parent_artifact_ref
                    AND parent_artifact_ref <> ''
                )
            )
            AND (
                prompt_json_ref IS NULL
                OR (
                    btrim(prompt_json_ref) = prompt_json_ref
                    AND prompt_json_ref <> ''
                )
            )
        ),
    CONSTRAINT chk_atelier_comfy_output_registration_failure_sidecar_parent
        CHECK (routing_intent <> 'sidecar' OR parent_artifact_ref IS NOT NULL),
    CONSTRAINT chk_atelier_comfy_output_registration_failure_workflow_input_metadata_json
        CHECK (jsonb_typeof(workflow_input_metadata) = 'object'),
    CONSTRAINT chk_atelier_comfy_output_registration_failure_evidence_json
        CHECK (jsonb_typeof(evidence) = 'object'),
    CONSTRAINT chk_atelier_comfy_output_registration_failure_resolution
        CHECK (
            (status = 'retryable' AND resolved_intake_output_id IS NULL)
            OR (status = 'registered' AND resolved_intake_output_id IS NOT NULL)
        )
);

CREATE INDEX IF NOT EXISTS idx_atelier_comfy_output_registration_failure_run_status
    ON atelier_comfy_output_registration_failure(workflow_run_id, status, updated_at_utc);
