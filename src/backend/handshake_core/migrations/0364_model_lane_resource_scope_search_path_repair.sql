-- WP-1 HBR-PRIV: repair 0363 so the scope columns land in the CURRENT schema.
--
-- WHY THIS IS A SEPARATE MIGRATION RATHER THAN AN EDIT TO 0363
-- `0363_model_lane_account_resource_scope.sql` guards every table with
-- `to_regclass(format('public.%I', scoped_table)) IS NULL THEN CONTINUE`. That
-- resolves ONLY against the `public` schema, so in any database where the WP-1
-- model-lane tables live somewhere else the guard reports "absent" and the
-- migration silently skips all 21 tables while still recording itself as
-- applied. That is exactly the topology of the PostgreSQL proof harness
-- (`tests/knowledge_pg_support.rs` gives each test an isolated
-- `knowledge_test_<uuidv7>` schema and runs the full chain with `search_path`
-- pointed at it), so 0363 added nothing there and every account-scope proof
-- failed with `column "owner_account_id" ... does not exist`.
--
-- 0363 is already recorded in `_sqlx_migrations`, and sqlx validates migration
-- checksums on every run: editing it in place would turn a working database
-- into a `VersionMismatch` boot failure. Applied migrations are immutable, so
-- the defect is corrected forward.
--
-- This migration is a strict superset no-op wherever 0363 actually worked:
-- every statement is `IF NOT EXISTS` / `ON CONFLICT DO NOTHING`, and the guard
-- is now `to_regclass(quote_ident(...))`, which resolves through `search_path`
-- and therefore finds the tables in whichever schema this database keeps them.
--
-- The column set, index names, nullability, and forward-compatibility contract
-- are unchanged from 0363: WP-KERNEL-006 MT-015 still owns the eventual
-- NOT NULL tightening and the FKs, MT-014/016 still own RLS, and WP-KERNEL-007
-- still owns AccessSpace semantics. Nothing here pre-empts them.

DO $migration_0364_scope_columns$
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
        -- Schema-relative, unlike 0363. A table that genuinely does not exist in
        -- this database (partially provisioned proof schema) still simply has
        -- nothing to scope yet.
        IF to_regclass(quote_ident(scoped_table)) IS NULL THEN
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
$migration_0364_scope_columns$;

-- 0363 also creates this table but only populates it for tables it found in
-- `public`, so outside `public` it exists and is empty. Repopulate it
-- schema-relatively so tools, tests, and the KERNEL-006 takeover can enumerate
-- the participating tables without re-parsing SQL.
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
WHERE to_regclass(quote_ident(scoped.table_name)) IS NOT NULL
ON CONFLICT (table_name) DO NOTHING;
