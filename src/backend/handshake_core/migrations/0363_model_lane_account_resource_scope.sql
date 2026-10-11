-- WP-1 HBR-PRIV: account-bound resource scope columns for model-lane authority.
--
-- WHY THIS EXISTS
-- HANDSHAKE_BUILD_RULES v1.8.0 (2026-07-26) added the PRIV pillar
-- (HBR-PRIV-001..008): every durable product resource must carry a stable
-- resource identity plus an authoritative owning-account / Principal /
-- AccessSpace visibility linkage before it is discoverable or usable.
--
-- The WP-1 model-lane tables were created before that pillar existed and carry
-- no account-bound scope at all. `owner_session` is NOT an owner: it is a
-- governance ROLE LABEL (literals such as 'swarm_coordinator' / 'KERNEL_BUILDER'
-- assigned at swarm_orchestration/model_lane.rs from SpawnRequest.owner_role),
-- identical for every operator on every machine, and dangling after restart on
-- the operator-chat path. It is retained here unchanged because it is real
-- lineage and load-bearing for the idx_*_locus indexes and the reclaim join in
-- process_ledger/reclaim.rs.
--
-- FORWARD-COMPATIBILITY CONTRACT (deliberate, do not "improve" these names)
-- WP-KERNEL-006 MT-015 `AuthorityTableOwnershipColumns` is the declared owner of
-- account/Principal/session/AccessSpace columns on Kernel V1 authority tables.
-- This migration adds exactly the columns that MT-015 will later populate and
-- constrain, so KERNEL-006 takes them over instead of colliding with a second,
-- competing authority model. Therefore this migration deliberately does NOT:
--   * create a local_accounts / principals / access_spaces table
--     (WP-KERNEL-006 MT-005 PostgresAccountPrincipalSchema owns the parents),
--   * add FOREIGN KEY constraints (no parent table exists yet; KERNEL-006 adds
--     them together with the parents),
--   * define any AccessSpace semantics — access_space_id is carried as an
--     opaque nullable seam only (WP-KERNEL-007 owns its meaning),
--   * enable ROW LEVEL SECURITY or write policies
--     (WP-KERNEL-006 MT-014/016 own FORCE ROW LEVEL SECURITY and the policy
--     matrix). WP-1 enforcement is application-layer at the store/API boundary
--     and is explicitly pre-RLS.
--
-- Columns are NULLable on purpose: existing rows predate account identity and
-- there is no account authority to backfill them from yet. NOT NULL is
-- KERNEL-006's tightening step once LocalAccount exists. Enforcement in WP-1
-- fails closed in application code rather than trusting a nullable column.
--
-- workspace_id is included because it is the ONLY scope key resolvable today
-- (see migrations/0311_parallel_swarm_state_recovery.sql, which already carries
-- workspace_id + actor_id on the adjacent WP-KERNEL-009 swarm table). It lets
-- WP-1 prove real two-scope isolation before accounts exist.

DO $migration_0363_scope_columns$
DECLARE
    scoped_table TEXT;
    scoped_tables CONSTANT TEXT[] := ARRAY[
        'model_lane_runs',
        'model_lanes',
        'model_lane_messages',
        'model_lane_promotion_decisions',
        'model_lane_context_bundle_artifacts',
        'model_lane_context_bundle_handoffs',
        'model_lane_cloud_projection_plans',
        'model_lane_cloud_consent_receipts',
        'model_lane_recovery_checkpoints',
        'model_lane_recovery_events',
        'model_lane_leases',
        'model_lane_diagnostic_tier_statuses',
        'model_lane_mt_runtime_statuses',
        'model_lane_routing_executions',
        'model_lane_routing_stage_attempts',
        'model_lane_routing_outbox',
        'model_runtime_registry',
        'model_runtime_active_selection',
        'swarm_session_cleanup_receipts',
        'swarm_terminal_event_outbox',
        'palmistry_durable_verifier'
    ];
BEGIN
    FOREACH scoped_table IN ARRAY scoped_tables LOOP
        -- Skip tables that are not present in this database. Migration ranges
        -- are applied in order, but a partially-provisioned test database must
        -- not abort the whole migration; a missing table simply has nothing to
        -- scope yet.
        IF to_regclass(format('public.%I', scoped_table)) IS NULL THEN
            CONTINUE;
        END IF;

        EXECUTE format(
            'ALTER TABLE %I
                 ADD COLUMN IF NOT EXISTS owner_account_id UUID,
                 ADD COLUMN IF NOT EXISTS actor_principal_id UUID,
                 ADD COLUMN IF NOT EXISTS authenticated_session_id UUID,
                 ADD COLUMN IF NOT EXISTS access_space_id UUID,
                 ADD COLUMN IF NOT EXISTS workspace_id TEXT',
            scoped_table
        );

        -- Scope-first index: every default-deny read filters on the owning
        -- account and (where present) the workspace scope, so this is the
        -- access path, not a reporting convenience.
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS idx_%s_owner_scope
                 ON %I (owner_account_id, workspace_id)',
            scoped_table,
            scoped_table
        );

        -- Principal attribution is queried independently of ownership when
        -- answering "what did this actor do" (HBR-PRIV-005).
        EXECUTE format(
            'CREATE INDEX IF NOT EXISTS idx_%s_actor_principal
                 ON %I (actor_principal_id)',
            scoped_table,
            scoped_table
        );
    END LOOP;
END
$migration_0363_scope_columns$;

-- Machine-readable declaration of the scope contract so tools, tests, and the
-- KERNEL-006 takeover can enumerate exactly which tables participate and which
-- authority owns the next step, without re-parsing this file.
CREATE TABLE IF NOT EXISTS model_lane_resource_scope_registry (
    table_name TEXT PRIMARY KEY,
    scope_schema_id TEXT NOT NULL DEFAULT 'hsk.model_lane_resource_scope@1',
    owner_column TEXT NOT NULL DEFAULT 'owner_account_id',
    principal_column TEXT NOT NULL DEFAULT 'actor_principal_id',
    session_column TEXT NOT NULL DEFAULT 'authenticated_session_id',
    access_space_column TEXT NOT NULL DEFAULT 'access_space_id',
    workspace_column TEXT NOT NULL DEFAULT 'workspace_id',
    enforcement_layer TEXT NOT NULL DEFAULT 'APPLICATION_PRE_RLS',
    rls_owner TEXT NOT NULL DEFAULT 'WP-KERNEL-006-MT-014',
    column_owner TEXT NOT NULL DEFAULT 'WP-KERNEL-006-MT-015',
    access_space_semantics_owner TEXT NOT NULL DEFAULT 'WP-KERNEL-007',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_model_lane_resource_scope_enforcement
        CHECK (enforcement_layer IN ('APPLICATION_PRE_RLS', 'POSTGRES_RLS'))
);

INSERT INTO model_lane_resource_scope_registry (table_name)
SELECT scoped.table_name
FROM unnest(ARRAY[
    'model_lane_runs',
    'model_lanes',
    'model_lane_messages',
    'model_lane_promotion_decisions',
    'model_lane_context_bundle_artifacts',
    'model_lane_context_bundle_handoffs',
    'model_lane_cloud_projection_plans',
    'model_lane_cloud_consent_receipts',
    'model_lane_recovery_checkpoints',
    'model_lane_recovery_events',
    'model_lane_leases',
    'model_lane_diagnostic_tier_statuses',
    'model_lane_mt_runtime_statuses',
    'model_lane_routing_executions',
    'model_lane_routing_stage_attempts',
    'model_lane_routing_outbox',
    'model_runtime_registry',
    'model_runtime_active_selection',
    'swarm_session_cleanup_receipts',
    'swarm_terminal_event_outbox',
    'palmistry_durable_verifier'
]) AS scoped(table_name)
WHERE to_regclass(format('public.%I', scoped.table_name)) IS NOT NULL
ON CONFLICT (table_name) DO NOTHING;
