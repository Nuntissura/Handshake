CREATE TABLE model_lane_routing_executions (
    execution_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES model_lane_runs(run_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    selecting_decision_id TEXT NOT NULL REFERENCES model_lane_promotion_decisions(decision_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    selecting_decision_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    selecting_decision_event_seq BIGINT NOT NULL CHECK (selecting_decision_event_seq > 0),
    trace_id TEXT NOT NULL,
    run_span_id TEXT NOT NULL,
    coordinator_session_id TEXT NOT NULL,
    locus_ref TEXT NOT NULL,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT,
    task_board_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    initial_input_ref TEXT NOT NULL,
    initial_input_sha256 TEXT NOT NULL CHECK (initial_input_sha256 ~ '^[0-9a-f]{64}$'),
    graph_sha256 TEXT NOT NULL CHECK (length(graph_sha256) = 64),
    status TEXT NOT NULL CHECK (status IN ('running','awaiting_authority','succeeded','failed','cancelled')),
    revision BIGINT NOT NULL CHECK (revision >= 0),
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL CHECK (event_ledger_seq > 0),
    record_json JSONB NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL CHECK (updated_at_unix_ms >= 0)
);

CREATE INDEX model_lane_routing_executions_run_idx
    ON model_lane_routing_executions (run_id, status);
CREATE INDEX model_lane_routing_executions_trace_idx
    ON model_lane_routing_executions (trace_id, event_ledger_seq);

CREATE TABLE model_lane_routing_stage_attempts (
    execution_id TEXT NOT NULL REFERENCES model_lane_routing_executions(execution_id) ON DELETE RESTRICT,
    stage_id TEXT NOT NULL,
    attempt BIGINT NOT NULL CHECK (attempt > 0),
    dispatch_target TEXT NOT NULL CHECK (dispatch_target IN ('local_model','cloud_model','validator','operator','coordinator_join')),
    expected_run_id TEXT NOT NULL,
    expected_lane_id TEXT NOT NULL,
    expected_model_id TEXT NOT NULL,
    expected_provider TEXT,
    status TEXT NOT NULL CHECK (status IN ('scheduled','claimed','in_flight','awaiting_authority','succeeded','failed','joined','cancelled','compensated')),
    run_id TEXT NOT NULL,
    trace_id TEXT NOT NULL,
    locus_ref TEXT NOT NULL,
    authority_ref TEXT,
    input_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
    output_ref TEXT,
    output_message_ref TEXT,
    authority_request_message_ref TEXT,
    fencing_token TEXT,
    lease_owner TEXT,
    lease_expires_at_unix_ms BIGINT,
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL CHECK (event_ledger_seq > 0),
    record_json JSONB NOT NULL,
    updated_at_unix_ms BIGINT NOT NULL CHECK (updated_at_unix_ms >= 0),
    PRIMARY KEY (execution_id, stage_id, attempt),
    CHECK ((lease_owner IS NULL) = (lease_expires_at_unix_ms IS NULL)),
    CHECK ((output_ref IS NULL) = (output_message_ref IS NULL)),
    CHECK ((lease_owner IS NULL) = (fencing_token IS NULL))
);

CREATE INDEX model_lane_routing_stage_attempts_lease_idx
    ON model_lane_routing_stage_attempts (status, lease_expires_at_unix_ms)
    WHERE status IN ('claimed','in_flight');
CREATE INDEX model_lane_routing_stage_attempts_run_idx
    ON model_lane_routing_stage_attempts (run_id, stage_id, attempt);

CREATE TABLE model_lane_routing_outbox (
    command_id TEXT PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    execution_id TEXT NOT NULL REFERENCES model_lane_routing_executions(execution_id) ON DELETE RESTRICT,
    stage_id TEXT NOT NULL,
    attempt BIGINT NOT NULL CHECK (attempt > 0),
    dispatch_target TEXT NOT NULL CHECK (dispatch_target IN ('local_model','cloud_model','validator','operator','coordinator_join')),
    status TEXT NOT NULL CHECK (status IN ('pending','claimed','acked','dead_letter','cancelled','compensated')),
    command_json JSONB NOT NULL,
    fencing_token TEXT,
    lease_owner TEXT,
    lease_expires_at_unix_ms BIGINT,
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL CHECK (event_ledger_seq > 0),
    created_at_unix_ms BIGINT NOT NULL CHECK (created_at_unix_ms >= 0),
    updated_at_unix_ms BIGINT NOT NULL CHECK (updated_at_unix_ms >= 0),
    FOREIGN KEY (execution_id, stage_id, attempt)
        REFERENCES model_lane_routing_stage_attempts(execution_id, stage_id, attempt) ON DELETE RESTRICT,
    CHECK ((lease_owner IS NULL) = (lease_expires_at_unix_ms IS NULL)),
    CHECK ((lease_owner IS NULL) = (fencing_token IS NULL))
);

CREATE INDEX model_lane_routing_outbox_claim_idx
    ON model_lane_routing_outbox (status, created_at_unix_ms, command_id)
    WHERE status IN ('pending','claimed');

INSERT INTO model_lane_schema_registry (schema_id, schema_version, record_kind, table_name)
VALUES
('hsk.model_lane_routing_execution@5', 5, 'ModelLaneRoutingExecution', 'model_lane_routing_executions'),
('hsk.model_lane_routing_stage_attempt@4', 4, 'ModelLaneRoutingStageAttempt', 'model_lane_routing_stage_attempts'),
('hsk.model_lane_routing_outbox@4', 4, 'ModelLaneRoutingOutbox', 'model_lane_routing_outbox'),
('hsk.model_lane_run_extension@1', 1, 'ModelLaneRunExtension', 'model_lane_runs')
ON CONFLICT (schema_id) DO UPDATE SET
    schema_version = EXCLUDED.schema_version,
    record_kind = EXCLUDED.record_kind,
    table_name = EXCLUDED.table_name,
    source_component = EXCLUDED.source_component;
