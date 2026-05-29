-- WP-KERNEL-004 cluster X.2 MT-184/MT-185/MT-187/MT-188 schema:
-- MicroTaskJob queue + loop control + outcome record + distillation candidate.

CREATE TABLE IF NOT EXISTS kernel_micro_task_job (
    job_id UUID PRIMARY KEY NOT NULL,
    wp_id TEXT NOT NULL,
    mt_id TEXT NOT NULL,
    mt_contract_path TEXT NOT NULL,
    iteration_n BIGINT NOT NULL DEFAULT 0,
    max_iterations BIGINT NOT NULL,
    escalation_tier TEXT NOT NULL,
    escalation_history JSONB NOT NULL DEFAULT '[]'::jsonb,
    task_tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    lora_id TEXT,
    mailbox_thread_id UUID,
    state TEXT NOT NULL,
    claimed_by_session UUID,
    claimed_at_utc TIMESTAMPTZ,
    transition_reason TEXT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_kernel_micro_task_job_state_tier_created
    ON kernel_micro_task_job (state, escalation_tier, created_at_utc);

CREATE INDEX IF NOT EXISTS idx_kernel_micro_task_job_wp_mt
    ON kernel_micro_task_job (wp_id, mt_id);

CREATE INDEX IF NOT EXISTS idx_kernel_micro_task_job_mailbox_thread
    ON kernel_micro_task_job (mailbox_thread_id);

-- MT-185 loop control checkpoint.
CREATE TABLE IF NOT EXISTS kernel_mt_loop_checkpoint (
    checkpoint_id UUID PRIMARY KEY NOT NULL,
    job_id UUID NOT NULL REFERENCES kernel_micro_task_job(job_id) ON DELETE CASCADE,
    iteration_n BIGINT NOT NULL,
    state_at_checkpoint JSONB NOT NULL,
    retry_budget_remaining BIGINT NOT NULL,
    verifier_feedback_history JSONB NOT NULL DEFAULT '[]'::jsonb,
    compact_summary TEXT NOT NULL,
    evidence_pointers JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by_session UUID NOT NULL,
    CONSTRAINT compact_summary_bound CHECK (octet_length(compact_summary) <= 2048)
);

CREATE INDEX IF NOT EXISTS idx_kernel_mt_loop_checkpoint_job
    ON kernel_mt_loop_checkpoint (job_id, created_at_utc DESC);

-- MT-188 outcome record + distillation candidate.
CREATE TABLE IF NOT EXISTS kernel_mt_outcome (
    outcome_id UUID PRIMARY KEY NOT NULL,
    job_id UUID NOT NULL REFERENCES kernel_micro_task_job(job_id) ON DELETE CASCADE,
    iteration_n BIGINT NOT NULL,
    outcome_kind TEXT NOT NULL,
    completion_signal JSONB,
    run_ledger_ref JSONB,
    evidence_pointers JSONB NOT NULL DEFAULT '[]'::jsonb,
    distillation_candidate_id UUID,
    recorded_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    recorded_by_session UUID NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_kernel_mt_outcome_job
    ON kernel_mt_outcome (job_id, recorded_at_utc DESC);

CREATE TABLE IF NOT EXISTS kernel_distillation_candidate (
    candidate_id UUID PRIMARY KEY NOT NULL,
    job_id UUID NOT NULL REFERENCES kernel_micro_task_job(job_id) ON DELETE CASCADE,
    summary TEXT NOT NULL,
    prompt_response_trace JSONB NOT NULL DEFAULT '[]'::jsonb,
    tier_at_completion TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_kernel_distillation_candidate_job
    ON kernel_distillation_candidate (job_id);
