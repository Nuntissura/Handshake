DROP INDEX IF EXISTS idx_model_lanes_stable_anchor;
ALTER TABLE model_lanes DROP CONSTRAINT IF EXISTS chk_model_lanes_stable_anchor_sha256;
ALTER TABLE model_lanes DROP COLUMN IF EXISTS model_stable_anchor;
