-- WP-KERNEL-009 MT-076 CRDTAndConcurrencyCore-076-AgentLeaseExpiration.
-- Contract source: MT-041 swarm_lease_checkpoint_contract_seed (AgentLaneLease
-- MUST-level semantics):
--   * a lease MUST be claimed before a model session mutates knowledge
--     drafts, rich-document drafts, index runs, or graph mutation proposals;
--   * lease rows MUST carry lease_id (ULID), lane_id, actor_id, actor_kind,
--     session_id, correlation_id, scope_ref, claimed_at_utc, expires_at_utc,
--     renewal_count, released_at_utc (nullable), takeover_of (nullable);
--   * expiry MUST be enforced server-side; writes under expired/foreign
--     leases MUST be denied with typed denial receipts
--     (knowledge_crdt_denial_receipts, migration 0150);
--   * all transitions MUST be EventLedger events with idempotency keys.
--
-- Single-holder rule: at most one unreleased lease per scope (partial unique
-- index). Takeover of an expired lease releases the old row and inserts the
-- new one with takeover_of in a single transaction.

CREATE TABLE IF NOT EXISTS knowledge_crdt_agent_lane_leases (
    lease_id TEXT PRIMARY KEY,
    lane_id TEXT NOT NULL CHECK (btrim(lane_id) <> ''),
    actor_id TEXT NOT NULL CHECK (actor_id ~ '^(operator|local_model|cloud_model|validator|system):[A-Za-z0-9._-]+$'),
    actor_kind TEXT NOT NULL CHECK (actor_kind IN (
        'operator', 'local_model', 'cloud_model', 'validator', 'system'
    )),
    session_id TEXT NOT NULL CHECK (btrim(session_id) <> ''),
    correlation_id TEXT NOT NULL CHECK (btrim(correlation_id) <> ''),
    scope_kind TEXT NOT NULL CHECK (scope_kind IN (
        'workspace', 'document', 'source_root', 'index_run'
    )),
    scope_id TEXT NOT NULL CHECK (btrim(scope_id) <> ''),
    claimed_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at_utc TIMESTAMPTZ NOT NULL,
    renewal_count BIGINT NOT NULL DEFAULT 0 CHECK (renewal_count >= 0),
    released_at_utc TIMESTAMPTZ,
    -- Server-side expiry sweep stamp (KNOWLEDGE_CRDT_LEASE_EXPIRED event).
    expired_at_utc TIMESTAMPTZ,
    takeover_of TEXT REFERENCES knowledge_crdt_agent_lane_leases(lease_id)
        ON UPDATE RESTRICT ON DELETE RESTRICT,
    CONSTRAINT chk_knowledge_crdt_lease_id_ulid
        CHECK (lease_id ~ '^[0-9A-HJKMNP-TV-Z]{26}$'),
    CONSTRAINT chk_knowledge_crdt_lease_expiry_after_claim
        CHECK (expires_at_utc > claimed_at_utc)
);

-- Single unreleased holder per scope (server-side concurrency guard).
CREATE UNIQUE INDEX IF NOT EXISTS uq_knowledge_crdt_lease_active_scope
    ON knowledge_crdt_agent_lane_leases (scope_kind, scope_id)
    WHERE released_at_utc IS NULL;

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_lease_session
    ON knowledge_crdt_agent_lane_leases (session_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_crdt_lease_expiry
    ON knowledge_crdt_agent_lane_leases (expires_at_utc)
    WHERE released_at_utc IS NULL;

INSERT INTO knowledge_schema_registry
    (family_key, table_name, record_family, authority_class, migration_file, mt_id)
VALUES
    ('crdt_agent_lane_leases', 'knowledge_crdt_agent_lane_leases', 'AgentLaneLease',
     'support', '0151_knowledge_crdt_agent_lane_leases.sql', 'MT-076')
ON CONFLICT (family_key) DO NOTHING;
