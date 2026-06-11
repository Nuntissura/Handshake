-- WP-KERNEL-009 MT-122 ConflictDetectionAgentJob + MT-123 ConflictResolutionAgentJob.
-- Master Spec anchor: 02-system-architecture.md section 2.3.13.11 (KnowledgeClaim
-- conflict refs + resolution receipts) and the WP-009 contract field
-- `translated_memory_system_spec.agent_jobs`:
--   "ConflictDetectionJob: finds symbolic, temporal, alias, stale-source, and
--    semantic conflicts."
--   "ConflictResolutionJob: discards, refines, merges, temporal-qualifies,
--    granularity-qualifies, or promotes claims with receipts."
--
-- IMPORTANT (per WP-009 build notes): a conflict-detection/resolution "agent
-- job" here is a TYPED JOB RECORD plus the deterministic detection/resolution
-- logic. It is NOT a real spawned LLM agent. The job row captures what was
-- scanned, what conflicts were found (linking the existing
-- knowledge_claim_conflicts rows from 0137), and — for resolution — the chosen
-- outcome and its EventLedger receipt. LLM execution is a runtime concern
-- handled elsewhere; this layer records the job and the deterministic result.
--
-- The job rows REUSE the committed conflict substrate: detection links the
-- knowledge_claim_conflicts it produced; resolution names the conflict it
-- resolved, the outcome, and the resolution receipt (the same receipt the
-- 0137 knowledge_claim_conflicts.resolution_receipt_event_id carries).

CREATE TABLE IF NOT EXISTS knowledge_memory_conflict_detection_jobs (
    job_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- The conflict classes this detection pass searched for.
    detection_kind TEXT NOT NULL CHECK (detection_kind IN (
        'symbolic', 'temporal', 'alias', 'stale_source', 'granularity', 'semantic'
    )),
    -- Job lifecycle: queued -> running -> completed | failed.
    job_state TEXT NOT NULL DEFAULT 'completed'
        CHECK (job_state IN ('queued', 'running', 'completed', 'failed')),
    -- How many claims/facts the pass scanned and how many conflict pairs it
    -- recorded (accounting for the deterministic search).
    candidates_scanned INTEGER NOT NULL DEFAULT 0 CHECK (candidates_scanned >= 0),
    conflicts_found INTEGER NOT NULL DEFAULT 0 CHECK (conflicts_found >= 0),
    -- The deterministic search parameters (subject/predicate keys, alias keys),
    -- recorded so the pass is replayable.
    search_parameters JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- EventLedger receipt for the detection pass.
    detection_receipt_event_id TEXT
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    CONSTRAINT chk_kmcdj_id CHECK (job_id ~ '^KCDJ-[0-9a-f]{32}$'),
    CONSTRAINT chk_kmcdj_completed_shape
        CHECK (job_state <> 'completed' OR completed_at IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_kmcdj_workspace
    ON knowledge_memory_conflict_detection_jobs (workspace_id, detection_kind);

-- Which knowledge_claim_conflicts a detection job produced (job -> conflicts).
CREATE TABLE IF NOT EXISTS knowledge_memory_conflict_detection_findings (
    job_id TEXT NOT NULL
        REFERENCES knowledge_memory_conflict_detection_jobs(job_id) ON DELETE CASCADE,
    conflict_id TEXT NOT NULL
        REFERENCES knowledge_claim_conflicts(conflict_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (job_id, conflict_id)
);

CREATE INDEX IF NOT EXISTS idx_kmcdf_conflict
    ON knowledge_memory_conflict_detection_findings (conflict_id);

CREATE TABLE IF NOT EXISTS knowledge_memory_conflict_resolution_jobs (
    job_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- The conflict this resolution job acted on.
    conflict_id TEXT NOT NULL
        REFERENCES knowledge_claim_conflicts(conflict_id) ON DELETE CASCADE,
    -- The chosen resolution outcome (translated-spec ConflictResolutionJob).
    outcome TEXT NOT NULL CHECK (outcome IN (
        'discard', 'refine', 'temporal_qualify', 'granularity_qualify', 'merge'
    )),
    -- The claim that "won" / was kept (for discard/merge) or refined.
    kept_claim_id TEXT
        REFERENCES knowledge_claims(claim_id) ON DELETE SET NULL,
    -- The claim that was discarded / superseded (for discard/merge).
    discarded_claim_id TEXT
        REFERENCES knowledge_claims(claim_id) ON DELETE SET NULL,
    -- Structured notes about the resolution (qualifier values applied, merge
    -- target, refine diff).
    resolution_detail JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- REQUIRED EventLedger receipt: a resolution is receipt-backed (spec).
    resolution_receipt_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id) ON UPDATE RESTRICT ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_kmcrj_id CHECK (job_id ~ '^KCRJ-[0-9a-f]{32}$'),
    -- discard/merge name both a kept and a discarded claim; the qualify/refine
    -- outcomes name (at least) the kept claim.
    CONSTRAINT chk_kmcrj_claim_shape CHECK (
        (outcome IN ('discard', 'merge')
            AND kept_claim_id IS NOT NULL AND discarded_claim_id IS NOT NULL)
        OR (outcome IN ('refine', 'temporal_qualify', 'granularity_qualify')
            AND kept_claim_id IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_kmcrj_workspace
    ON knowledge_memory_conflict_resolution_jobs (workspace_id);

CREATE INDEX IF NOT EXISTS idx_kmcrj_conflict
    ON knowledge_memory_conflict_resolution_jobs (conflict_id);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('memory_conflict_detection_jobs', 'knowledge_memory_conflict_detection_jobs',
     'ConflictDetectionJob', 'authority', '0242_knowledge_memory_conflict_jobs.sql', 'MT-122'),
    ('memory_conflict_detection_findings', 'knowledge_memory_conflict_detection_findings',
     'ConflictDetectionJob', 'authority', '0242_knowledge_memory_conflict_jobs.sql', 'MT-122'),
    ('memory_conflict_resolution_jobs', 'knowledge_memory_conflict_resolution_jobs',
     'ConflictResolutionJob', 'authority', '0242_knowledge_memory_conflict_jobs.sql', 'MT-123')
ON CONFLICT (family_key) DO NOTHING;
