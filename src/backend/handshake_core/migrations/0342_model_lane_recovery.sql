-- WP-1 MT-007: Dexterity recovery, replay, diagnostic-tier, and MT status authority.
--
-- These tables are product runtime state, not repo-governance paperwork.
-- PostgreSQL tables hold typed runtime state; kernel_event_ledger payloads are
-- the append-only authority used to reject mutable-row drift during recovery.

INSERT INTO model_lane_schema_registry
    (schema_id, schema_version, record_kind, table_name)
VALUES
    ('hsk.model_lane_recovery_checkpoint@1', 1, 'ModelLaneRecoveryCheckpoint', 'model_lane_recovery_checkpoints'),
    ('hsk.model_lane_recovery_event@1', 1, 'ModelLaneRecoveryEvent', 'model_lane_recovery_events'),
    ('hsk.model_lane_lease@1', 1, 'ModelLaneLease', 'model_lane_leases'),
    ('hsk.model_lane_diagnostic_tier@1', 1, 'ModelLaneDiagnosticTierStatus', 'model_lane_diagnostic_tier_statuses'),
    ('hsk.model_lane_mt_runtime_status@1', 1, 'ModelLaneMtRuntimeStatus', 'model_lane_mt_runtime_statuses')
ON CONFLICT (schema_id) DO UPDATE SET
    schema_version = EXCLUDED.schema_version,
    record_kind = EXCLUDED.record_kind,
    table_name = EXCLUDED.table_name,
    source_component = EXCLUDED.source_component;

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lanes_run_lane_unique
    ON model_lanes(run_id, lane_id);

CREATE TABLE IF NOT EXISTS model_lane_recovery_checkpoints (
    checkpoint_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES model_lane_runs(run_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    lane_id TEXT,
    session_id TEXT NOT NULL,
    model_session_id TEXT NOT NULL,
    lane_status TEXT NOT NULL,
    checkpoint_status TEXT NOT NULL,
    last_event_ledger_seq BIGINT NOT NULL,
    last_message_id TEXT,
    open_payload_refs JSONB NOT NULL,
    lease_id TEXT,
    idempotency_scope TEXT NOT NULL,
    recovery_state TEXT NOT NULL,
    recovery_event_ref TEXT,
    event_ledger_stream_id TEXT NOT NULL,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT NOT NULL,
    task_board_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    created_at_utc TIMESTAMPTZ NOT NULL,
    recovery_hint_ref TEXT,
    diagnostic_payload JSONB NOT NULL,
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL,
    event_stream_version BIGINT NOT NULL,
    transaction_seq BIGINT NOT NULL,
    record_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_model_lane_recovery_checkpoints_status
        CHECK (checkpoint_status IN ('observed','checkpointed','recovered','failed')),
    CONSTRAINT chk_model_lane_recovery_checkpoints_lane_status
        CHECK (lane_status IN ('planned','ready','running','waiting','completed','failed','cancelled','reclaimable')),
    CONSTRAINT chk_model_lane_recovery_checkpoints_recovery_state
        CHECK (recovery_state IN ('restartable','reclaimable','terminal','blocked')),
    CONSTRAINT chk_model_lane_recovery_checkpoints_event_seq_positive
        CHECK (last_event_ledger_seq > 0),
    CONSTRAINT chk_model_lane_recovery_checkpoints_payload_refs
        CHECK (jsonb_typeof(open_payload_refs) = 'array'),
    CONSTRAINT chk_model_lane_recovery_checkpoints_diag
        CHECK (jsonb_typeof(diagnostic_payload) = 'object'),
    CONSTRAINT chk_model_lane_recovery_checkpoints_record
        CHECK (jsonb_typeof(record_json) = 'object')
);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_model_lane_recovery_checkpoints_run_lane'
    ) THEN
        ALTER TABLE model_lane_recovery_checkpoints
            ADD CONSTRAINT fk_model_lane_recovery_checkpoints_run_lane
            FOREIGN KEY (run_id, lane_id)
            REFERENCES model_lanes(run_id, lane_id)
            ON UPDATE RESTRICT
            ON DELETE RESTRICT;
    END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lane_recovery_checkpoints_event_seq
    ON model_lane_recovery_checkpoints(event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_recovery_checkpoints_run
    ON model_lane_recovery_checkpoints(run_id, event_ledger_seq DESC);

CREATE INDEX IF NOT EXISTS idx_model_lane_recovery_checkpoints_locus
    ON model_lane_recovery_checkpoints(work_packet_id, micro_task_id, task_board_id, owner_session);

CREATE TABLE IF NOT EXISTS model_lane_recovery_events (
    recovery_event_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES model_lane_runs(run_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    lane_id TEXT,
    trace_id TEXT NOT NULL,
    span_id TEXT NOT NULL,
    parent_span_id TEXT,
    linked_span_contexts JSONB NOT NULL,
    session_id TEXT,
    model_session_id TEXT,
    event_kind TEXT NOT NULL,
    recovery_status TEXT NOT NULL,
    replay_order_seq BIGINT NOT NULL,
    source_event_ledger_seq BIGINT,
    payload_refs JSONB NOT NULL,
    artifact_refs JSONB NOT NULL,
    crdt_base_snapshot_ref TEXT,
    crdt_state_vector TEXT,
    crdt_stale_base_ref TEXT,
    lease_id TEXT,
    failure_kind TEXT,
    error_code TEXT,
    replay_hint TEXT NOT NULL,
    event_ledger_stream_id TEXT NOT NULL,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT NOT NULL,
    task_board_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    recovery_hint_ref TEXT,
    diagnostic_payload JSONB NOT NULL,
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL,
    event_stream_version BIGINT NOT NULL,
    transaction_seq BIGINT NOT NULL,
    record_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_model_lane_recovery_events_replay_order
        UNIQUE (run_id, replay_order_seq),
    CONSTRAINT chk_model_lane_recovery_events_kind
        CHECK (event_kind IN (
            'run_created','run_completed','run_failed',
            'lane_planned','lane_started','lane_status_changed','lane_completed','lane_failed','lane_cancelled','orphan_detected',
            'message_recorded','payload_ref_recorded','payload_ref_missing',
            'recovery_requested','replay_reconstructed','recovery_failed',
            'checkpoint_restored','crdt_update_observed','payload_ref_observed','lease_observed','cloud_consent_denied','mt_status_restored'
        )),
    CONSTRAINT chk_model_lane_recovery_events_status
        CHECK (recovery_status IN ('observed','checkpointed','recovered','failed')),
    CONSTRAINT chk_model_lane_recovery_events_replay_order_positive
        CHECK (replay_order_seq > 0),
    CONSTRAINT chk_model_lane_recovery_events_source_positive
        CHECK (source_event_ledger_seq IS NULL OR source_event_ledger_seq > 0),
    CONSTRAINT chk_model_lane_recovery_events_links
        CHECK (jsonb_typeof(linked_span_contexts) = 'array'),
    CONSTRAINT chk_model_lane_recovery_events_payload_refs
        CHECK (jsonb_typeof(payload_refs) = 'array'),
    CONSTRAINT chk_model_lane_recovery_events_artifact_refs
        CHECK (jsonb_typeof(artifact_refs) = 'array'),
    CONSTRAINT chk_model_lane_recovery_events_diag
        CHECK (jsonb_typeof(diagnostic_payload) = 'object'),
    CONSTRAINT chk_model_lane_recovery_events_record
        CHECK (jsonb_typeof(record_json) = 'object')
);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_model_lane_recovery_events_run_lane'
    ) THEN
        ALTER TABLE model_lane_recovery_events
            ADD CONSTRAINT fk_model_lane_recovery_events_run_lane
            FOREIGN KEY (run_id, lane_id)
            REFERENCES model_lanes(run_id, lane_id)
            ON UPDATE RESTRICT
            ON DELETE RESTRICT;
    END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lane_recovery_events_event_seq
    ON model_lane_recovery_events(event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_recovery_events_run_replay
    ON model_lane_recovery_events(run_id, replay_order_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_recovery_events_locus
    ON model_lane_recovery_events(work_packet_id, micro_task_id, task_board_id, owner_session);

CREATE TABLE IF NOT EXISTS model_lane_leases (
    lease_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES model_lane_runs(run_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    lane_id TEXT,
    scope TEXT NOT NULL,
    scope_ref TEXT NOT NULL,
    holder_actor_id TEXT NOT NULL,
    holder_session_id TEXT NOT NULL,
    lease_expires_at_utc TIMESTAMPTZ NOT NULL,
    takeover_policy_ref TEXT NOT NULL,
    state TEXT NOT NULL,
    event_ledger_stream_id TEXT NOT NULL,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT NOT NULL,
    task_board_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    recovery_hint_ref TEXT,
    diagnostic_payload JSONB NOT NULL,
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL,
    event_stream_version BIGINT NOT NULL,
    transaction_seq BIGINT NOT NULL,
    record_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_model_lane_leases_scope
        CHECK (scope IN ('run','lane')),
    CONSTRAINT chk_model_lane_leases_state
        CHECK (state IN ('active','released','reclaimed','cancelled')),
    CONSTRAINT chk_model_lane_leases_diag
        CHECK (jsonb_typeof(diagnostic_payload) = 'object'),
    CONSTRAINT chk_model_lane_leases_record
        CHECK (jsonb_typeof(record_json) = 'object')
);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'fk_model_lane_leases_run_lane'
    ) THEN
        ALTER TABLE model_lane_leases
            ADD CONSTRAINT fk_model_lane_leases_run_lane
            FOREIGN KEY (run_id, lane_id)
            REFERENCES model_lanes(run_id, lane_id)
            ON UPDATE RESTRICT
            ON DELETE RESTRICT;
    END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lane_leases_event_seq
    ON model_lane_leases(event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_leases_run
    ON model_lane_leases(run_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_leases_locus
    ON model_lane_leases(work_packet_id, micro_task_id, task_board_id, owner_session);

CREATE TABLE IF NOT EXISTS model_lane_diagnostic_tier_statuses (
    diagnostic_status_id TEXT PRIMARY KEY,
    behavior_id TEXT NOT NULL,
    run_id TEXT NOT NULL REFERENCES model_lane_runs(run_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    tier TEXT NOT NULL,
    state TEXT NOT NULL,
    reason TEXT NOT NULL,
    evidence_ref TEXT NOT NULL,
    follow_up_ref TEXT,
    event_ledger_stream_id TEXT NOT NULL,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT NOT NULL,
    task_board_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    diagnostic_payload JSONB NOT NULL,
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL,
    event_stream_version BIGINT NOT NULL,
    transaction_seq BIGINT NOT NULL,
    record_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_model_lane_diag_tier
        CHECK (tier IN ('flight_recorder','internal_diagnostics','palmistry')),
    CONSTRAINT chk_model_lane_diag_state
        CHECK (state IN ('wired','deferred_with_reason','missing')),
    CONSTRAINT chk_model_lane_diag_payload
        CHECK (jsonb_typeof(diagnostic_payload) = 'object'),
    CONSTRAINT chk_model_lane_diag_record
        CHECK (jsonb_typeof(record_json) = 'object')
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lane_diagnostic_tiers_event_seq
    ON model_lane_diagnostic_tier_statuses(event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_diagnostic_tiers_behavior
    ON model_lane_diagnostic_tier_statuses(run_id, behavior_id, event_ledger_seq DESC);

CREATE TABLE IF NOT EXISTS model_lane_mt_runtime_statuses (
    mt_status_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES model_lane_runs(run_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT NOT NULL,
    task_board_id TEXT NOT NULL,
    status TEXT NOT NULL,
    claimed_by_ref TEXT,
    blocker_ref TEXT,
    missing_resource_ref TEXT,
    proof_status_ref TEXT,
    hbr_status_ref TEXT,
    last_recovery_event_ref TEXT,
    last_runtime_status_ref TEXT,
    event_ledger_stream_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    diagnostic_payload JSONB NOT NULL,
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL,
    event_stream_version BIGINT NOT NULL,
    transaction_seq BIGINT NOT NULL,
    record_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_model_lane_mt_runtime_status
        CHECK (status IN ('pending','claimed','blocked','proof_running','ready_for_validation','completed')),
    CONSTRAINT chk_model_lane_mt_runtime_status_diag
        CHECK (jsonb_typeof(diagnostic_payload) = 'object'),
    CONSTRAINT chk_model_lane_mt_runtime_status_record
        CHECK (jsonb_typeof(record_json) = 'object')
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lane_mt_runtime_statuses_event_seq
    ON model_lane_mt_runtime_statuses(event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_mt_runtime_statuses_run
    ON model_lane_mt_runtime_statuses(run_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_mt_runtime_statuses_locus
    ON model_lane_mt_runtime_statuses(work_packet_id, micro_task_id, task_board_id, owner_session);
