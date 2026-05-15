-- WP-KERNEL-002-CRDT-Workspace-Write-Box-Preuse-Hardening-v1:
-- Restart-replayable CRDT update and snapshot storage.

CREATE TABLE IF NOT EXISTS kernel_crdt_updates (
    schema_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    document_id TEXT NOT NULL,
    crdt_document_id TEXT NOT NULL,
    update_id TEXT NOT NULL,
    update_seq BIGINT NOT NULL,
    update_sha256 TEXT NOT NULL,
    update_bytes_ref TEXT NOT NULL,
    update_bytes BYTEA NOT NULL,
    actor_id TEXT NOT NULL,
    actor_kind TEXT NOT NULL,
    session_id TEXT NOT NULL,
    trace_id TEXT NOT NULL,
    state_vector_before TEXT NOT NULL,
    state_vector_after TEXT NOT NULL,
    replay_metadata_json JSONB NOT NULL,
    event_ledger_stream_id TEXT NOT NULL,
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    storage_authority TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (workspace_id, document_id, crdt_document_id, update_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_kernel_crdt_updates_seq
    ON kernel_crdt_updates (workspace_id, document_id, crdt_document_id, update_seq);

CREATE UNIQUE INDEX IF NOT EXISTS idx_kernel_crdt_updates_event
    ON kernel_crdt_updates (event_ledger_stream_id, event_ledger_event_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_kernel_crdt_updates_bytes_ref
    ON kernel_crdt_updates (update_bytes_ref);

CREATE INDEX IF NOT EXISTS idx_kernel_crdt_updates_replay
    ON kernel_crdt_updates (workspace_id, document_id, crdt_document_id, update_seq);

CREATE INDEX IF NOT EXISTS idx_kernel_crdt_updates_state_vector_after
    ON kernel_crdt_updates (state_vector_after);

CREATE TABLE IF NOT EXISTS kernel_crdt_snapshots (
    schema_id TEXT NOT NULL,
    snapshot_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    document_id TEXT NOT NULL,
    crdt_document_id TEXT NOT NULL,
    covered_update_seq BIGINT NOT NULL,
    state_vector TEXT NOT NULL,
    snapshot_sha256 TEXT NOT NULL,
    snapshot_bytes_ref TEXT NOT NULL,
    snapshot_bytes BYTEA NOT NULL,
    actor_id TEXT NOT NULL,
    actor_kind TEXT NOT NULL,
    event_ledger_stream_id TEXT NOT NULL,
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    promotion_evidence_update_ids JSONB NOT NULL,
    storage_authority TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (workspace_id, document_id, crdt_document_id, snapshot_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_kernel_crdt_snapshots_event
    ON kernel_crdt_snapshots (event_ledger_stream_id, event_ledger_event_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_kernel_crdt_snapshots_bytes_ref
    ON kernel_crdt_snapshots (snapshot_bytes_ref);

CREATE INDEX IF NOT EXISTS idx_kernel_crdt_snapshots_latest
    ON kernel_crdt_snapshots (workspace_id, document_id, crdt_document_id, covered_update_seq DESC);

CREATE INDEX IF NOT EXISTS idx_kernel_crdt_snapshots_state_vector
    ON kernel_crdt_snapshots (state_vector);
