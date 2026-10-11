-- WP-KERNEL-012 MT-027: crash-recoverable Flight Recorder publication for
-- saved Block Collection View mutations.

CREATE TABLE IF NOT EXISTS loom_block_view_fr_outbox (
    event_id       TEXT PRIMARY KEY,
    workspace_id   TEXT NOT NULL,
    block_id       TEXT NOT NULL,
    operation      TEXT NOT NULL,
    event          JSONB NOT NULL,
    event_hash     TEXT NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL,
    published_at   TIMESTAMPTZ,
    attempt_count  BIGINT NOT NULL DEFAULT 0,
    last_error     TEXT,
    last_error_at  TIMESTAMPTZ,
    quarantined_at TIMESTAMPTZ,
    CONSTRAINT fk_loom_block_view_fr_outbox_workspace
        FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    CONSTRAINT fk_loom_block_view_fr_outbox_block
        FOREIGN KEY (block_id) REFERENCES loom_blocks(block_id) ON DELETE CASCADE,
    CONSTRAINT chk_loom_block_view_fr_outbox_operation
        CHECK (operation IN ('create', 'update')),
    CONSTRAINT chk_loom_block_view_fr_outbox_hash
        CHECK (event_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_loom_block_view_fr_outbox_attempt_count
        CHECK (attempt_count >= 0),
    CONSTRAINT chk_loom_block_view_fr_outbox_published_after_create
        CHECK (published_at IS NULL OR published_at >= created_at),
    CONSTRAINT chk_loom_block_view_fr_outbox_quarantine_state
        CHECK (
            (quarantined_at IS NULL AND (last_error IS NULL OR last_error_at IS NOT NULL))
            OR
            (quarantined_at IS NOT NULL AND last_error IS NOT NULL AND last_error_at IS NOT NULL)
        )
);

CREATE INDEX IF NOT EXISTS idx_loom_block_view_fr_outbox_pending
    ON loom_block_view_fr_outbox (workspace_id, created_at, event_id)
    WHERE published_at IS NULL AND quarantined_at IS NULL;
