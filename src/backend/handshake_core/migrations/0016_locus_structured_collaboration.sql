-- WP-1-Postgres-Structured-Collaboration-Artifact-Parity-v1:
-- bounded structured-collaboration schema coverage for migration portability

CREATE TABLE IF NOT EXISTS work_packets (
    wp_id TEXT PRIMARY KEY,
    version BIGINT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL,
    priority BIGINT NOT NULL,
    phase TEXT,
    routing TEXT,
    task_packet_path TEXT,
    task_board_status TEXT NOT NULL,
    assignee TEXT,
    reporter TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    vector_clock TEXT NOT NULL,
    metadata TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_wp_status ON work_packets(status);
CREATE INDEX IF NOT EXISTS idx_wp_priority ON work_packets(priority);
CREATE INDEX IF NOT EXISTS idx_wp_task_board_status ON work_packets(task_board_status);

CREATE TABLE IF NOT EXISTS micro_tasks (
    mt_id TEXT PRIMARY KEY,
    wp_id TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    current_iteration BIGINT,
    escalation_level BIGINT,
    metadata TEXT NOT NULL,
    FOREIGN KEY (wp_id) REFERENCES work_packets(wp_id) ON DELETE CASCADE
);
