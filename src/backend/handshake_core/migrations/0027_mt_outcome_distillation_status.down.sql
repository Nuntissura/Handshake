-- WP-KERNEL-004 cluster X.2 MT-188 rollback.

DROP INDEX IF EXISTS idx_kernel_mt_outcome_unique_job_iteration;
DROP INDEX IF EXISTS idx_kernel_distillation_candidate_status_tier;
ALTER TABLE kernel_distillation_candidate DROP COLUMN IF EXISTS status;
