DROP TRIGGER IF EXISTS trg_kernel_crdt_snapshots_append_only_truncate
    ON kernel_crdt_snapshots;
DROP TRIGGER IF EXISTS trg_kernel_crdt_snapshots_append_only
    ON kernel_crdt_snapshots;
DROP TRIGGER IF EXISTS trg_kernel_crdt_updates_append_only_truncate
    ON kernel_crdt_updates;
DROP TRIGGER IF EXISTS trg_kernel_crdt_updates_append_only
    ON kernel_crdt_updates;
DROP FUNCTION IF EXISTS reject_kernel_crdt_authority_mutation();
