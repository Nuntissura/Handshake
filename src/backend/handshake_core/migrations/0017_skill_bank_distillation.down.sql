-- WP-1-Distillation-v2 MT-002 rollback

DROP VIEW IF EXISTS replay_candidates;
DROP TABLE IF EXISTS eval_run;
DROP TABLE IF EXISTS adapter_checkpoint;
DROP TABLE IF EXISTS distill_example;
DROP TABLE IF EXISTS distill_job;
DROP INDEX IF EXISTS idx_skill_log_file_ref_log;
DROP TABLE IF EXISTS skill_log_file_ref;
DROP INDEX IF EXISTS idx_skill_log_entry_task_type;
DROP INDEX IF EXISTS idx_skill_log_entry_privacy;
DROP INDEX IF EXISTS idx_skill_log_entry_quality;
DROP INDEX IF EXISTS idx_skill_log_entry_session;
DROP TABLE IF EXISTS skill_log_entry;
