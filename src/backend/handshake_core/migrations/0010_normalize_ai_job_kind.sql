-- Normalize legacy ai_jobs.job_kind strings to canonical Master Spec v02.113 values.
--
-- - term_exec -> terminal_exec (alias normalization on write)
-- - doc_test -> doc_summarize (remove non-spec persisted kind)
-- - governance_pack_export -> workflow_run (remove non-spec persisted kind)
--   + ensure protocol_id is set for deterministic dispatch

UPDATE ai_jobs
SET job_kind = 'terminal_exec'
WHERE job_kind = 'term_exec';

UPDATE ai_jobs
SET job_kind = 'doc_summarize'
WHERE job_kind = 'doc_test';

UPDATE ai_jobs
SET job_kind = 'workflow_run',
    protocol_id = 'hsk.governance_pack.export.v0'
WHERE job_kind = 'governance_pack_export';

