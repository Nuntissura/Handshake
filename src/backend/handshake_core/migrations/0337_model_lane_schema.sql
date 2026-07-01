-- WP-1 MT-002: Dexterity model-lane schema.
--
-- Dexterity is the operator/manual name for the internal model switching and
-- launch kernel. Stable machine schema IDs remain hsk.model_lane_*@1.
-- PostgreSQL rows are authority; kernel_event_ledger rows are the append-only
-- replay trail.

CREATE TABLE IF NOT EXISTS model_lane_schema_registry (
    schema_id TEXT PRIMARY KEY,
    schema_version INTEGER NOT NULL,
    record_kind TEXT NOT NULL,
    table_name TEXT NOT NULL,
    source_component TEXT NOT NULL DEFAULT 'dexterity_model_lane',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO model_lane_schema_registry
    (schema_id, schema_version, record_kind, table_name)
VALUES
    ('hsk.model_lane_run@1', 1, 'ModelLaneRun', 'model_lane_runs'),
    ('hsk.model_lane@1', 1, 'ModelLane', 'model_lanes'),
    ('hsk.model_lane_message@1', 1, 'ModelLaneMessage', 'model_lane_messages'),
    ('hsk.model_lane_terminal@1', 1, 'ModelLaneTerminal', 'model_lanes')
ON CONFLICT (schema_id) DO UPDATE SET
    schema_version = EXCLUDED.schema_version,
    record_kind = EXCLUDED.record_kind,
    table_name = EXCLUDED.table_name,
    source_component = EXCLUDED.source_component;

CREATE TABLE IF NOT EXISTS model_lane_runs (
    run_id TEXT PRIMARY KEY,
    trace_id TEXT NOT NULL,
    run_span_id TEXT NOT NULL,
    coordinator_session_id TEXT NOT NULL,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT NOT NULL,
    task_board_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    replay_order_key TEXT NOT NULL,
    event_ledger_stream_id TEXT NOT NULL,
    event_ledger_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL,
    record_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_model_lane_runs_record_object
        CHECK (jsonb_typeof(record_json) = 'object')
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lane_runs_event_seq
    ON model_lane_runs(event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_runs_stream_replay
    ON model_lane_runs(event_ledger_stream_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_runs_locus
    ON model_lane_runs(work_packet_id, micro_task_id, owner_session);

CREATE TABLE IF NOT EXISTS model_lanes (
    lane_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL
        REFERENCES model_lane_runs(run_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    trace_id TEXT NOT NULL,
    lane_span_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    runtime_binding TEXT NOT NULL,
    launch_authority TEXT NOT NULL,
    status TEXT NOT NULL,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT NOT NULL,
    task_board_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    event_ledger_stream_id TEXT NOT NULL,
    event_ledger_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL,
    record_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_model_lanes_record_object
        CHECK (jsonb_typeof(record_json) = 'object')
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lanes_event_seq
    ON model_lanes(event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lanes_stream_replay
    ON model_lanes(event_ledger_stream_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lanes_run_replay
    ON model_lanes(run_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lanes_locus
    ON model_lanes(work_packet_id, micro_task_id, owner_session);

CREATE TABLE IF NOT EXISTS model_lane_messages (
    message_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL
        REFERENCES model_lane_runs(run_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    trace_id TEXT NOT NULL,
    message_span_id TEXT NOT NULL,
    from_lane_id TEXT NOT NULL
        REFERENCES model_lanes(lane_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    coordinator_session_id TEXT NOT NULL,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT NOT NULL,
    task_board_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    payload_sha256 TEXT NOT NULL,
    replay_order_key TEXT NOT NULL,
    authority TEXT NOT NULL,
    event_ledger_stream_id TEXT NOT NULL,
    event_ledger_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL,
    event_stream_version BIGINT NOT NULL,
    transaction_seq BIGINT NOT NULL,
    record_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_model_lane_messages_hash
        CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_messages_record_object
        CHECK (jsonb_typeof(record_json) = 'object')
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lane_messages_event_seq
    ON model_lane_messages(event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_messages_stream_replay
    ON model_lane_messages(event_ledger_stream_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_messages_run_replay
    ON model_lane_messages(run_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_messages_payload_hash
    ON model_lane_messages(payload_sha256);

CREATE INDEX IF NOT EXISTS idx_model_lane_messages_locus
    ON model_lane_messages(work_packet_id, micro_task_id, task_board_id, owner_session);

CREATE INDEX IF NOT EXISTS idx_model_lane_messages_record_gin
    ON model_lane_messages USING GIN (record_json);
