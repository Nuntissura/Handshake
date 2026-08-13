-- WP-1 MT-023: durable, account-scoped worktree -> microVM ownership.
--
-- The process handle and latest snapshot are serialized from the same typed
-- sandbox values the runtime uses.  A registry restart can therefore resolve
-- the exact VM through the owning adapter, or fail closed with a named
-- adoption error when that adapter instance cannot recover the handle.

CREATE TABLE IF NOT EXISTS worktree_vm_bindings (
    binding_id UUID PRIMARY KEY,
    worktree_id TEXT NOT NULL,
    adapter_id TEXT NOT NULL,
    process_handle JSONB NOT NULL,
    latest_snapshot JSONB,
    binding_state TEXT NOT NULL,
    generation BIGINT NOT NULL DEFAULT 1,
    failure_reason TEXT,
    owner_account_id UUID NOT NULL,
    actor_principal_id UUID NOT NULL,
    authenticated_session_id UUID NOT NULL,
    access_space_id UUID NOT NULL,
    workspace_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_worktree_vm_binding_worktree
        CHECK (length(btrim(worktree_id)) > 0),
    CONSTRAINT chk_worktree_vm_binding_state
        CHECK (binding_state IN ('active', 'snapshotted', 'terminated', 'failed')),
    CONSTRAINT uq_worktree_vm_binding_scope
        UNIQUE (owner_account_id, workspace_id, worktree_id)
);

CREATE INDEX IF NOT EXISTS idx_worktree_vm_bindings_owner_workspace_state
    ON worktree_vm_bindings (owner_account_id, workspace_id, binding_state);

CREATE INDEX IF NOT EXISTS idx_worktree_vm_bindings_actor
    ON worktree_vm_bindings (actor_principal_id);

INSERT INTO model_lane_resource_scope_registry (table_name)
VALUES ('worktree_vm_bindings')
ON CONFLICT (table_name) DO NOTHING;
