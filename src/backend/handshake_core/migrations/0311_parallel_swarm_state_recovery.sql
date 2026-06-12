-- WP-KERNEL-009 MT-209..216 ParallelSwarmStateRecovery backend foundations.
-- PostgreSQL/EventLedger authority for agent lanes, worktree/workspace claims,
-- mailbox handoff refs, compaction checkpoints, recovery receipts, and the
-- parallel indexing lease queue.

CREATE TABLE IF NOT EXISTS knowledge_agent_worktree_claims (
    claim_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    wp_id TEXT NOT NULL,
    mt_id TEXT,
    scope_kind TEXT NOT NULL CHECK (scope_kind IN ('worktree', 'workspace', 'index_run')),
    scope_id TEXT NOT NULL,
    lane_id TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    lane_kind TEXT NOT NULL,
    attribution_jsonb JSONB NOT NULL,
    session_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'released', 'reclaimed')),
    reason TEXT NOT NULL,
    claimed_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at_utc TIMESTAMPTZ NOT NULL,
    released_at_utc TIMESTAMPTZ,
    event_ledger_event_id TEXT REFERENCES kernel_event_ledger(event_id),
    release_event_ledger_event_id TEXT REFERENCES kernel_event_ledger(event_id),
    reclaim_event_ledger_event_id TEXT REFERENCES kernel_event_ledger(event_id),
    CONSTRAINT chk_agent_worktree_claims_id
        CHECK (claim_id LIKE 'PSR-CLAIM-%')
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_agent_worktree_claims_active_scope
    ON knowledge_agent_worktree_claims (scope_kind, scope_id)
    WHERE status = 'active' AND released_at_utc IS NULL;

CREATE INDEX IF NOT EXISTS idx_agent_worktree_claims_workspace_active
    ON knowledge_agent_worktree_claims (workspace_id, claimed_at_utc DESC)
    WHERE status = 'active' AND released_at_utc IS NULL;

CREATE TABLE IF NOT EXISTS knowledge_agent_role_mailbox_handoffs (
    handoff_id TEXT PRIMARY KEY,
    wp_id TEXT NOT NULL,
    mt_id TEXT NOT NULL,
    claim_id TEXT REFERENCES knowledge_agent_worktree_claims(claim_id),
    from_lane_id TEXT NOT NULL,
    from_actor_id TEXT NOT NULL,
    from_lane_kind TEXT NOT NULL,
    from_attribution_jsonb JSONB NOT NULL,
    to_role TEXT NOT NULL,
    mailbox_thread_id TEXT NOT NULL,
    mailbox_message_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('started', 'progress', 'blocked', 'pass', 'fail')),
    summary TEXT NOT NULL,
    body_sha256 TEXT NOT NULL CHECK (body_sha256 ~ '^[0-9a-f]{64}$'),
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id),
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_agent_role_mailbox_handoffs_id
        CHECK (handoff_id LIKE 'PSR-HANDOFF-%')
);

CREATE INDEX IF NOT EXISTS idx_agent_role_mailbox_handoffs_wp_mt
    ON knowledge_agent_role_mailbox_handoffs (wp_id, mt_id, created_at_utc DESC);

CREATE TABLE IF NOT EXISTS knowledge_agent_state_recovery_checkpoints (
    checkpoint_id TEXT PRIMARY KEY,
    lane_id TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    lane_kind TEXT NOT NULL,
    attribution_jsonb JSONB NOT NULL,
    session_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    wp_id TEXT NOT NULL,
    mt_id TEXT NOT NULL,
    claim_id TEXT REFERENCES knowledge_agent_worktree_claims(claim_id),
    mailbox_handoff_id TEXT REFERENCES knowledge_agent_role_mailbox_handoffs(handoff_id),
    navigation_command_id TEXT,
    resume_pointer_jsonb JSONB NOT NULL,
    touched_files_jsonb JSONB NOT NULL,
    tests_jsonb JSONB NOT NULL,
    hbr_rows_jsonb JSONB NOT NULL,
    next_step_context TEXT NOT NULL,
    payload_jsonb JSONB NOT NULL,
    payload_sha256 TEXT NOT NULL CHECK (payload_sha256 ~ '^[0-9a-f]{64}$'),
    compaction_reason TEXT NOT NULL,
    git_head TEXT NOT NULL,
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id),
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_agent_state_recovery_checkpoints_id
        CHECK (checkpoint_id LIKE 'PSR-CHKPT-%')
);

CREATE INDEX IF NOT EXISTS idx_agent_state_recovery_checkpoints_session
    ON knowledge_agent_state_recovery_checkpoints (session_id, created_at_utc DESC);

CREATE TABLE IF NOT EXISTS knowledge_agent_recovery_receipts (
    receipt_id TEXT PRIMARY KEY,
    checkpoint_id TEXT NOT NULL REFERENCES knowledge_agent_state_recovery_checkpoints(checkpoint_id),
    prior_session_id TEXT NOT NULL,
    new_session_id TEXT NOT NULL,
    new_lane_id TEXT NOT NULL,
    new_actor_id TEXT NOT NULL,
    new_lane_kind TEXT NOT NULL,
    new_attribution_jsonb JSONB NOT NULL,
    resume_pointer_jsonb JSONB NOT NULL,
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id),
    recovered_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_agent_recovery_receipts_id
        CHECK (receipt_id LIKE 'PSR-RECOVERY-%')
);

CREATE INDEX IF NOT EXISTS idx_agent_recovery_receipts_checkpoint
    ON knowledge_agent_recovery_receipts (checkpoint_id, recovered_at_utc DESC);

CREATE TABLE IF NOT EXISTS knowledge_parallel_indexing_lease_queue (
    lease_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    wp_id TEXT NOT NULL,
    mt_id TEXT NOT NULL,
    scope_kind TEXT NOT NULL CHECK (scope_kind IN ('worktree', 'workspace', 'index_run')),
    scope_id TEXT NOT NULL,
    lane_id TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    lane_kind TEXT NOT NULL,
    attribution_jsonb JSONB NOT NULL,
    session_id TEXT NOT NULL,
    index_run_id TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    ttl_seconds BIGINT NOT NULL CHECK (ttl_seconds > 0),
    status TEXT NOT NULL CHECK (status IN ('queued', 'acquired', 'completed', 'cancelled', 'reclaimed')),
    blocked_by_lease_id TEXT,
    enqueued_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    acquired_at_utc TIMESTAMPTZ,
    expires_at_utc TIMESTAMPTZ,
    completed_at_utc TIMESTAMPTZ,
    event_ledger_event_id TEXT REFERENCES kernel_event_ledger(event_id),
    CONSTRAINT chk_parallel_indexing_lease_queue_id
        CHECK (lease_id LIKE 'PSR-IDXLEASE-%')
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_parallel_indexing_lease_queue_active_scope
    ON knowledge_parallel_indexing_lease_queue (scope_kind, scope_id)
    WHERE status = 'acquired';

CREATE INDEX IF NOT EXISTS idx_parallel_indexing_lease_queue_queued
    ON knowledge_parallel_indexing_lease_queue (scope_kind, scope_id, priority DESC, enqueued_at_utc ASC)
    WHERE status = 'queued';

INSERT INTO knowledge_schema_registry (
    family_key, table_name, record_family, authority_class, migration_file, mt_id
) VALUES
    ('parallel_swarm_claims', 'knowledge_agent_worktree_claims', 'SwarmClaim',
     'support', '0311_parallel_swarm_state_recovery.sql', 'MT-210'),
    ('parallel_swarm_handoffs', 'knowledge_agent_role_mailbox_handoffs', 'SwarmHandoff',
     'support', '0311_parallel_swarm_state_recovery.sql', 'MT-211'),
    ('parallel_swarm_checkpoints', 'knowledge_agent_state_recovery_checkpoints', 'SwarmCheckpoint',
     'support', '0311_parallel_swarm_state_recovery.sql', 'MT-213'),
    ('parallel_swarm_recovery_receipts', 'knowledge_agent_recovery_receipts', 'SwarmRecoveryReceipt',
     'support', '0311_parallel_swarm_state_recovery.sql', 'MT-214'),
    ('parallel_indexing_lease_queue', 'knowledge_parallel_indexing_lease_queue', 'IndexingLease',
     'support', '0311_parallel_swarm_state_recovery.sql', 'MT-216')
ON CONFLICT (family_key) DO NOTHING;
