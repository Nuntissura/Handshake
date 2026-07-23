-- WP-1 MT-004/MT-005/MT-009: persisted CRDT receipts are append-only authority.
-- ModelLane validates and replays these rows under FOR SHARE. Rejecting
-- mutation removes the read/validate/write TOCTOU window and preserves the
-- EventLedger hash and identity binding proven at message admission.

CREATE OR REPLACE FUNCTION reject_kernel_crdt_authority_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION '% is append-only CRDT authority; % is forbidden',
        TG_TABLE_NAME, TG_OP
        USING ERRCODE = '55000';
END;
$$;

DROP TRIGGER IF EXISTS trg_kernel_crdt_updates_append_only
    ON kernel_crdt_updates;
CREATE TRIGGER trg_kernel_crdt_updates_append_only
BEFORE UPDATE OR DELETE ON kernel_crdt_updates
FOR EACH ROW
EXECUTE FUNCTION reject_kernel_crdt_authority_mutation();

DROP TRIGGER IF EXISTS trg_kernel_crdt_updates_append_only_truncate
    ON kernel_crdt_updates;
CREATE TRIGGER trg_kernel_crdt_updates_append_only_truncate
BEFORE TRUNCATE ON kernel_crdt_updates
FOR EACH STATEMENT
EXECUTE FUNCTION reject_kernel_crdt_authority_mutation();

DROP TRIGGER IF EXISTS trg_kernel_crdt_snapshots_append_only
    ON kernel_crdt_snapshots;
CREATE TRIGGER trg_kernel_crdt_snapshots_append_only
BEFORE UPDATE OR DELETE ON kernel_crdt_snapshots
FOR EACH ROW
EXECUTE FUNCTION reject_kernel_crdt_authority_mutation();

DROP TRIGGER IF EXISTS trg_kernel_crdt_snapshots_append_only_truncate
    ON kernel_crdt_snapshots;
CREATE TRIGGER trg_kernel_crdt_snapshots_append_only_truncate
BEFORE TRUNCATE ON kernel_crdt_snapshots
FOR EACH STATEMENT
EXECUTE FUNCTION reject_kernel_crdt_authority_mutation();
