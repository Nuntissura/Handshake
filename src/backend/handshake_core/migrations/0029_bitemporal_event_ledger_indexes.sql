-- WP-KERNEL-004 MT-157:
-- Add replay/query indexes for bitemporal memory events stored as JSONB
-- payloads in kernel_event_ledger.
--
-- Keep these as text expression indexes. PostgreSQL timestamp casts from
-- text are not immutable, while JSONB text extraction is safe for expression
-- indexes and matches the RFC3339 strings emitted by the ledger payloads.

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_memory_bitemporal_world
    ON kernel_event_ledger (
        (payload #>> '{item,stamps,valid_from}'),
        (payload #>> '{item,stamps,valid_until}'),
        aggregate_id,
        event_sequence DESC
    )
    WHERE aggregate_type = 'memory_bitemporal_item';

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_memory_bitemporal_recorded
    ON kernel_event_ledger (
        (payload #>> '{item,stamps,recorded_at}'),
        (payload #>> '{item,stamps,invalidated_at}'),
        aggregate_id,
        event_sequence DESC
    )
    WHERE aggregate_type = 'memory_bitemporal_item';

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_memory_bitemporal_manifest
    ON kernel_event_ledger (
        event_sequence,
        (payload #>> '{item_id}')
    )
    WHERE aggregate_type = 'memory_bitemporal_manifest'
      AND aggregate_id = 'memory_bitemporal_manifest_v1';
