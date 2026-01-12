-- Migration: Add is_pinned column for retention policy support [HSK-GC-002]
-- Pinned items are excluded from automated garbage collection.
-- NOTE: Replay-safe normalization: `ai_jobs.is_pinned` is created in `0002_create_ai_core_tables.sql`.

-- Index for efficient GC queries
CREATE INDEX IF NOT EXISTS idx_ai_jobs_gc ON ai_jobs(status, created_at, is_pinned);
