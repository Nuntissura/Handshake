-- WP-KERNEL-009 MT-079 CRDTAndConcurrencyCore-079-CrdtRecoveryReceiptFormat.
-- Contract source: MT-041 swarm_lease_checkpoint_contract_seed
-- (SwarmCheckpoint MUST-level semantics):
--   * a checkpoint MUST capture checkpoint_id, session_id, actor_id,
--     lane_id, lease_id, scope_ref, resume_pointer (typed), created_at_utc,
--     payload_hash;
--   * checkpoints MUST be durable PostgreSQL/EventLedger records; recovery
--     after session loss MUST be reconstructable from checkpoints plus
--     receipts alone, with no chat-history dependency;
--   * a recovery MUST emit a recovery receipt linking the new session to the
--     recovered checkpoint and lease lineage.

CREATE TABLE IF NOT EXISTS knowledge_crdt_swarm_checkpoints (
    checkpoint_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL CHECK (btrim(session_id) <> ''),
    actor_id TEXT NOT NULL CHECK (actor_id ~ '^(operator|local_model|cloud_model|validator|system):[A-Za-z0-9._-]+$'),
    lane_id TEXT NOT NULL CHECK (btrim(lane_id) <> ''),
    lease_id TEXT NOT NULL
        REFERENCES knowledge_crdt_agent_lane_leases(lease_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    scope_ref TEXT NOT NULL CHECK (btrim(scope_ref) <> ''),
    -- Typed resume pointer (serialized SwarmResumePointerV1: micro_task |
    -- claim | document_revision | index_run_position).
    resume_pointer JSONB NOT NULL,
    -- Free checkpoint payload (work-in-flight state) + its sha256.
    checkpoint_payload JSONB NOT NULL,
    payload_sha256 TEXT NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    recorded_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_crdt_swarm_checkpoints_id
        CHECK (checkpoint_id ~ '^[0-9A-HJKMNP-TV-Z]{26}$')
);

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_swarm_checkpoints_session
    ON knowledge_crdt_swarm_checkpoints (session_id, created_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_swarm_checkpoints_lane
    ON knowledge_crdt_swarm_checkpoints (lane_id, created_at_utc DESC);

CREATE TABLE IF NOT EXISTS knowledge_crdt_recovery_receipts (
    receipt_id TEXT PRIMARY KEY,
    checkpoint_id TEXT NOT NULL
        REFERENCES knowledge_crdt_swarm_checkpoints(checkpoint_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    prior_session_id TEXT NOT NULL CHECK (btrim(prior_session_id) <> ''),
    new_session_id TEXT NOT NULL CHECK (btrim(new_session_id) <> ''),
    new_actor_id TEXT NOT NULL CHECK (new_actor_id ~ '^(operator|local_model|cloud_model|validator|system):[A-Za-z0-9._-]+$'),
    -- Lease under which the recovered session continues (takeover or fresh).
    new_lease_id TEXT NOT NULL
        REFERENCES knowledge_crdt_agent_lane_leases(lease_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    -- Ordered lease lineage (JSONB array, newest first) from the new lease
    -- back to the checkpointed lease's root claim.
    lease_lineage JSONB NOT NULL CHECK (
        jsonb_typeof(lease_lineage) = 'array'
        AND jsonb_array_length(lease_lineage) >= 1
    ),
    resume_pointer JSONB NOT NULL,
    recorded_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_crdt_recovery_receipts_id
        CHECK (receipt_id ~ '^[0-9A-HJKMNP-TV-Z]{26}$')
);

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_recovery_receipts_checkpoint
    ON knowledge_crdt_recovery_receipts (checkpoint_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_recovery_receipts_new_session
    ON knowledge_crdt_recovery_receipts (new_session_id);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('crdt_swarm_checkpoints', 'knowledge_crdt_swarm_checkpoints', 'SwarmCheckpoint',
     'support', '0155_knowledge_crdt_swarm_checkpoints.sql', 'MT-079'),
    ('crdt_recovery_receipts', 'knowledge_crdt_recovery_receipts', 'CrdtRecoveryReceipt',
     'support', '0155_knowledge_crdt_swarm_checkpoints.sql', 'MT-079')
ON CONFLICT (family_key) DO NOTHING;
