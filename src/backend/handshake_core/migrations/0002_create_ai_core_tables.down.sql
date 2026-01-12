-- Rollback AI core tables (dev/test/CI only)

DROP TABLE IF EXISTS workflow_runs;
DROP TABLE IF EXISTS ai_jobs;
