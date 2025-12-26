-- Migration: Add is_pinned column for retention policy support [HSK-GC-002]
-- Pinned items are excluded from automated garbage collection.

ALTER TABLE ai_jobs ADD COLUMN is_pinned INTEGER NOT NULL DEFAULT 0;

-- Index for efficient GC queries
CREATE INDEX idx_ai_jobs_gc ON ai_jobs(status, created_at, is_pinned);
