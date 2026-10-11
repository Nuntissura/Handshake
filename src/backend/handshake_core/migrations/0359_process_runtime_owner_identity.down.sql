DROP INDEX IF EXISTS idx_kernel_process_lifecycle_runtime_owner_open;

DROP TRIGGER IF EXISTS trg_kernel_process_runtime_owner_descriptor_guard
    ON kernel_process_lifecycle;
DROP FUNCTION IF EXISTS kernel_process_runtime_owner_descriptor_guard();

DROP TABLE IF EXISTS kernel_process_runtime_owner_legacy_quarantine;

ALTER TABLE kernel_process_lifecycle
    DROP CONSTRAINT IF EXISTS chk_kernel_process_lifecycle_runtime_owner_complete,
    DROP COLUMN IF EXISTS owner_lease_port,
    DROP COLUMN IF EXISTS owner_lease_address,
    DROP COLUMN IF EXISTS owner_lease_protocol,
    DROP COLUMN IF EXISTS owner_lease_schema_id,
    DROP COLUMN IF EXISTS owner_host_scope_id,
    DROP COLUMN IF EXISTS owner_runtime_instance_id;
