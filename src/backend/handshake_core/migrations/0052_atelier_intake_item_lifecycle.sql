-- WP-KERNEL-005 MT-029: explicit intake item lifecycle states plus
-- idempotent rejection audit rows. PostgreSQL authority only.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

UPDATE atelier_intake_item
SET lane = 'pending'
WHERE lane = 'new';

ALTER TABLE atelier_intake_item
    ALTER COLUMN lane SET DEFAULT 'pending';

ALTER TABLE atelier_intake_item
    DROP CONSTRAINT IF EXISTS chk_atelier_intake_item_lane;

ALTER TABLE atelier_intake_item
    ADD CONSTRAINT chk_atelier_intake_item_lane
    CHECK (lane IN ('pending', 'accepted', 'rejected', 'deferred', 'skipped', 'failed'));

CREATE TABLE IF NOT EXISTS atelier_intake_item_rejection_audit (
    audit_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    item_id         UUID NOT NULL REFERENCES atelier_intake_item(item_id) ON DELETE CASCADE,
    batch_id        UUID NOT NULL REFERENCES atelier_intake_batch(batch_id) ON DELETE CASCADE,
    lane            TEXT NOT NULL,
    reason          TEXT NOT NULL,
    source_path_ref TEXT NOT NULL,
    created_at_utc  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CHECK (lane IN ('rejected', 'skipped', 'failed')),
    CHECK (btrim(reason) = reason AND reason <> ''),
    CHECK (btrim(source_path_ref) = source_path_ref AND source_path_ref <> ''),
    UNIQUE (item_id, lane, reason)
);

CREATE INDEX IF NOT EXISTS idx_atelier_intake_rejection_audit_batch
    ON atelier_intake_item_rejection_audit(batch_id, created_at_utc ASC);

CREATE INDEX IF NOT EXISTS idx_atelier_intake_rejection_audit_item
    ON atelier_intake_item_rejection_audit(item_id, created_at_utc ASC);

INSERT INTO atelier_intake_item_rejection_audit
    (item_id, batch_id, lane, reason, source_path_ref, created_at_utc)
SELECT item_id,
       batch_id,
       lane,
       COALESCE(NULLIF(btrim(lane_reason), ''), 'legacy rejection'),
       'sha256:' || encode(digest(source_path, 'sha256'), 'hex'),
       updated_at_utc
FROM atelier_intake_item
WHERE lane IN ('rejected', 'skipped', 'failed')
ON CONFLICT (item_id, lane, reason) DO NOTHING;
