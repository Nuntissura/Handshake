-- WP-KERNEL-012 MT-066/074: harden Stage capture into an exact-byte,
-- idempotent, auditable runtime operation.

ALTER TABLE stage_capture_artifacts
    ADD COLUMN IF NOT EXISTS content_bytes BYTEA,
    ADD COLUMN IF NOT EXISTS size_bytes BIGINT,
    ADD COLUMN IF NOT EXISTS idempotency_key TEXT,
    ADD COLUMN IF NOT EXISTS request_hash TEXT,
    ADD COLUMN IF NOT EXISTS actor_kind TEXT,
    ADD COLUMN IF NOT EXISTS actor_id TEXT,
    ADD COLUMN IF NOT EXISTS correlation_id TEXT,
    ADD COLUMN IF NOT EXISTS approval_id TEXT,
    ADD COLUMN IF NOT EXISTS job_id TEXT,
    ADD COLUMN IF NOT EXISTS event_ledger_event_id TEXT;

-- Legacy 0341 rows were metadata-only. Preserve them while making every new
-- row exact-byte capable; their canonical JSON serialization is the only
-- recoverable byte representation.
UPDATE stage_capture_artifacts
SET content_bytes = convert_to(content_json::text, 'UTF8')
WHERE content_bytes IS NULL;

UPDATE stage_capture_artifacts
SET size_bytes = octet_length(content_bytes),
    idempotency_key = COALESCE(NULLIF(idempotency_key, ''), 'legacy:' || artifact_id),
    request_hash = COALESCE(NULLIF(request_hash, ''), content_sha256),
    actor_kind = COALESCE(NULLIF(actor_kind, ''), 'system'),
    actor_id = COALESCE(NULLIF(actor_id, ''), 'stage-0341-migration'),
    correlation_id = COALESCE(NULLIF(correlation_id, ''), 'legacy:' || artifact_id),
    approval_id = COALESCE(NULLIF(approval_id, ''), 'legacy-pre-privileged-contract')
WHERE size_bytes IS NULL
   OR idempotency_key IS NULL
   OR request_hash IS NULL
   OR actor_kind IS NULL
   OR actor_id IS NULL
   OR correlation_id IS NULL
   OR approval_id IS NULL;

ALTER TABLE stage_capture_artifacts
    ALTER COLUMN content_bytes SET NOT NULL,
    ALTER COLUMN size_bytes SET NOT NULL,
    ALTER COLUMN idempotency_key SET NOT NULL,
    ALTER COLUMN request_hash SET NOT NULL,
    ALTER COLUMN actor_kind SET NOT NULL,
    ALTER COLUMN actor_id SET NOT NULL,
    ALTER COLUMN correlation_id SET NOT NULL,
    ALTER COLUMN approval_id SET NOT NULL;

ALTER TABLE stage_capture_artifacts
    DROP CONSTRAINT IF EXISTS stage_capture_artifacts_size_bytes_check,
    ADD CONSTRAINT stage_capture_artifacts_size_bytes_check
        CHECK (
            size_bytes >= 1
            AND (idempotency_key LIKE 'legacy:%' OR size_bytes <= 16384)
        ),
    DROP CONSTRAINT IF EXISTS stage_capture_artifacts_request_hash_check,
    ADD CONSTRAINT stage_capture_artifacts_request_hash_check
        CHECK (request_hash ~ '^[0-9a-f]{64}$'),
    DROP CONSTRAINT IF EXISTS stage_capture_artifacts_actor_kind_check,
    ADD CONSTRAINT stage_capture_artifacts_actor_kind_check
        CHECK (actor_kind IN ('operator', 'system')),
    DROP CONSTRAINT IF EXISTS stage_capture_artifacts_exact_size_check,
    ADD CONSTRAINT stage_capture_artifacts_exact_size_check
        CHECK (size_bytes = octet_length(content_bytes));

ALTER TABLE stage_capture_artifacts
    DROP CONSTRAINT IF EXISTS stage_capture_artifacts_job_fk,
    ADD CONSTRAINT stage_capture_artifacts_job_fk
        FOREIGN KEY (job_id) REFERENCES ai_jobs(id) ON DELETE SET NULL,
    DROP CONSTRAINT IF EXISTS stage_capture_artifacts_event_ledger_fk,
    ADD CONSTRAINT stage_capture_artifacts_event_ledger_fk
        FOREIGN KEY (event_ledger_event_id) REFERENCES kernel_event_ledger(event_id) ON DELETE SET NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_stage_capture_artifacts_idempotency
    ON stage_capture_artifacts (workspace_id, idempotency_key);

CREATE INDEX IF NOT EXISTS idx_stage_capture_artifacts_job
    ON stage_capture_artifacts (job_id)
    WHERE job_id IS NOT NULL;
