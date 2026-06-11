-- WP-KERNEL-009 MT-062 PostgresEventLedgerCore-062-TransactionalIdempotencyKeys.
-- Master Spec anchor: 2.3.13.11 (write-box/promotion discipline: replayed
-- mutations must not double-write authority records) + the EventLedger
-- idempotency precedent (kernel_event_ledger.idempotency_key, 0018).
--
-- knowledge_idempotency_keys is the shared idempotency ledger for WP-009
-- knowledge mutations (parallel indexing, editor saves, graph writes, bundle
-- builds). Discipline (storage/knowledge.rs):
--
--   1. The caller supplies an idempotency_key and the write's request_hash
--      (sha256 over the canonical request payload).
--   2. The write and the key row commit in ONE transaction.
--   3. A replay with the SAME key + SAME request_hash returns the prior
--      result (result_ref_kind/result_ref_id) without writing anything.
--   4. The SAME key with a DIFFERENT request_hash is a typed Conflict
--      (divergent duplicate), mirroring kernel_event_ledger semantics.
--   5. Two racing writers on the same key: the second transaction fails the
--      primary-key insert, rolls back its write, and re-reads the winner's
--      result — no double-write under concurrency.

CREATE TABLE IF NOT EXISTS knowledge_idempotency_keys (
    idempotency_key TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    operation_kind TEXT NOT NULL CHECK (operation_kind IN (
        'index_run_start', 'source_upsert', 'span_write', 'entity_write',
        'edge_write', 'claim_write', 'passage_write', 'projection_write',
        'rich_document_save', 'bundle_build'
    )),
    -- sha256 over the canonical JSON of the request payload.
    request_hash TEXT NOT NULL,
    -- What the original request produced.
    result_ref_kind TEXT NOT NULL,
    result_ref_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_knowledge_idempotency_keys_key
        CHECK (btrim(idempotency_key) = idempotency_key AND idempotency_key <> ''),
    CONSTRAINT chk_knowledge_idempotency_keys_hash
        CHECK (request_hash ~ '^[0-9a-f]{64}$'),
    CONSTRAINT chk_knowledge_idempotency_keys_result
        CHECK (btrim(result_ref_kind) = result_ref_kind AND result_ref_kind <> ''
               AND btrim(result_ref_id) = result_ref_id AND result_ref_id <> '')
);

CREATE INDEX IF NOT EXISTS idx_knowledge_idempotency_keys_workspace
    ON knowledge_idempotency_keys (workspace_id, operation_kind);

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('idempotency_keys', 'knowledge_idempotency_keys', 'Support',
     'support', '0142_knowledge_idempotency_keys.sql', 'MT-062')
ON CONFLICT (family_key) DO NOTHING;
