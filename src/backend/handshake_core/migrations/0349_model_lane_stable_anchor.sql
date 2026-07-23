-- MT-014/MT-017: durable cross-session model identity for lane diagnostics.
ALTER TABLE model_lanes ADD COLUMN IF NOT EXISTS model_stable_anchor TEXT NULL;
ALTER TABLE model_lanes DROP CONSTRAINT IF EXISTS chk_model_lanes_stable_anchor_sha256;
ALTER TABLE model_lanes ADD CONSTRAINT chk_model_lanes_stable_anchor_sha256
    CHECK (model_stable_anchor IS NULL OR model_stable_anchor ~ '^[0-9a-f]{64}$');
CREATE INDEX IF NOT EXISTS idx_model_lanes_stable_anchor
    ON model_lanes(model_stable_anchor) WHERE model_stable_anchor IS NOT NULL;
