-- WP-KERNEL-012 MT-065 validation-v2 remediation: make FR-EVT-MEM-003 a
-- crash-recoverable projection of the authoritative PostgreSQL commit.

CREATE TABLE IF NOT EXISTS fems_memory_commit_fr_outbox (
    event_id       TEXT PRIMARY KEY,
    workspace_id   TEXT NOT NULL,
    proposal_id    TEXT NOT NULL,
    commit_id      TEXT NOT NULL,
    event_code     TEXT NOT NULL,
    event          JSONB NOT NULL,
    event_hash     TEXT NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL,
    published_at   TIMESTAMPTZ,
    attempt_count  BIGINT NOT NULL DEFAULT 0,
    last_error     TEXT,
    last_error_at  TIMESTAMPTZ,
    quarantined_at TIMESTAMPTZ,
    CONSTRAINT uq_fems_memory_commit_fr_outbox_proposal_event
        UNIQUE (proposal_id, event_code),
    CONSTRAINT uq_fems_memory_commit_fr_outbox_commit_event
        UNIQUE (commit_id, event_code),
    CONSTRAINT fk_fems_memory_commit_fr_outbox_workspace
        FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    CONSTRAINT fk_fems_memory_commit_fr_outbox_proposal
        FOREIGN KEY (proposal_id) REFERENCES fems_memory_proposals(proposal_id) ON DELETE CASCADE,
    CONSTRAINT fk_fems_memory_commit_fr_outbox_commit
        FOREIGN KEY (commit_id) REFERENCES fems_memory_commit_reports(commit_id) ON DELETE CASCADE,
    CONSTRAINT chk_fems_memory_commit_fr_outbox_hash
        CHECK (event_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_fems_memory_commit_fr_outbox_event_code
        CHECK (event_code IN ('FR-EVT-MEM-003', 'FR-EVT-MEM-004')),
    CONSTRAINT chk_fems_memory_commit_fr_outbox_attempt_count
        CHECK (attempt_count >= 0),
    CONSTRAINT chk_fems_memory_commit_fr_outbox_published_after_create
        CHECK (published_at IS NULL OR published_at >= created_at),
    CONSTRAINT chk_fems_memory_commit_fr_outbox_quarantine_state
        CHECK (
            (quarantined_at IS NULL AND (last_error IS NULL OR last_error_at IS NOT NULL))
            OR
            (quarantined_at IS NOT NULL AND last_error IS NOT NULL AND last_error_at IS NOT NULL)
        )
);

CREATE INDEX IF NOT EXISTS idx_fems_memory_commit_fr_outbox_pending
    ON fems_memory_commit_fr_outbox (workspace_id, created_at, event_code, event_id)
    WHERE published_at IS NULL AND quarantined_at IS NULL;
