-- WP-KERNEL-005 atelier EventLedger projection linkage.
-- Keeps atelier_event as a compatibility projection while routing new events
-- through the canonical kernel_event_ledger authority.

ALTER TABLE atelier_event ADD COLUMN IF NOT EXISTS kernel_event_id TEXT;
ALTER TABLE atelier_event ADD COLUMN IF NOT EXISTS kernel_event_sequence BIGINT;

CREATE INDEX IF NOT EXISTS idx_atelier_event_kernel_event
    ON atelier_event(kernel_event_id);
