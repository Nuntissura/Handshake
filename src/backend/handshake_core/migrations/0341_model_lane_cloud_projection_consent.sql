-- WP-1 MT-006: Dexterity cloud ProjectionPlan and ConsentReceipt authority.
--
-- Cloud launch permission is durable PostgreSQL/EventLedger state before a
-- provider call exists. These rows are intentionally not FK-bound to
-- model_lanes: they must be recordable before launch and usable to deny launch
-- without creating partial lane authority.

INSERT INTO model_lane_schema_registry
    (schema_id, schema_version, record_kind, table_name)
VALUES
    ('hsk.model_lane_cloud_projection_plan@1', 1, 'ModelLaneCloudProjectionPlan', 'model_lane_cloud_projection_plans'),
    ('hsk.model_lane_cloud_consent_receipt@1', 1, 'ModelLaneCloudConsentReceipt', 'model_lane_cloud_consent_receipts'),
    ('hsk.model_lane_cloud_consent_denial@1', 1, 'ModelLaneCloudConsentDenial', 'kernel_event_ledger')
ON CONFLICT (schema_id) DO UPDATE SET
    schema_version = EXCLUDED.schema_version,
    record_kind = EXCLUDED.record_kind,
    table_name = EXCLUDED.table_name,
    source_component = EXCLUDED.source_component;

CREATE TABLE IF NOT EXISTS model_lane_cloud_projection_plans (
    projection_plan_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    trace_id TEXT NOT NULL,
    lane_id TEXT NOT NULL,
    model_session_id TEXT NOT NULL,
    provider_kind TEXT NOT NULL,
    requested_model_id TEXT NOT NULL,
    scope_hash TEXT NOT NULL,
    source_artifact_refs JSONB NOT NULL,
    payload_artifact_ref TEXT NOT NULL,
    payload_sha256 TEXT NOT NULL,
    redaction_policy_ref TEXT NOT NULL,
    redaction_summary TEXT NOT NULL,
    retention_policy TEXT NOT NULL,
    export_posture TEXT NOT NULL,
    provider_profile_ref TEXT NOT NULL,
    fan_out_targets JSONB NOT NULL,
    consent_scope TEXT NOT NULL,
    status TEXT NOT NULL,
    event_ledger_stream_id TEXT NOT NULL,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT NOT NULL,
    task_board_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    created_at_utc TEXT NOT NULL,
    user_manual_behavior_ref TEXT NOT NULL,
    diagnostic_payload JSONB NOT NULL,
    projection_plan_hash TEXT NOT NULL,
    event_ledger_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL,
    event_stream_version BIGINT NOT NULL,
    transaction_seq BIGINT NOT NULL,
    record_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_model_lane_cloud_projection_plans_scope_hash
        CHECK (scope_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_cloud_projection_plans_payload_hash
        CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_cloud_projection_plans_hash
        CHECK (projection_plan_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_cloud_projection_plans_source_refs
        CHECK (jsonb_typeof(source_artifact_refs) = 'array'),
    CONSTRAINT chk_model_lane_cloud_projection_plans_fanout
        CHECK (jsonb_typeof(fan_out_targets) = 'array'),
    CONSTRAINT chk_model_lane_cloud_projection_plans_diag
        CHECK (jsonb_typeof(diagnostic_payload) = 'object'),
    CONSTRAINT chk_model_lane_cloud_projection_plans_record
        CHECK (jsonb_typeof(record_json) = 'object'),
    CONSTRAINT chk_model_lane_cloud_projection_plans_status
        CHECK (status IN ('active', 'superseded')),
    CONSTRAINT uq_model_lane_cloud_projection_plans_lane
        UNIQUE (run_id, lane_id, model_session_id, provider_kind)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lane_cloud_projection_plans_event_seq
    ON model_lane_cloud_projection_plans(event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_cloud_projection_plans_stream_replay
    ON model_lane_cloud_projection_plans(event_ledger_stream_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_cloud_projection_plans_run_replay
    ON model_lane_cloud_projection_plans(run_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_cloud_projection_plans_locus
    ON model_lane_cloud_projection_plans(work_packet_id, micro_task_id, task_board_id, owner_session);

CREATE INDEX IF NOT EXISTS idx_model_lane_cloud_projection_plans_record_gin
    ON model_lane_cloud_projection_plans USING GIN (record_json);

CREATE TABLE IF NOT EXISTS model_lane_cloud_consent_receipts (
    consent_receipt_id TEXT PRIMARY KEY,
    projection_plan_id TEXT NOT NULL,
    projection_plan_hash TEXT NOT NULL,
    run_id TEXT NOT NULL,
    trace_id TEXT NOT NULL,
    lane_id TEXT NOT NULL,
    model_session_id TEXT NOT NULL,
    provider_kind TEXT NOT NULL,
    requested_model_id TEXT NOT NULL,
    scope_hash TEXT NOT NULL,
    consent_scope TEXT NOT NULL,
    retention_policy TEXT NOT NULL,
    export_posture TEXT NOT NULL,
    fan_out_targets JSONB NOT NULL,
    approved BOOLEAN NOT NULL,
    approved_by_ref TEXT NOT NULL,
    approved_at_utc TEXT NOT NULL,
    valid_from_utc TEXT NOT NULL,
    valid_until_utc TEXT NOT NULL,
    revoked_at_utc TEXT,
    revocation_ref TEXT,
    status TEXT NOT NULL,
    event_ledger_stream_id TEXT NOT NULL,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT NOT NULL,
    task_board_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    created_at_utc TEXT NOT NULL,
    user_manual_behavior_ref TEXT NOT NULL,
    diagnostic_payload JSONB NOT NULL,
    consent_receipt_hash TEXT NOT NULL,
    event_ledger_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL,
    event_stream_version BIGINT NOT NULL,
    transaction_seq BIGINT NOT NULL,
    record_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_model_lane_cloud_consent_receipts_projection_hash
        CHECK (projection_plan_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_cloud_consent_receipts_scope_hash
        CHECK (scope_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_cloud_consent_receipts_hash
        CHECK (consent_receipt_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_cloud_consent_receipts_fanout
        CHECK (jsonb_typeof(fan_out_targets) = 'array'),
    CONSTRAINT chk_model_lane_cloud_consent_receipts_diag
        CHECK (jsonb_typeof(diagnostic_payload) = 'object'),
    CONSTRAINT chk_model_lane_cloud_consent_receipts_record
        CHECK (jsonb_typeof(record_json) = 'object'),
    CONSTRAINT chk_model_lane_cloud_consent_receipts_status
        CHECK (status IN ('approved', 'revoked'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lane_cloud_consent_receipts_event_seq
    ON model_lane_cloud_consent_receipts(event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_cloud_consent_receipts_stream_replay
    ON model_lane_cloud_consent_receipts(event_ledger_stream_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_cloud_consent_receipts_run_replay
    ON model_lane_cloud_consent_receipts(run_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_cloud_consent_receipts_projection
    ON model_lane_cloud_consent_receipts(projection_plan_id, projection_plan_hash);

CREATE INDEX IF NOT EXISTS idx_model_lane_cloud_consent_receipts_locus
    ON model_lane_cloud_consent_receipts(work_packet_id, micro_task_id, task_board_id, owner_session);

CREATE INDEX IF NOT EXISTS idx_model_lane_cloud_consent_receipts_record_gin
    ON model_lane_cloud_consent_receipts USING GIN (record_json);
