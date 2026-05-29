DROP INDEX IF EXISTS idx_kernel_restart_resume_report_started;

ALTER TABLE kernel_restart_resume_report
    DROP COLUMN IF EXISTS schema_version,
    DROP COLUMN IF EXISTS fr_events_emitted,
    DROP COLUMN IF EXISTS operator_decision_requests,
    DROP COLUMN IF EXISTS orphan_reclaims;
