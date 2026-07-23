DROP INDEX IF EXISTS idx_stage_capture_artifacts_job;
DROP INDEX IF EXISTS idx_stage_capture_artifacts_idempotency;

ALTER TABLE stage_capture_artifacts
    DROP CONSTRAINT IF EXISTS stage_capture_artifacts_event_ledger_fk,
    DROP CONSTRAINT IF EXISTS stage_capture_artifacts_job_fk,
    DROP CONSTRAINT IF EXISTS stage_capture_artifacts_exact_size_check,
    DROP CONSTRAINT IF EXISTS stage_capture_artifacts_actor_kind_check,
    DROP CONSTRAINT IF EXISTS stage_capture_artifacts_request_hash_check,
    DROP CONSTRAINT IF EXISTS stage_capture_artifacts_size_bytes_check,
    DROP COLUMN IF EXISTS event_ledger_event_id,
    DROP COLUMN IF EXISTS job_id,
    DROP COLUMN IF EXISTS approval_id,
    DROP COLUMN IF EXISTS correlation_id,
    DROP COLUMN IF EXISTS actor_id,
    DROP COLUMN IF EXISTS actor_kind,
    DROP COLUMN IF EXISTS request_hash,
    DROP COLUMN IF EXISTS idempotency_key,
    DROP COLUMN IF EXISTS size_bytes,
    DROP COLUMN IF EXISTS content_bytes;
