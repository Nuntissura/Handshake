-- WP-KERNEL-004 cluster X.1 MT-176/MT-177 schema for the durable
-- Postgres-backed Role Mailbox thread + message authority surface.

CREATE TABLE IF NOT EXISTS role_mailbox_thread (
    thread_id UUID PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    linked_record_kind TEXT NOT NULL,
    linked_record_id TEXT,
    lifecycle_state TEXT NOT NULL,
    executor_kind_allowlist JSONB NOT NULL DEFAULT '[]'::jsonb,
    claim_mode TEXT NOT NULL,
    lease_duration_secs BIGINT,
    takeover_policy TEXT NOT NULL,
    response_authority_scope TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at_utc TIMESTAMPTZ,
    archived_at_utc TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_role_mailbox_thread_state_expires
    ON role_mailbox_thread (lifecycle_state, expires_at_utc);

CREATE INDEX IF NOT EXISTS idx_role_mailbox_thread_linked
    ON role_mailbox_thread (linked_record_kind, linked_record_id);

CREATE INDEX IF NOT EXISTS idx_role_mailbox_thread_updated
    ON role_mailbox_thread (updated_at_utc DESC);

CREATE TABLE IF NOT EXISTS role_mailbox_message (
    message_id UUID PRIMARY KEY NOT NULL,
    thread_id UUID NOT NULL REFERENCES role_mailbox_thread(thread_id) ON DELETE CASCADE,
    message_type TEXT NOT NULL,
    from_role TEXT NOT NULL,
    to_roles JSONB NOT NULL DEFAULT '[]'::jsonb,
    expected_response JSONB,
    expires_at_utc TIMESTAMPTZ,
    delivery_state TEXT NOT NULL,
    body JSONB NOT NULL,
    parent_message_id UUID,
    audit_reason TEXT,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_role_mailbox_message_thread_created
    ON role_mailbox_message (thread_id, created_at_utc);

CREATE INDEX IF NOT EXISTS idx_role_mailbox_message_delivery_state
    ON role_mailbox_message (delivery_state);

-- MT-180 RoleMailboxClaimLeaseV1 schema with partial unique index enforcing
-- exactly-one-active-lease-per-thread at the database level (spec line 6156).
CREATE TABLE IF NOT EXISTS role_mailbox_claim_lease (
    lease_id UUID PRIMARY KEY NOT NULL,
    thread_id UUID NOT NULL REFERENCES role_mailbox_thread(thread_id) ON DELETE CASCADE,
    holder_executor_kind TEXT NOT NULL,
    holder_role_id TEXT NOT NULL,
    holder_session_id UUID NOT NULL,
    acquired_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at_utc TIMESTAMPTZ NOT NULL,
    released_at_utc TIMESTAMPTZ,
    takeover_of UUID REFERENCES role_mailbox_claim_lease(lease_id),
    takeover_reason TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_role_mailbox_claim_lease_active
    ON role_mailbox_claim_lease (thread_id)
    WHERE released_at_utc IS NULL;

CREATE INDEX IF NOT EXISTS idx_role_mailbox_claim_lease_holder
    ON role_mailbox_claim_lease (holder_session_id);

-- MT-183 handoff bundle table.
CREATE TABLE IF NOT EXISTS role_mailbox_handoff_bundle (
    bundle_id UUID PRIMARY KEY NOT NULL,
    source_thread_id UUID NOT NULL REFERENCES role_mailbox_thread(thread_id) ON DELETE CASCADE,
    source_message_id UUID NOT NULL,
    target_role TEXT NOT NULL,
    target_executor_kind TEXT NOT NULL,
    context_summary TEXT NOT NULL,
    linked_artifacts JSONB NOT NULL DEFAULT '[]'::jsonb,
    transcript_pointer JSONB,
    capability_grants JSONB NOT NULL DEFAULT '[]'::jsonb,
    expires_at_utc TIMESTAMPTZ,
    content_hash TEXT NOT NULL,
    created_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by_session UUID NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_role_mailbox_handoff_bundle_thread
    ON role_mailbox_handoff_bundle (source_thread_id);
