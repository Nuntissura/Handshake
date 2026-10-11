-- WP-1 MT-013 V2: durable bounded reconciliation state for pid-less
-- in-process model runtimes. Kept separate from MT-014 registry authority so
-- either feature can be rolled back without disabling the other.

CREATE TABLE IF NOT EXISTS kernel_pidless_embedded_reclaim_cursor (
    host_scope_id TEXT PRIMARY KEY,
    last_instance_id TEXT,
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    CONSTRAINT chk_kernel_pidless_embedded_reclaim_cursor_host_scope
        CHECK (length(pg_catalog.btrim(host_scope_id)) > 0)
);

CREATE INDEX IF NOT EXISTS idx_kernel_process_lifecycle_pidless_embedded_instance_open
    ON kernel_process_lifecycle (
        (metadata_jsonb->>'runtime_instance_id'),
        process_uuid,
        started_at
    )
    WHERE parent_session_id IS NULL
      AND os_pid IS NULL
      AND stopped_at IS NULL
      AND exit_code IS NULL
      AND stop_reason IS NULL
      AND engine_kind IN ('llamacpp', 'candle');
