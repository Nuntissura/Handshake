-- WP-1-Calendar-Storage-v1: portable calendar source/event storage

CREATE TABLE IF NOT EXISTS calendar_sources (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    display_name TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    write_policy TEXT NOT NULL,
    default_tzid TEXT NOT NULL DEFAULT 'UTC',
    auto_export BOOLEAN NOT NULL DEFAULT FALSE,
    credentials_ref TEXT,
    provider_calendar_id TEXT,
    capability_profile_id TEXT,
    config_json TEXT NOT NULL DEFAULT '{}',
    sync_state TEXT,
    sync_token TEXT,
    last_sync_ts TIMESTAMP,
    last_full_sync_ts TIMESTAMP,
    last_ok_at TIMESTAMP,
    last_pull_at TIMESTAMP,
    last_push_at TIMESTAMP,
    last_error_at TIMESTAMP,
    last_error_code TEXT,
    last_error TEXT,
    backoff_until TIMESTAMP,
    consecutive_failures BIGINT,
    last_remote_watermark TEXT,
    last_local_applied_rev BIGINT,
    last_job_id TEXT,
    last_workflow_id TEXT,
    last_actor_id TEXT,
    edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS calendar_events (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    source_id TEXT NOT NULL REFERENCES calendar_sources(id) ON DELETE CASCADE,
    external_id TEXT,
    external_etag TEXT,
    title TEXT NOT NULL,
    description TEXT,
    location TEXT,
    start_ts_utc TIMESTAMP NOT NULL,
    end_ts_utc TIMESTAMP NOT NULL,
    start_local TEXT,
    end_local TEXT,
    tzid TEXT NOT NULL DEFAULT 'UTC',
    all_day BOOLEAN NOT NULL DEFAULT FALSE,
    was_floating BOOLEAN NOT NULL DEFAULT FALSE,
    status TEXT NOT NULL DEFAULT 'confirmed',
    visibility TEXT NOT NULL DEFAULT 'private',
    export_mode TEXT NOT NULL DEFAULT 'full_export',
    rrule TEXT,
    rdate_json TEXT NOT NULL DEFAULT '[]',
    exdate_json TEXT NOT NULL DEFAULT '[]',
    is_recurring BOOLEAN NOT NULL DEFAULT FALSE,
    series_id TEXT,
    instance_key TEXT,
    is_override BOOLEAN NOT NULL DEFAULT FALSE,
    source_last_seen_at TIMESTAMP,
    created_by TEXT,
    attendees_json TEXT NOT NULL DEFAULT '[]',
    links_json TEXT NOT NULL DEFAULT '[]',
    provider_payload_json TEXT,
    last_job_id TEXT,
    last_workflow_id TEXT,
    last_actor_id TEXT,
    edit_event_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    last_actor_kind TEXT NOT NULL DEFAULT 'SYSTEM' CHECK (last_actor_kind != 'AI' OR last_job_id IS NOT NULL),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_calendar_sources_workspace_provider
    ON calendar_sources(workspace_id, provider_type);

CREATE INDEX IF NOT EXISTS idx_calendar_events_workspace_window
    ON calendar_events(workspace_id, start_ts_utc, end_ts_utc);

CREATE UNIQUE INDEX IF NOT EXISTS idx_calendar_events_source_external
    ON calendar_events(source_id, external_id);

CREATE INDEX IF NOT EXISTS idx_calendar_events_source_instance
    ON calendar_events(source_id, instance_key);
