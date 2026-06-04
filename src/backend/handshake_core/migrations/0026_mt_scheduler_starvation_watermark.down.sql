DROP INDEX IF EXISTS idx_kernel_micro_task_job_wp_claimed_at;
ALTER TABLE kernel_micro_task_job
    DROP COLUMN IF EXISTS starvation_watermark_at_utc;
