-- Best-effort rollback.
--
-- Reverts micro_task_execution back to workflow_run for the micro_task_executor_v1 profile.
-- Note: the original protocol_id value (if any) is not recoverable; this migration leaves protocol_id unchanged.

UPDATE ai_jobs
SET job_kind = 'workflow_run'
WHERE job_kind = 'micro_task_execution'
  AND profile_id = 'micro_task_executor_v1';

