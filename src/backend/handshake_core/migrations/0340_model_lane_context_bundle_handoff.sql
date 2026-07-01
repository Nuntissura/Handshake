-- WP-1 MT-005: Dexterity ContextBundle handoffs.
--
-- Model outputs move between lanes through replayable ContextBundle handoff
-- rows. Each handoff is backed by kernel_event_ledger; PostgreSQL is the
-- authority for restart/replay, artifact binding, and selection state.

INSERT INTO model_lane_schema_registry
    (schema_id, schema_version, record_kind, table_name)
VALUES
    ('hsk.model_lane_context_bundle_artifact@1', 1, 'ModelLaneContextBundleArtifactBinding', 'model_lane_context_bundle_artifacts'),
    ('hsk.model_lane_context_bundle_handoff@1', 1, 'ModelLaneContextBundleHandoff', 'model_lane_context_bundle_handoffs')
ON CONFLICT (schema_id) DO UPDATE SET
    schema_version = EXCLUDED.schema_version,
    record_kind = EXCLUDED.record_kind,
    table_name = EXCLUDED.table_name,
    source_component = EXCLUDED.source_component;

CREATE TABLE IF NOT EXISTS model_lane_context_bundle_artifacts (
    artifact_binding_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL
        REFERENCES model_lane_runs(run_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    trace_id TEXT NOT NULL,
    artifact_ref TEXT NOT NULL,
    artifact_sha256 TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    artifact_kind TEXT NOT NULL,
    artifact_manifest_ref TEXT NOT NULL,
    artifact_payload_ref TEXT NOT NULL,
    payload_json JSONB NOT NULL,
    event_ledger_stream_id TEXT NOT NULL,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT NOT NULL,
    task_board_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    artifact_binding_hash TEXT NOT NULL,
    event_ledger_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL,
    event_stream_version BIGINT NOT NULL,
    transaction_seq BIGINT NOT NULL,
    record_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_model_lane_context_bundle_artifacts_hash
        CHECK (artifact_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_context_bundle_artifacts_content_hash
        CHECK (content_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_context_bundle_artifacts_hashes_match
        CHECK (artifact_sha256 = content_hash),
    CONSTRAINT chk_model_lane_context_bundle_artifacts_binding_hash
        CHECK (artifact_binding_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_context_bundle_artifacts_payload_object
        CHECK (jsonb_typeof(payload_json) = 'object'),
    CONSTRAINT chk_model_lane_context_bundle_artifacts_record_object
        CHECK (jsonb_typeof(record_json) = 'object'),
    CONSTRAINT uq_model_lane_context_bundle_artifacts_ref
        UNIQUE (run_id, artifact_ref),
    CONSTRAINT uq_model_lane_context_bundle_artifacts_ref_hash
        UNIQUE (run_id, artifact_ref, artifact_sha256, content_hash)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lane_context_bundle_artifacts_event_seq
    ON model_lane_context_bundle_artifacts(event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_context_bundle_artifacts_stream_replay
    ON model_lane_context_bundle_artifacts(event_ledger_stream_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_context_bundle_artifacts_locus
    ON model_lane_context_bundle_artifacts(work_packet_id, micro_task_id, task_board_id, owner_session);

CREATE INDEX IF NOT EXISTS idx_model_lane_context_bundle_artifacts_record_gin
    ON model_lane_context_bundle_artifacts USING GIN (record_json);

CREATE TABLE IF NOT EXISTS model_lane_context_bundle_handoffs (
    handoff_id TEXT PRIMARY KEY,
    context_bundle_id TEXT NOT NULL,
    run_id TEXT NOT NULL
        REFERENCES model_lane_runs(run_id) ON UPDATE RESTRICT ON DELETE CASCADE,
    trace_id TEXT NOT NULL,
    handoff_span_id TEXT NOT NULL,
    downstream_lane_id TEXT NOT NULL
        REFERENCES model_lanes(lane_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    source_lane_id TEXT NOT NULL
        REFERENCES model_lanes(lane_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    source_message_id TEXT NOT NULL
        REFERENCES model_lane_messages(message_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    artifact_ref TEXT NOT NULL,
    artifact_sha256 TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    authority_state TEXT NOT NULL,
    selection_state TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    decision_ref TEXT,
    reviewer_ref TEXT,
    work_packet_id TEXT NOT NULL,
    micro_task_id TEXT NOT NULL,
    task_board_id TEXT NOT NULL,
    owner_session TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    context_bundle_hash TEXT NOT NULL,
    event_ledger_stream_id TEXT NOT NULL,
    event_ledger_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    event_ledger_seq BIGINT NOT NULL,
    event_stream_version BIGINT NOT NULL,
    transaction_seq BIGINT NOT NULL,
    record_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_model_lane_context_bundle_handoffs_artifact
        FOREIGN KEY (run_id, artifact_ref, artifact_sha256, content_hash)
        REFERENCES model_lane_context_bundle_artifacts(run_id, artifact_ref, artifact_sha256, content_hash)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    CONSTRAINT chk_model_lane_context_bundle_handoffs_artifact_hash
        CHECK (artifact_sha256 ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_context_bundle_handoffs_content_hash
        CHECK (content_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_context_bundle_handoffs_hashes_match
        CHECK (artifact_sha256 = content_hash),
    CONSTRAINT chk_model_lane_context_bundle_handoffs_context_hash
        CHECK (context_bundle_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_model_lane_context_bundle_handoffs_record_object
        CHECK (jsonb_typeof(record_json) = 'object'),
    CONSTRAINT chk_model_lane_context_bundle_handoffs_selection
        CHECK (selection_state IN ('selected', 'rejected', 'unresolved', 'superseded')),
    CONSTRAINT chk_model_lane_context_bundle_handoffs_source_kind
        CHECK (source_kind IN (
            'proposal',
            'critique',
            'tool_request',
            'tool_result',
            'status',
            'promotion_request',
            'recovery'
        )),
    CONSTRAINT chk_model_lane_context_bundle_handoffs_authority
        CHECK (authority_state IN (
            'advisory',
            'promotion_candidate',
            'promoted',
            'operator_decision',
            'validator_verdict'
        ))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_lane_context_bundle_handoffs_event_seq
    ON model_lane_context_bundle_handoffs(event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_context_bundle_handoffs_stream_replay
    ON model_lane_context_bundle_handoffs(event_ledger_stream_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_context_bundle_handoffs_bundle_replay
    ON model_lane_context_bundle_handoffs(run_id, context_bundle_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_context_bundle_handoffs_selection
    ON model_lane_context_bundle_handoffs(run_id, context_bundle_id, selection_state, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_context_bundle_handoffs_source_message
    ON model_lane_context_bundle_handoffs(run_id, source_message_id, event_ledger_seq);

CREATE INDEX IF NOT EXISTS idx_model_lane_context_bundle_handoffs_locus
    ON model_lane_context_bundle_handoffs(work_packet_id, micro_task_id, task_board_id, owner_session);

CREATE INDEX IF NOT EXISTS idx_model_lane_context_bundle_handoffs_record_gin
    ON model_lane_context_bundle_handoffs USING GIN (record_json);
