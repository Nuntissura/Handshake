-- WP-KERNEL-005 MT-126/MT-127/MT-128: ComfyUI job lifecycle queue.
-- The Handshake-native, Comfy-compatible JOB LIFECYCLE record: a job request is
-- enqueued (QUEUED), advanced through RUNNING, and resolved to a terminal state
-- (COMPLETED, FAILED, CANCELLED, TIMED_OUT). This is the managed Workflow-Engine
-- ownership surface; it never executes ComfyUI, never opens a socket, never
-- spawns a process. The queue is Handshake-managed durable state, not a live
-- ComfyUI daemon dependency. It pairs with (but is distinct from) the workflow
-- *receipt* in atelier_comfy_workflow_receipt: the receipt summarizes a completed
-- run's refs/outputs, while this table is the request + poll + cancel/timeout
-- state machine that precedes/drives a run.
--
-- Lifecycle (MT-126 enqueue, MT-127 poll/advance, MT-128 timeout/cancel):
--   QUEUED -> RUNNING -> COMPLETED
--   QUEUED -> RUNNING -> FAILED
--   QUEUED|RUNNING       -> CANCELLED   (cancel of a terminal state is rejected)
--   QUEUED|RUNNING       -> TIMED_OUT
-- On cancel/timeout/fail the partial_evidence_ref preserves any partial artifact
-- or log ref so no evidence is lost (MT-128 partial-evidence preservation). The
-- ref is a portable Handshake-native handle (legacy/.GOV/SQLite/localhost/
-- machine-local refs are rejected in code via reject_legacy_runtime_ref).
--
-- Storage authority is PostgreSQL only (LAW-COMFY-INTAKE-004); SQLite is
-- forbidden.

CREATE TABLE IF NOT EXISTS atelier_comfy_job (
    job_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- The workflow run this job belongs to (containment, Section 6.9.1). One job
    -- per run: re-enqueue of the same run is an idempotent upsert.
    workflow_run_id UUID NOT NULL UNIQUE,
    -- Optional FK to the registered, versioned workflow spec this job executes.
    spec_id UUID REFERENCES atelier_comfy_workflow_spec(spec_id),
    -- The captured job request (the Comfy-compatible job creation contract). Stored
    -- as scrubbed JSON; never raw bytes, never credential material.
    request_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    status TEXT NOT NULL DEFAULT 'QUEUED',
    -- Portable Handshake-native ref to preserved partial evidence (artifact/log)
    -- captured on cancel/timeout/fail so no evidence is lost (MT-128). Null while
    -- the job is QUEUED/RUNNING with nothing preserved yet.
    partial_evidence_ref TEXT,
    -- The terminal failure/cancel/timeout reason, when one applies.
    error_reason TEXT,
    queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    CONSTRAINT chk_atelier_comfy_job_status
        CHECK (status IN (
            'QUEUED', 'RUNNING', 'COMPLETED', 'FAILED', 'CANCELLED', 'TIMED_OUT'
        )),
    CONSTRAINT chk_atelier_comfy_job_request_json
        CHECK (jsonb_typeof(request_json) = 'object')
);

CREATE INDEX IF NOT EXISTS idx_atelier_comfy_job_status
    ON atelier_comfy_job(status, queued_at);

CREATE INDEX IF NOT EXISTS idx_atelier_comfy_job_spec
    ON atelier_comfy_job(spec_id);
