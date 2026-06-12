-- WP-KERNEL-009 MT-219 Quiet mode background work.
-- Extends the parallel swarm state-recovery surface with durable quiet-work
-- receipts and persisted no-window/no-focus policy for indexing leases.

ALTER TABLE knowledge_parallel_indexing_lease_queue
    ADD COLUMN IF NOT EXISTS quiet_policy_jsonb JSONB NOT NULL DEFAULT
        '{"work_kind":"indexing","no_foreground_window":true,"no_focus_steal":true,"no_os_shell_window":true,"bounded":true,"observable":true}'::jsonb;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_constraint
         WHERE conname = 'chk_parallel_indexing_lease_queue_quiet_policy'
           AND conrelid = 'knowledge_parallel_indexing_lease_queue'::regclass
    ) THEN
        ALTER TABLE knowledge_parallel_indexing_lease_queue
            ADD CONSTRAINT chk_parallel_indexing_lease_queue_quiet_policy
            CHECK (
                quiet_policy_jsonb ->> 'work_kind' = 'indexing'
                AND (quiet_policy_jsonb ->> 'no_foreground_window')::boolean IS TRUE
                AND (quiet_policy_jsonb ->> 'no_focus_steal')::boolean IS TRUE
                AND (quiet_policy_jsonb ->> 'no_os_shell_window')::boolean IS TRUE
                AND (quiet_policy_jsonb ->> 'bounded')::boolean IS TRUE
                AND (quiet_policy_jsonb ->> 'observable')::boolean IS TRUE
            );
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS knowledge_agent_quiet_background_work (
    receipt_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    wp_id TEXT NOT NULL,
    mt_id TEXT NOT NULL,
    work_kind TEXT NOT NULL CHECK (work_kind IN ('indexing', 'backend_navigation', 'visual_capture', 'test_run')),
    subject_id TEXT NOT NULL,
    lane_id TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    lane_kind TEXT NOT NULL,
    attribution_jsonb JSONB NOT NULL,
    session_id TEXT NOT NULL,
    quiet_policy_jsonb JSONB NOT NULL,
    evidence_ref TEXT NOT NULL,
    event_ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id),
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_agent_quiet_background_work_id
        CHECK (receipt_id LIKE 'PSR-QUIET-%'),
    CONSTRAINT chk_agent_quiet_background_work_evidence_ref
        CHECK (length(btrim(evidence_ref)) > 0),
    CONSTRAINT chk_agent_quiet_background_work_policy
        CHECK (
            quiet_policy_jsonb ->> 'work_kind' = work_kind
            AND (quiet_policy_jsonb ->> 'no_foreground_window')::boolean IS TRUE
            AND (quiet_policy_jsonb ->> 'no_focus_steal')::boolean IS TRUE
            AND (quiet_policy_jsonb ->> 'no_os_shell_window')::boolean IS TRUE
            AND (quiet_policy_jsonb ->> 'bounded')::boolean IS TRUE
            AND (quiet_policy_jsonb ->> 'observable')::boolean IS TRUE
        )
);

CREATE INDEX IF NOT EXISTS idx_agent_quiet_background_work_workspace
    ON knowledge_agent_quiet_background_work (workspace_id, created_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_agent_quiet_background_work_subject
    ON knowledge_agent_quiet_background_work (subject_id, created_at_utc DESC);

INSERT INTO knowledge_schema_registry (
    family_key, table_name, record_family, authority_class, migration_file, mt_id
) VALUES
    ('parallel_swarm_quiet_background_work', 'knowledge_agent_quiet_background_work',
     'SwarmQuietBackgroundWork', 'support',
     '0313_parallel_swarm_quiet_background_work.sql', 'MT-219')
ON CONFLICT (family_key) DO NOTHING;
