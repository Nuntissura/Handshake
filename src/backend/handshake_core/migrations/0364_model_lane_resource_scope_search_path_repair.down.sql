-- Reverse of 0364_model_lane_resource_scope_search_path_repair.sql.
--
-- 0364 added the same columns/indexes as 0363, but in the CURRENT schema rather
-- than only in `public`. Reversing it therefore drops them schema-relatively.
-- Everything is `IF EXISTS`, so reversing in a database where 0363's down step
-- already removed them (or where they were never added) is a clean no-op.
--
-- `owner_session` is deliberately NOT touched: it predates both migrations, is
-- real lineage, and is load-bearing for the idx_*_locus indexes and the
-- process_ledger reclaim join.

DO $migration_0364_down_scope_columns$
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
        IF to_regclass(quote_ident(scoped_table)) IS NULL THEN
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
$migration_0364_down_scope_columns$;

DROP TABLE IF EXISTS model_lane_resource_scope_registry;
