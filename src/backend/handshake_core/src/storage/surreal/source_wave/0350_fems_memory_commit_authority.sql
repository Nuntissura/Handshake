-- WP-KERNEL-012 MT-065 remediation: approved FEMS proposals commit through one
-- canonical PostgreSQL authority transaction with an immutable commit report.

CREATE TABLE IF NOT EXISTS fems_memory_commit_reports (
    commit_id       TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL,
    proposal_id     TEXT NOT NULL UNIQUE,
    memory_id       TEXT NOT NULL,
    report          JSONB NOT NULL,
    report_hash     TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL,
    CONSTRAINT fk_fems_memory_commit_reports_workspace
        FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    CONSTRAINT fk_fems_memory_commit_reports_proposal
        FOREIGN KEY (proposal_id) REFERENCES fems_memory_proposals(proposal_id) ON DELETE CASCADE,
    CONSTRAINT fk_fems_memory_commit_reports_item
        FOREIGN KEY (memory_id) REFERENCES fems_memory_items(memory_id) ON DELETE RESTRICT,
    CONSTRAINT chk_fems_memory_commit_reports_hash
        CHECK (report_hash ~ '^[0-9a-f]{64}$')
);

CREATE INDEX IF NOT EXISTS idx_fems_memory_commit_reports_workspace_created
    ON fems_memory_commit_reports (workspace_id, created_at DESC, commit_id DESC);

