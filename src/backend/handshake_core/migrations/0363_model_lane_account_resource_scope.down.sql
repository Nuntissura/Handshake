-- Reverse of 0363_model_lane_account_resource_scope.sql.
--
-- Drops the account-bound resource scope seam from the WP-1 model-lane tables.
-- Indexes are dropped implicitly with their columns, but they are dropped
-- explicitly first so the intent is auditable and so a partially-applied
-- forward migration (columns present, indexes missing, or vice versa) still
-- reverses cleanly.
--
-- `owner_session` is deliberately NOT touched here: it predates this migration,
-- is real lineage, and is load-bearing for the idx_*_locus indexes and the
-- process_ledger reclaim join.

DO $migration_0363_down_scope_columns$
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
        IF to_regclass(format('public.%I', scoped_table)) IS NULL THEN
            CONTINUE;
        END IF;

        EXECUTE format('DROP INDEX IF EXISTS idx_%s_owner_scope', scoped_table);
        EXECUTE format('DROP INDEX IF EXISTS idx_%s_actor_principal', scoped_table);

        EXECUTE format(
            'ALTER TABLE %I
                 DROP COLUMN IF EXISTS owner_account_id,
                 DROP COLUMN IF EXISTS actor_principal_id,
                 DROP COLUMN IF EXISTS authenticated_session_id,
                 DROP COLUMN IF EXISTS access_space_id,
                 DROP COLUMN IF EXISTS workspace_id',
            scoped_table
        );
    END LOOP;
END
$migration_0363_down_scope_columns$;

DROP TABLE IF EXISTS model_lane_resource_scope_registry;
