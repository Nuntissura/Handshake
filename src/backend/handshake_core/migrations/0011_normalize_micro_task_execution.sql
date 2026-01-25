-- Normalize legacy micro_task_executor_v1 jobs to JobKind micro_task_execution.
--
-- Contract (Handshake_Master_Spec_v02_116.md):
-- - job_kind == micro_task_execution <-> profile_id == micro_task_executor_v1 AND protocol_id == micro_task_executor_v1
--
-- This migration rewrites historical rows that persisted micro_task_executor_v1 jobs as job_kind=workflow_run.

UPDATE ai_jobs
SET job_kind = 'micro_task_execution',
    protocol_id = 'micro_task_executor_v1'
WHERE job_kind = 'workflow_run'
  AND profile_id = 'micro_task_executor_v1';

UPDATE ai_jobs
SET protocol_id = 'micro_task_executor_v1'
WHERE profile_id = 'micro_task_executor_v1'
  AND protocol_id <> 'micro_task_executor_v1';

