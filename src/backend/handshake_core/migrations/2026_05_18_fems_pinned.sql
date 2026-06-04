-- MT-159 FEMS pinned core memory.
-- Pin state is an event-ledger projection, not a memory_item table column.
-- This preserves the MT-157 decision that Memory V0+ authority is in
-- kernel_event_ledger JSONB rows and avoids reintroducing a shadow table.

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_memory_pin_item
    ON kernel_event_ledger (aggregate_id, event_sequence DESC)
    WHERE aggregate_type = 'memory_item'
      AND payload->'write_box_envelope'->'payload'->>'schema_id' = 'hsk.memory_pin.payload@1';

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_memory_pin_manifest
    ON kernel_event_ledger (event_sequence DESC)
    WHERE aggregate_type = 'memory_pin_manifest'
      AND aggregate_id = 'memory_pin_manifest_v1';

CREATE INDEX IF NOT EXISTS idx_kernel_event_ledger_memory_pin_payload_pinned
    ON kernel_event_ledger ((payload->'write_box_envelope'->'payload'->'pinned_item'->>'pinned'))
    WHERE aggregate_type = 'memory_item'
      AND payload->'write_box_envelope'->'payload'->>'schema_id' = 'hsk.memory_pin.payload@1';
