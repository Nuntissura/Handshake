-- WP-KERNEL-004 cluster X.3 MT-190/MT-193/MT-194 schema:
-- SessionCheckpoint + ResumeReport + IdempotencyLedger.

CREATE TABLE IF NOT EXISTS kernel_session_checkpoint (
    checkpoint_id UUID PRIMARY KEY NOT NULL,
    session_id UUID NOT NULL,
    model_session_id UUID NOT NULL,
    last_event_ledger_seq BIGINT NOT NULL,
    compact_state JSONB NOT NULL,
    state_kind TEXT NOT NULL,
    pending_artifacts JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by_process INTEGER NOT NULL,
    schema_version INTEGER NOT NULL DEFAULT 1,
    CONSTRAINT compact_state_size CHECK (octet_length(compact_state::text) <= 32768)
);

CREATE INDEX IF NOT EXISTS idx_kernel_session_checkpoint_session
    ON kernel_session_checkpoint (session_id, created_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_kernel_session_checkpoint_model_session
    ON kernel_session_checkpoint (model_session_id);

CREATE TABLE IF NOT EXISTS kernel_restart_resume_report (
    report_id UUID PRIMARY KEY NOT NULL,
    sessions_examined INTEGER NOT NULL,
    sessions_resumed JSONB NOT NULL DEFAULT '[]'::jsonb,
    sessions_recovery_failed JSONB NOT NULL DEFAULT '[]'::jsonb,
    total_replay_events BIGINT NOT NULL,
    total_duration_ms BIGINT NOT NULL,
    started_at_utc TIMESTAMPTZ NOT NULL,
    completed_at_utc TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS kernel_idempotency_ledger (
    session_id UUID NOT NULL,
    event_seq BIGINT NOT NULL,
    side_effect_kind TEXT NOT NULL,
    applied_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (session_id, event_seq, side_effect_kind)
);

-- MT-196/MT-197 Flight Recorder span tables.
CREATE TABLE IF NOT EXISTS kernel_model_session_span (
    span_id UUID PRIMARY KEY NOT NULL,
    model_session_id UUID NOT NULL,
    session_id UUID NOT NULL,
    started_at_utc TIMESTAMPTZ NOT NULL,
    ended_at_utc TIMESTAMPTZ,
    status TEXT NOT NULL,
    attributes JSONB NOT NULL DEFAULT '{}'::jsonb,
    last_event_ledger_seq BIGINT
);

CREATE INDEX IF NOT EXISTS idx_kernel_model_session_span_model_session
    ON kernel_model_session_span (model_session_id, started_at_utc DESC);

CREATE TABLE IF NOT EXISTS kernel_activity_span (
    span_id UUID PRIMARY KEY NOT NULL,
    parent_span_id UUID NOT NULL REFERENCES kernel_model_session_span(span_id) ON DELETE CASCADE,
    activity_kind TEXT NOT NULL,
    started_at_utc TIMESTAMPTZ NOT NULL,
    ended_at_utc TIMESTAMPTZ,
    status TEXT NOT NULL,
    attributes JSONB NOT NULL DEFAULT '{}'::jsonb,
    related_event_ledger_seqs JSONB NOT NULL DEFAULT '[]'::jsonb
);

CREATE INDEX IF NOT EXISTS idx_kernel_activity_span_parent
    ON kernel_activity_span (parent_span_id, started_at_utc);

CREATE INDEX IF NOT EXISTS idx_kernel_activity_span_kind
    ON kernel_activity_span (activity_kind);
