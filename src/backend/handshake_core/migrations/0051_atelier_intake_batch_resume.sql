-- MT-028: durable intake batch mode/source-ref/resume metadata.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

ALTER TABLE atelier_intake_batch
    ADD COLUMN IF NOT EXISTS source_ref TEXT,
    ADD COLUMN IF NOT EXISTS mode TEXT NOT NULL DEFAULT 'manual',
    ADD COLUMN IF NOT EXISTS resume_cursor TEXT,
    ADD COLUMN IF NOT EXISTS resumed_at_utc TIMESTAMPTZ;

UPDATE atelier_intake_batch
SET source_ref = 'sha256:' || encode(digest(source_label, 'sha256'), 'hex')
WHERE source_ref IS NULL OR btrim(source_ref) = '';

ALTER TABLE atelier_intake_batch
    ALTER COLUMN source_ref SET NOT NULL;

ALTER TABLE atelier_intake_batch
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_batch_status;

ALTER TABLE atelier_intake_batch
    ADD CONSTRAINT chk_atelier_intake_batch_status
    CHECK (status IN ('open', 'in_progress', 'closed'));

ALTER TABLE atelier_intake_batch
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_batch_mode;

ALTER TABLE atelier_intake_batch
    ADD CONSTRAINT chk_atelier_intake_batch_mode
    CHECK (mode IN ('manual', 'folder_scan', 'sourcing_run'));

ALTER TABLE atelier_intake_batch
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_batch_source_ref;

ALTER TABLE atelier_intake_batch
    ADD CONSTRAINT chk_atelier_intake_batch_source_ref
    CHECK (btrim(source_ref) <> '');

ALTER TABLE atelier_intake_batch
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_batch_resume_cursor;

ALTER TABLE atelier_intake_batch
    ADD CONSTRAINT chk_atelier_intake_batch_resume_cursor
    CHECK (resume_cursor IS NULL OR btrim(resume_cursor) <> '');

CREATE INDEX IF NOT EXISTS idx_atelier_intake_batch_mode_status
    ON atelier_intake_batch(mode, status, updated_at_utc DESC);

CREATE INDEX IF NOT EXISTS idx_atelier_intake_batch_source_ref
    ON atelier_intake_batch(source_ref);
