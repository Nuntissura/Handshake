DROP TABLE IF EXISTS calendar_mutation_outbox;
DROP INDEX IF EXISTS idx_calendar_events_workspace_all_day_dates;

ALTER TABLE calendar_events
    DROP CONSTRAINT IF EXISTS calendar_events_normalized_shape,
    DROP CONSTRAINT IF EXISTS calendar_events_temporal_version,
    DROP CONSTRAINT IF EXISTS calendar_events_positive_utc_window,
    DROP COLUMN IF EXISTS temporal_contract_version,
    DROP COLUMN IF EXISTS normalization_note,
    DROP COLUMN IF EXISTS end_date_exclusive,
    DROP COLUMN IF EXISTS start_date;
