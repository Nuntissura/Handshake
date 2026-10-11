-- WP-1 MT-002/003/004/005/006/007: typed process-owner identity.
-- Runtime liveness and cross-host reclaim decisions must not depend on
-- free-form JSON metadata.

ALTER TABLE kernel_process_lifecycle
    ADD COLUMN IF NOT EXISTS owner_runtime_instance_id UUID,
    ADD COLUMN IF NOT EXISTS owner_host_scope_id TEXT,
    ADD COLUMN IF NOT EXISTS owner_lease_schema_id TEXT,
    ADD COLUMN IF NOT EXISTS owner_lease_protocol TEXT,
    ADD COLUMN IF NOT EXISTS owner_lease_address TEXT,
    ADD COLUMN IF NOT EXISTS owner_lease_port INTEGER;

-- Conflicting legacy rows cannot be backfilled safely: one runtime UUID must
-- identify exactly one immutable lease descriptor. Keep those rows untyped
-- and persist actionable diagnostics instead of silently creating an
-- authority ambiguity that the guard below can no longer repair.
DO $migration_0359_quarantine$
BEGIN
    IF (
        SELECT relation.relpersistence = 't'
        FROM pg_catalog.pg_class AS relation
        WHERE relation.oid = 'kernel_process_lifecycle'::pg_catalog.regclass
    ) THEN
        CREATE TEMP TABLE IF NOT EXISTS kernel_process_runtime_owner_legacy_quarantine (
            process_uuid UUID PRIMARY KEY,
            runtime_instance_id UUID NOT NULL,
            descriptor_jsonb JSONB NOT NULL,
            conflicting_descriptors_jsonb JSONB NOT NULL,
            reason TEXT NOT NULL,
            repair_hint TEXT NOT NULL,
            quarantined_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
    ELSE
        CREATE TABLE IF NOT EXISTS kernel_process_runtime_owner_legacy_quarantine (
            process_uuid UUID PRIMARY KEY,
            runtime_instance_id UUID NOT NULL,
            descriptor_jsonb JSONB NOT NULL,
            conflicting_descriptors_jsonb JSONB NOT NULL,
            reason TEXT NOT NULL,
            repair_hint TEXT NOT NULL,
            quarantined_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
    END IF;
END;
$migration_0359_quarantine$;

-- The quarantine is a current-state projection. Rebuilding it on a rerun is
-- the repair path: once the legacy metadata for every row sharing a runtime
-- UUID describes one canonical lease, the rows leave quarantine and the
-- ordinary backfill below completes.
DELETE FROM kernel_process_runtime_owner_legacy_quarantine;

WITH eligible_legacy_runtime_owner AS (
    SELECT
        process_uuid,
        (metadata_jsonb->>'runtime_instance_id')::UUID AS runtime_instance_id,
        pg_catalog.jsonb_build_object(
            'runtime_host_scope_id', metadata_jsonb->>'runtime_host_scope_id',
            'runtime_instance_schema_id', metadata_jsonb->>'runtime_instance_schema_id',
            'runtime_lease_protocol', metadata_jsonb->>'runtime_lease_protocol',
            'runtime_lease_address', metadata_jsonb->>'runtime_lease_address',
            'runtime_lease_port', (metadata_jsonb->>'runtime_lease_port')::INTEGER
        ) AS descriptor_jsonb
    FROM kernel_process_lifecycle
    WHERE owner_runtime_instance_id IS NULL
      AND metadata_jsonb->>'runtime_instance_id'
          ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
      AND NULLIF(pg_catalog.btrim(metadata_jsonb->>'runtime_host_scope_id'), '') IS NOT NULL
      AND NULLIF(pg_catalog.btrim(metadata_jsonb->>'runtime_instance_schema_id'), '') IS NOT NULL
      AND NULLIF(pg_catalog.btrim(metadata_jsonb->>'runtime_lease_protocol'), '') IS NOT NULL
      AND metadata_jsonb->>'runtime_lease_address' IN ('127.0.0.1', '::1')
      AND metadata_jsonb->>'runtime_lease_port' ~ '^[0-9]+$'
      AND pg_catalog.length(metadata_jsonb->>'runtime_lease_port') <= 5
      AND (metadata_jsonb->>'runtime_lease_port')::INTEGER BETWEEN 1 AND 65535
), conflicting_runtime_owner AS (
    SELECT
        runtime_instance_id,
        pg_catalog.jsonb_agg(DISTINCT descriptor_jsonb) AS conflicting_descriptors_jsonb
    FROM eligible_legacy_runtime_owner
    GROUP BY runtime_instance_id
    HAVING pg_catalog.count(DISTINCT descriptor_jsonb) > 1
)
INSERT INTO kernel_process_runtime_owner_legacy_quarantine (
    process_uuid,
    runtime_instance_id,
    descriptor_jsonb,
    conflicting_descriptors_jsonb,
    reason,
    repair_hint
)
SELECT
    legacy.process_uuid,
    legacy.runtime_instance_id,
    legacy.descriptor_jsonb,
    conflict.conflicting_descriptors_jsonb,
    'legacy runtime UUID has conflicting lease descriptors',
    'make all legacy metadata rows for this runtime UUID use one descriptor, then rerun migration 0359'
FROM eligible_legacy_runtime_owner AS legacy
JOIN conflicting_runtime_owner AS conflict
  ON conflict.runtime_instance_id = legacy.runtime_instance_id;

WITH legacy_runtime_owner AS (
    SELECT
        process_uuid,
        CASE
            WHEN metadata_jsonb->>'runtime_instance_id'
                 ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
            THEN (metadata_jsonb->>'runtime_instance_id')::UUID
            ELSE NULL
        END AS runtime_instance_id,
        metadata_jsonb->>'runtime_host_scope_id' AS runtime_host_scope_id,
        metadata_jsonb->>'runtime_instance_schema_id' AS runtime_instance_schema_id,
        metadata_jsonb->>'runtime_lease_protocol' AS runtime_lease_protocol,
        metadata_jsonb->>'runtime_lease_address' AS runtime_lease_address,
        CASE
            WHEN metadata_jsonb->>'runtime_lease_port' ~ '^[0-9]+$'
                 AND pg_catalog.length(metadata_jsonb->>'runtime_lease_port') <= 5
            THEN (metadata_jsonb->>'runtime_lease_port')::INTEGER
            ELSE NULL
        END AS runtime_lease_port
    FROM kernel_process_lifecycle
    WHERE owner_runtime_instance_id IS NULL
)
UPDATE kernel_process_lifecycle AS lifecycle
SET owner_runtime_instance_id = legacy.runtime_instance_id,
    owner_host_scope_id = legacy.runtime_host_scope_id,
    owner_lease_schema_id = legacy.runtime_instance_schema_id,
    owner_lease_protocol = legacy.runtime_lease_protocol,
    owner_lease_address = legacy.runtime_lease_address,
    owner_lease_port = legacy.runtime_lease_port
FROM legacy_runtime_owner AS legacy
WHERE lifecycle.process_uuid = legacy.process_uuid
  AND NOT EXISTS (
      SELECT 1
      FROM kernel_process_runtime_owner_legacy_quarantine AS quarantine
      WHERE quarantine.process_uuid = lifecycle.process_uuid
  )
  AND legacy.runtime_instance_id IS NOT NULL
  AND NULLIF(pg_catalog.btrim(legacy.runtime_host_scope_id), '') IS NOT NULL
  AND NULLIF(pg_catalog.btrim(legacy.runtime_instance_schema_id), '') IS NOT NULL
  AND NULLIF(pg_catalog.btrim(legacy.runtime_lease_protocol), '') IS NOT NULL
  AND legacy.runtime_lease_address IN ('127.0.0.1', '::1')
  AND legacy.runtime_lease_port BETWEEN 1 AND 65535;

ALTER TABLE kernel_process_lifecycle
    DROP CONSTRAINT IF EXISTS chk_kernel_process_lifecycle_runtime_owner_complete,
    ADD CONSTRAINT chk_kernel_process_lifecycle_runtime_owner_complete CHECK (
        (owner_runtime_instance_id IS NULL
         AND owner_host_scope_id IS NULL
         AND owner_lease_schema_id IS NULL
         AND owner_lease_protocol IS NULL
         AND owner_lease_address IS NULL
         AND owner_lease_port IS NULL)
        OR
        (owner_runtime_instance_id IS NOT NULL
         AND NULLIF(pg_catalog.btrim(owner_host_scope_id), '') IS NOT NULL
         AND NULLIF(pg_catalog.btrim(owner_lease_schema_id), '') IS NOT NULL
         AND NULLIF(pg_catalog.btrim(owner_lease_protocol), '') IS NOT NULL
         AND NULLIF(pg_catalog.btrim(owner_lease_address), '') IS NOT NULL
         AND owner_lease_port BETWEEN 1 AND 65535)
    );

CREATE INDEX IF NOT EXISTS idx_kernel_process_lifecycle_runtime_owner_open
    ON kernel_process_lifecycle (
        owner_host_scope_id,
        owner_runtime_instance_id,
        parent_session_id,
        started_at
    )
    WHERE stopped_at IS NULL;

-- One UUID denotes one immutable OS lease descriptor. Multiple lifecycle rows
-- may share that descriptor, but a second descriptor for the same UUID would
-- make a death probe ambiguous and could authorize reclaim of a live owner.
-- The advisory transaction lock closes the concurrent first-insert race.
CREATE OR REPLACE FUNCTION kernel_process_runtime_owner_descriptor_guard()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.owner_runtime_instance_id IS NULL THEN
        RETURN NEW;
    END IF;

    IF TG_OP = 'UPDATE'
       AND OLD.owner_runtime_instance_id IS NOT NULL
       AND (
           OLD.owner_runtime_instance_id IS DISTINCT FROM NEW.owner_runtime_instance_id
           OR OLD.owner_host_scope_id IS DISTINCT FROM NEW.owner_host_scope_id
           OR OLD.owner_lease_schema_id IS DISTINCT FROM NEW.owner_lease_schema_id
           OR OLD.owner_lease_protocol IS DISTINCT FROM NEW.owner_lease_protocol
           OR OLD.owner_lease_address IS DISTINCT FROM NEW.owner_lease_address
           OR OLD.owner_lease_port IS DISTINCT FROM NEW.owner_lease_port
       ) THEN
        RAISE EXCEPTION
            'typed runtime-owner descriptor is immutable for process %',
            NEW.process_uuid
            USING ERRCODE = '23514';
    END IF;

    PERFORM pg_catalog.pg_advisory_xact_lock(
        pg_catalog.hashtextextended(NEW.owner_runtime_instance_id::text, 359)
    );

    IF EXISTS (
        SELECT 1
        FROM kernel_process_lifecycle AS existing
        WHERE existing.owner_runtime_instance_id = NEW.owner_runtime_instance_id
          AND existing.process_uuid <> NEW.process_uuid
          AND (
              existing.owner_host_scope_id IS DISTINCT FROM NEW.owner_host_scope_id
              OR existing.owner_lease_schema_id IS DISTINCT FROM NEW.owner_lease_schema_id
              OR existing.owner_lease_protocol IS DISTINCT FROM NEW.owner_lease_protocol
              OR existing.owner_lease_address IS DISTINCT FROM NEW.owner_lease_address
              OR existing.owner_lease_port IS DISTINCT FROM NEW.owner_lease_port
          )
    ) THEN
        RAISE EXCEPTION
            'runtime instance % already has a different typed lease descriptor',
            NEW.owner_runtime_instance_id
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_kernel_process_runtime_owner_descriptor_guard
    ON kernel_process_lifecycle;
CREATE TRIGGER trg_kernel_process_runtime_owner_descriptor_guard
BEFORE INSERT OR UPDATE OF
    owner_runtime_instance_id,
    owner_host_scope_id,
    owner_lease_schema_id,
    owner_lease_protocol,
    owner_lease_address,
    owner_lease_port
ON kernel_process_lifecycle
FOR EACH ROW
EXECUTE FUNCTION kernel_process_runtime_owner_descriptor_guard();
