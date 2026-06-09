-- WP-KERNEL-005 / MT-200 settings preference metadata.
-- Additive only: do not modify applied migration 0031.

ALTER TABLE atelier_preference
    ADD COLUMN IF NOT EXISTS namespace TEXT,
    ADD COLUMN IF NOT EXISTS name TEXT,
    ADD COLUMN IF NOT EXISTS default_value TEXT,
    ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'operator',
    ADD COLUMN IF NOT EXISTS updated_by TEXT,
    ADD COLUMN IF NOT EXISTS revision BIGINT NOT NULL DEFAULT 1,
    ADD COLUMN IF NOT EXISTS redaction_class TEXT NOT NULL DEFAULT 'public';

UPDATE atelier_preference
SET namespace = split_part(key, '.', 1),
    name = substring(key from position('.' in key) + 1)
WHERE namespace IS NULL
  AND position('.' in key) > 0;

UPDATE atelier_preference
SET namespace = 'legacy',
    name = key
WHERE namespace IS NULL;

ALTER TABLE atelier_preference
    ALTER COLUMN namespace SET NOT NULL,
    ALTER COLUMN name SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_atelier_preference_namespace
    ON atelier_preference(namespace, name);

CREATE INDEX IF NOT EXISTS idx_atelier_preference_scope_revision
    ON atelier_preference(scope_kind, character_internal_id, revision);
