DROP INDEX IF EXISTS idx_kernel_crdt_snapshots_state_vector;
DROP INDEX IF EXISTS idx_kernel_crdt_snapshots_latest;
DROP INDEX IF EXISTS idx_kernel_crdt_snapshots_bytes_ref;
DROP INDEX IF EXISTS idx_kernel_crdt_snapshots_event;
DROP TABLE IF EXISTS kernel_crdt_snapshots;

DROP INDEX IF EXISTS idx_kernel_crdt_updates_state_vector_after;
DROP INDEX IF EXISTS idx_kernel_crdt_updates_replay;
DROP INDEX IF EXISTS idx_kernel_crdt_updates_bytes_ref;
DROP INDEX IF EXISTS idx_kernel_crdt_updates_event;
DROP INDEX IF EXISTS idx_kernel_crdt_updates_seq;
DROP TABLE IF EXISTS kernel_crdt_updates;
