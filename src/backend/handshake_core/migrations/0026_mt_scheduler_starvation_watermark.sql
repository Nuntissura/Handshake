-- WP-KERNEL-004 cluster X.2 MT-187: persistent monotonic watermark for
-- StarvationGuard so a single (job_id, threshold-crossing) signal is emitted
-- at most once even across process restarts (red_team minimum_control #2:
-- 'Starvation metric is monotonic (no flapping) via last_emitted_at_utc
-- watermark per job').
--
-- The fairness_penalty key (per_wp_recent_claims) is computed from the
-- existing claimed_at_utc column on this same table (red_team minimum_control
-- #3: 'computed from kernel_micro_task_job table rows in last 60s window, not
-- from in-memory counter (survives process restart)') so no new column is
-- required for the fairness side; only the starvation watermark needs a
-- durable per-job slot.

ALTER TABLE kernel_micro_task_job
    ADD COLUMN IF NOT EXISTS starvation_watermark_at_utc TIMESTAMPTZ;

-- Speeds up the 60s claim window scan used by the fairness CTE in
-- FairScheduler::claim_next_priority.
CREATE INDEX IF NOT EXISTS idx_kernel_micro_task_job_wp_claimed_at
    ON kernel_micro_task_job (wp_id, claimed_at_utc DESC)
    WHERE claimed_at_utc IS NOT NULL;
