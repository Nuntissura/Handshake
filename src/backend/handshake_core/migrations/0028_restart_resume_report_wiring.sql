-- WP-KERNEL-004 MT-193:
-- Extend restart-resume report persistence to cover the full runtime report.

ALTER TABLE kernel_restart_resume_report
    ADD COLUMN IF NOT EXISTS orphan_reclaims JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS operator_decision_requests JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS fr_events_emitted JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS schema_version INTEGER NOT NULL DEFAULT 2;

CREATE INDEX IF NOT EXISTS idx_kernel_restart_resume_report_started
    ON kernel_restart_resume_report (started_at_utc DESC);

