-- WP-KERNEL-012 MT-067: the calendar activity-span store.
--
-- The native editor records which documents it edited during a calendar event
-- as a calendar ACTIVITY SPAN. This is a DISTINCT concept from
-- `flight_recorder::spans` (table `kernel_activity_span`, a swarm / mt-iteration
-- span with no calendar linkage): every row here carries a `calendar_event_id`
-- and the set of documents edited during that event window (`edited_doc_ids`).
--
-- Additive / back-compat: a new table + indexes only. PostgreSQL authority.
CREATE TABLE IF NOT EXISTS calendar_activity_spans (
    span_id            TEXT PRIMARY KEY,
    workspace_id       TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    -- FK-ish soft reference to calendar_events(id). Intentionally NOT a hard FK:
    -- an activity span is the native editor's own edit provenance and must
    -- survive calendar-event churn (sync replace/delete), so it is a soft
    -- reference validated at write time, not a cascade target.
    calendar_event_id  TEXT NOT NULL,
    started_utc        TIMESTAMPTZ NOT NULL,
    -- NULL while the span is still open (an in-progress edit block).
    ended_utc          TIMESTAMPTZ,
    -- The documents edited during the span, as a JSON array of doc-id strings.
    edited_doc_ids     JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Primary read path: all spans for one calendar event in a workspace.
CREATE INDEX IF NOT EXISTS ix_calendar_activity_spans_event
    ON calendar_activity_spans (workspace_id, calendar_event_id);

CREATE INDEX IF NOT EXISTS ix_calendar_activity_spans_workspace
    ON calendar_activity_spans (workspace_id);
