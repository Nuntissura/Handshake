-- WP-KERNEL-005 MT-014 bulk-operation receipts.
-- Bulk actions validate every target before mutation, then commit target writes
-- and one durable receipt in the same PostgreSQL/EventLedger transaction.

CREATE TABLE IF NOT EXISTS atelier_bulk_operation_receipt (
    receipt_id      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    operation       TEXT NOT NULL,
    requested_by    TEXT NOT NULL,
    target_count    BIGINT NOT NULL,
    mutation_count  BIGINT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'applied',
    payload         JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_atelier_bulk_operation_receipt_operation
    ON atelier_bulk_operation_receipt(operation, created_at_utc DESC);

CREATE TABLE IF NOT EXISTS atelier_trash_marker (
    marker_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    target_type     TEXT NOT NULL CHECK (target_type IN ('media_asset', 'sheet_version')),
    target_id       UUID NOT NULL,
    reason          TEXT NOT NULL,
    requested_by    TEXT NOT NULL,
    created_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (target_type, target_id)
);
CREATE INDEX IF NOT EXISTS idx_atelier_trash_marker_target
    ON atelier_trash_marker(target_type, target_id);
