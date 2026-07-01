-- WP-1 MT-004: Dexterity model-lane promotion decisions.
--
-- Advisory model output only becomes authority through a durable promotion
-- decision. Each decision is backed by kernel_event_ledger and replayable from
-- PostgreSQL for state recovery.

INSERT INTO model_lane_schema_registry
    (schema_id, schema_version, record_kind, table_name)
VALUES
    ('hsk.model_lane_promotion_decision@1', 1, 'ModelLanePromotionDecision', 'model_lane_promotion_decisions')
ON CONFLICT (schema_id) DO UPDATE SET
    schema_version = EXCLUDED.schema_version,
    record_kind = EXCLUDED.record_kind,
    table_name = EXCLUDED.table_name,
    source_component = EXCLUDED.source_component;

CREATE TABLE IF NOT EXISTS model_lane_promotion_decisions (
    decision_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL
        REFERENCES model_lane_runs(run_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    trace_id TEXT NOT NULL,
    decision_span_id TEXT NOT NULL,
    coordinator_session_id TEXT NOT NULL,
    routing_policy TEXT NOT NULL,
    outcome TEXT NOT NULL,
    final_state TEXT NOT NULL,
    denial_reason TEXT,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT NOT NULL,
    task_board_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    canonical_decision_hash TEXT NOT NULL,
    expected_event_ledger_aggregate_type TEXT NOT NULL,
    expected_event_ledger_aggregate_id TEXT NOT NULL,
    expected_event_ledger_version BIGINT NOT NULL,
    current_event_ledger_version BIGINT,
    schema_id TEXT NOT NULL,
    current_schema_id TEXT,
    base_snapshot_ref TEXT NOT NULL,
    current_base_snapshot_ref TEXT NOT NULL,
    state_vector TEXT NOT NULL,
    current_state_vector TEXT NOT NULL,
    promotion_gate_ref TEXT NOT NULL,
    promotion_receipt_ref TEXT,
    event_ledger_stream_id TEXT NOT NULL,
    event_ledger_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL,
    event_stream_version BIGINT NOT NULL,
    transaction_seq BIGINT NOT NULL,
    record_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_model_lane_promotion_decisions_hash
        CHECK (canonical_decision_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_promotion_decisions_record_object
        CHECK (jsonb_typeof(record_json) = 'object'),
    CONSTRAINT chk_model_lane_promotion_decisions_outcome
        CHECK (outcome IN ('approved', 'denied')),
    CONSTRAINT chk_model_lane_promotion_decisions_final_state
        CHECK (final_state IN (
            'advisory',
            'promotion_requested',
            'pending_policy',
            'pending_approval',
            'approved',
            'denied',
            'expired',
            'executing',
            'executed',
            'skipped',
            'unsupported'
        ))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lane_promotion_decisions_event_seq
    ON model_lane_promotion_decisions(event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_promotion_decisions_stream_replay
    ON model_lane_promotion_decisions(event_ledger_stream_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_promotion_decisions_run_replay
    ON model_lane_promotion_decisions(run_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_promotion_decisions_gate_receipt
    ON model_lane_promotion_decisions(run_id, promotion_gate_ref, promotion_receipt_ref, outcome);

CREATE INDEX IF NOT EXISTS idx_model_lane_promotion_decisions_locus
    ON model_lane_promotion_decisions(work_packet_id, micro_task_id, task_board_id, owner_session);

CREATE INDEX IF NOT EXISTS idx_model_lane_promotion_decisions_record_gin
    ON model_lane_promotion_decisions USING GIN (record_json);
