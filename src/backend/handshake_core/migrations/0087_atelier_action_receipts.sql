-- WP-KERNEL-005 MT-139 generic model-visible action receipt schema.
-- Receipts persist action identity, params hash, actor/session, timing, status,
-- and refs. Raw params are intentionally not stored.

CREATE TABLE IF NOT EXISTS atelier_action_receipt (
    receipt_id        UUID PRIMARY KEY,
    action_id         TEXT NOT NULL,
    params_sha256     TEXT NOT NULL,
    actor_kind        TEXT NOT NULL,
    actor_id          TEXT NOT NULL,
    session_id        TEXT NOT NULL,
    started_at_utc    TIMESTAMPTZ NOT NULL,
    completed_at_utc  TIMESTAMPTZ NOT NULL,
    status            TEXT NOT NULL,
    target_refs       JSONB NOT NULL,
    evidence_refs     JSONB NOT NULL,
    result_refs       JSONB NOT NULL,
    error_class       TEXT,
    recovery_hint     TEXT,
    created_at_utc    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_atelier_action_receipt_action_id_trimmed
        CHECK (action_id = btrim(action_id) AND action_id <> ''),
    CONSTRAINT chk_atelier_action_receipt_params_sha256
        CHECK (params_sha256 ~ '^sha256:[0-9a-f]{64}$'),
    CONSTRAINT chk_atelier_action_receipt_actor_kind_trimmed
        CHECK (actor_kind = btrim(actor_kind) AND actor_kind <> ''),
    CONSTRAINT chk_atelier_action_receipt_actor_id_trimmed
        CHECK (actor_id = btrim(actor_id) AND actor_id <> ''),
    CONSTRAINT chk_atelier_action_receipt_session_id_trimmed
        CHECK (session_id = btrim(session_id) AND session_id <> ''),
    CONSTRAINT chk_atelier_action_receipt_timing
        CHECK (completed_at_utc >= started_at_utc),
    CONSTRAINT chk_atelier_action_receipt_status
        CHECK (status IN ('succeeded', 'failed', 'rejected')),
    CONSTRAINT chk_atelier_action_receipt_target_refs_array
        CHECK (jsonb_typeof(target_refs) = 'array' AND jsonb_array_length(target_refs) > 0),
    CONSTRAINT chk_atelier_action_receipt_evidence_refs_array
        CHECK (jsonb_typeof(evidence_refs) = 'array' AND jsonb_array_length(evidence_refs) > 0),
    CONSTRAINT chk_atelier_action_receipt_result_refs_array
        CHECK (jsonb_typeof(result_refs) = 'array' AND jsonb_array_length(result_refs) > 0),
    CONSTRAINT chk_atelier_action_receipt_failure_recovery
        CHECK (
            status = 'succeeded'
            OR (
                error_class IS NOT NULL
                AND btrim(error_class) <> ''
                AND recovery_hint IS NOT NULL
                AND btrim(recovery_hint) <> ''
            )
        )
);

CREATE INDEX IF NOT EXISTS idx_atelier_action_receipt_action
    ON atelier_action_receipt(action_id, created_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_atelier_action_receipt_session
    ON atelier_action_receipt(session_id, created_at_utc DESC);
