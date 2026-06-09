-- WP-KERNEL-005 MT-027 follow-up: harden structured media source
-- provenance refs at the database boundary. Rust validates richer portable-ref
-- rules; SQL enforces the durable minimum of non-empty trimmed strings.

DELETE FROM atelier_media_source_provenance_ref
WHERE NULLIF(btrim(source_url_ref), '') IS NULL
  AND NULLIF(btrim(source_path_ref), '') IS NULL
  AND NULLIF(btrim(source_note_ref), '') IS NULL
  AND NULLIF(btrim(contact_sheet_ref), '') IS NULL
  AND NULLIF(btrim(task_ref), '') IS NULL
  AND NULLIF(btrim(run_ref), '') IS NULL;

UPDATE atelier_media_source_provenance_ref
SET source_url_ref = NULLIF(btrim(source_url_ref), ''),
    source_path_ref = NULLIF(btrim(source_path_ref), ''),
    source_note_ref = NULLIF(btrim(source_note_ref), ''),
    contact_sheet_ref = NULLIF(btrim(contact_sheet_ref), ''),
    task_ref = NULLIF(btrim(task_ref), ''),
    run_ref = NULLIF(btrim(run_ref), ''),
    updated_by = COALESCE(NULLIF(btrim(updated_by), ''), 'migration:unknown');

ALTER TABLE atelier_media_source_provenance_ref
    DROP CONSTRAINT IF EXISTS chk_atelier_media_source_provenance_ref_trimmed_nonempty;

ALTER TABLE atelier_media_source_provenance_ref
    ADD CONSTRAINT chk_atelier_media_source_provenance_ref_trimmed_nonempty
    CHECK (
        updated_by = btrim(updated_by)
        AND updated_by <> ''
        AND (source_url_ref IS NULL OR (source_url_ref = btrim(source_url_ref) AND source_url_ref <> ''))
        AND (source_path_ref IS NULL OR (source_path_ref = btrim(source_path_ref) AND source_path_ref <> ''))
        AND (source_note_ref IS NULL OR (source_note_ref = btrim(source_note_ref) AND source_note_ref <> ''))
        AND (contact_sheet_ref IS NULL OR (contact_sheet_ref = btrim(contact_sheet_ref) AND contact_sheet_ref <> ''))
        AND (task_ref IS NULL OR (task_ref = btrim(task_ref) AND task_ref <> ''))
        AND (run_ref IS NULL OR (run_ref = btrim(run_ref) AND run_ref <> ''))
    );
