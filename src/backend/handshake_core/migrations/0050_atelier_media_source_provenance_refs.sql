-- MT-027: structured media source provenance refs.
-- Keeps the legacy source_provenance text for compatibility while adding
-- distinct durable refs for source URL/path/note/contact-sheet/task/run.

CREATE TABLE IF NOT EXISTS atelier_media_source_provenance_ref (
    asset_id           UUID PRIMARY KEY REFERENCES atelier_media_asset(asset_id) ON DELETE CASCADE,
    source_url_ref     TEXT,
    source_path_ref    TEXT,
    source_note_ref    TEXT,
    contact_sheet_ref  TEXT,
    task_ref           TEXT,
    run_ref            TEXT,
    updated_by         TEXT NOT NULL,
    updated_at_utc     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (
        source_url_ref IS NOT NULL
        OR source_path_ref IS NOT NULL
        OR source_note_ref IS NOT NULL
        OR contact_sheet_ref IS NOT NULL
        OR task_ref IS NOT NULL
        OR run_ref IS NOT NULL
    )
);

CREATE INDEX IF NOT EXISTS idx_atelier_media_source_provenance_url
    ON atelier_media_source_provenance_ref(source_url_ref)
    WHERE source_url_ref IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_atelier_media_source_provenance_path
    ON atelier_media_source_provenance_ref(source_path_ref)
    WHERE source_path_ref IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_atelier_media_source_provenance_contact_sheet
    ON atelier_media_source_provenance_ref(contact_sheet_ref)
    WHERE contact_sheet_ref IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_atelier_media_source_provenance_task
    ON atelier_media_source_provenance_ref(task_ref)
    WHERE task_ref IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_atelier_media_source_provenance_run
    ON atelier_media_source_provenance_ref(run_ref)
    WHERE run_ref IS NOT NULL;
