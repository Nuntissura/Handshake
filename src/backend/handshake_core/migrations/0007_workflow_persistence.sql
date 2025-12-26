-- Add heartbeat tracking for workflow runs (crash recovery)
ALTER TABLE workflow_runs
ADD COLUMN last_heartbeat TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP;

-- Durable per-node execution history
CREATE TABLE workflow_node_executions (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_run_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    node_type TEXT NOT NULL,
    status TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    input_payload TEXT NULL,
    output_payload TEXT NULL,
    error_message TEXT NULL,
    started_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (workflow_run_id) REFERENCES workflow_runs(id) ON DELETE CASCADE
);

CREATE INDEX idx_wne_run_sequence ON workflow_node_executions (workflow_run_id, sequence);
CREATE INDEX idx_wne_run_node ON workflow_node_executions (workflow_run_id, node_id);
CREATE INDEX idx_wne_status ON workflow_node_executions (status);
