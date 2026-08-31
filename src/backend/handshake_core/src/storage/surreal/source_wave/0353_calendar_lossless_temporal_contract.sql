-- WP-KERNEL-012 MT-067 / Calendar Law v0.4: preserve date-only all-day
-- intent and explicit DST-normalization evidence. Existing rows are left
-- untouched: deriving local dates from UTC would be a silent coercion.

ALTER TABLE calendar_events
    ADD COLUMN IF NOT EXISTS start_date DATE,
    ADD COLUMN IF NOT EXISTS end_date_exclusive DATE,
    ADD COLUMN IF NOT EXISTS normalization_note JSONB,
    ADD COLUMN IF NOT EXISTS temporal_contract_version TEXT;

-- Existing rows remain explicitly legacy/unverified (NULL). The default only
-- applies to new inserts after this migration; constraints therefore harden
-- new writes without guessing missing intent for historic data.
ALTER TABLE calendar_events
    ALTER COLUMN temporal_contract_version SET DEFAULT 'calendar-v02.201';

ALTER TABLE calendar_events
    ADD CONSTRAINT calendar_events_positive_utc_window
        CHECK (end_ts_utc > start_ts_utc) NOT VALID,
    ADD CONSTRAINT calendar_events_temporal_version
        CHECK (temporal_contract_version IS NULL OR temporal_contract_version = 'calendar-v02.201') NOT VALID,
    ADD CONSTRAINT calendar_events_normalized_shape
        CHECK (
            temporal_contract_version IS NULL OR
            (
                all_day = TRUE AND start_date IS NOT NULL AND end_date_exclusive IS NOT NULL AND
                end_date_exclusive > start_date AND start_local IS NULL AND end_local IS NULL AND
                was_floating = FALSE AND normalization_note IS NULL
            ) OR (
                all_day = FALSE AND start_date IS NULL AND end_date_exclusive IS NULL AND
                start_local IS NOT NULL AND end_local IS NOT NULL
            )
        ) NOT VALID;

CREATE INDEX IF NOT EXISTS idx_calendar_events_workspace_all_day_dates
    ON calendar_events (workspace_id, start_date, end_date_exclusive)
    WHERE all_day = TRUE;

CREATE TABLE IF NOT EXISTS calendar_mutation_outbox (
    idempotency_key TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    source_id TEXT NOT NULL,
    calendar_event_id TEXT NOT NULL,
    job_id TEXT,
    workflow_id TEXT,
    actor_kind TEXT NOT NULL,
    actor_id TEXT,
    edit_event_id TEXT NOT NULL UNIQUE,
    ledger_event_id TEXT NOT NULL REFERENCES kernel_event_ledger(event_id),
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_calendar_mutation_outbox_workflow
    ON calendar_mutation_outbox (workflow_id, calendar_event_id, created_at);
