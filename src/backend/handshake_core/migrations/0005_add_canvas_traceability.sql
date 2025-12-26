-- Backfill missing traceability columns on canvases table.

ALTER TABLE canvases ADD COLUMN last_job_id TEXT;
ALTER TABLE canvases ADD COLUMN last_workflow_id TEXT;
ALTER TABLE canvases ADD COLUMN last_actor_id TEXT;
ALTER TABLE canvases ADD COLUMN edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';
ALTER TABLE canvases ADD COLUMN last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL);
