//! Regression proof for migration 0353 legacy `single_run` compatibility.
//!
//! Pre-0353 `single_run` authority was lane-bound. The upgrade must retain
//! those PostgreSQL projection rows and their append-only EventLedger payloads
//! without treating them as native v2 run-wide launch authority.

#[allow(dead_code)]
mod knowledge_pg_support;

use knowledge_pg_support::base_database_url;
use serde_json::{json, Value};
use sqlx::{Connection, Executor, Row};
use uuid::Uuid;

const LEGACY_RUN_ID: &str = "run-0353-legacy";
const LEGACY_STREAM_ID: &str = "stream-0353-legacy";

async fn insert_event(
    conn: &mut sqlx::PgConnection,
    event_id: &str,
    aggregate_type: &str,
    aggregate_id: &str,
    payload: &Value,
) -> i64 {
    sqlx::query_scalar(
        r#"
        INSERT INTO kernel_event_ledger (
            event_id, event_version, kernel_task_run_id, session_run_id,
            aggregate_type, aggregate_id, idempotency_key, event_type,
            actor_kind, actor_id, payload_hash, source_component, payload
        ) VALUES (
            $1, '1', 'wp-1-migration-proof', $2,
            $3, $4, $5, 'artifact_stored',
            'system', 'migration-proof', repeat('a', 64),
            'dexterity_model_lane', $6
        )
        RETURNING event_sequence
        "#,
    )
    .bind(event_id)
    .bind(LEGACY_STREAM_ID)
    .bind(aggregate_type)
    .bind(aggregate_id)
    .bind(format!("event:{event_id}"))
    .bind(payload)
    .fetch_one(conn)
    .await
    .expect("insert legacy EventLedger authority")
}

async fn insert_legacy_plan(
    conn: &mut sqlx::PgConnection,
    suffix: &str,
    lane_id: &str,
) -> (String, String) {
    let plan_id = format!("plan-{suffix}");
    let event_id = format!("event-plan-{suffix}");
    let record = json!({
        "projection_plan_id": plan_id,
        "run_id": LEGACY_RUN_ID,
        "lane_id": lane_id,
        "model_session_id": format!("session-{suffix}"),
        "provider_kind": "openai",
        "requested_model_id": "gpt-legacy",
        "consent_scope": "single_run",
        "legacy_marker": suffix
    });
    let payload = json!({
        "schema_id": "hsk.model_lane_cloud_projection_plan@1",
        "record": record
    });
    let sequence = insert_event(
        conn,
        &event_id,
        "model_lane_cloud_projection_plan",
        &plan_id,
        &payload,
    )
    .await;

    sqlx::query(
        r#"
        INSERT INTO model_lane_cloud_projection_plans (
            projection_plan_id, run_id, trace_id, lane_id,
            model_session_id, provider_kind, requested_model_id,
            scope_hash, source_artifact_refs, payload_artifact_ref,
            payload_sha256, redaction_policy_ref, redaction_summary,
            retention_policy, export_posture, provider_profile_ref,
            fan_out_targets, consent_scope, status,
            event_ledger_stream_id, work_packet_id, micro_task_id,
            task_board_id, owner_session, idempotency_key,
            created_at_utc, user_manual_behavior_ref, diagnostic_payload,
            projection_plan_hash, event_ledger_event_id, event_ledger_seq,
            event_stream_version, transaction_seq, record_json
        ) VALUES (
            $1, $2, 'trace-0353', $3,
            $4, 'openai', 'gpt-legacy',
            repeat('b', 64), '["artifact://legacy-source"]'::jsonb,
            'artifact://legacy-payload', repeat('c', 64),
            'redaction://legacy', 'legacy-redaction',
            'no_training_ephemeral', 'redacted_context_only',
            'provider://legacy', '["legacy-target"]'::jsonb,
            'single_run', 'active',
            $5, 'WP-1', 'MT-017', 'TASK-1', 'migration-proof',
            $6, '2026-01-01T00:00:00Z', 'usermanual://legacy',
            '{}'::jsonb, repeat('d', 64), $7, $8, $8, $8, $9
        )
        "#,
    )
    .bind(&plan_id)
    .bind(LEGACY_RUN_ID)
    .bind(lane_id)
    .bind(format!("session-{suffix}"))
    .bind(LEGACY_STREAM_ID)
    .bind(format!("plan-idempotency-{suffix}"))
    .bind(&event_id)
    .bind(sequence)
    .bind(&record)
    .execute(conn)
    .await
    .expect("insert pre-0353 legacy single_run projection plan");

    (plan_id, event_id)
}

async fn insert_legacy_receipt(
    conn: &mut sqlx::PgConnection,
    plan_id: &str,
    lane_id: &str,
) -> (String, String) {
    let receipt_id = "receipt-legacy-a".to_string();
    let event_id = "event-receipt-legacy-a".to_string();
    let record = json!({
        "consent_receipt_id": receipt_id,
        "projection_plan_id": plan_id,
        "run_id": LEGACY_RUN_ID,
        "lane_id": lane_id,
        "model_session_id": "session-a",
        "provider_kind": "openai",
        "requested_model_id": "gpt-legacy",
        "consent_scope": "single_run",
        "legacy_marker": "receipt-a"
    });
    let payload = json!({
        "schema_id": "hsk.model_lane_cloud_consent_receipt@1",
        "record": record
    });
    let sequence = insert_event(
        conn,
        &event_id,
        "model_lane_cloud_consent_receipt",
        &receipt_id,
        &payload,
    )
    .await;

    sqlx::query(
        r#"
        INSERT INTO model_lane_cloud_consent_receipts (
            consent_receipt_id, projection_plan_id, projection_plan_hash,
            run_id, trace_id, lane_id, model_session_id, provider_kind,
            requested_model_id, scope_hash, consent_scope, retention_policy,
            export_posture, fan_out_targets, approved, approved_by_ref,
            approved_at_utc, valid_from_utc, valid_until_utc,
            revoked_at_utc, revocation_ref, status,
            event_ledger_stream_id, work_packet_id, micro_task_id,
            task_board_id, owner_session, idempotency_key, created_at_utc,
            user_manual_behavior_ref, diagnostic_payload,
            consent_receipt_hash, event_ledger_event_id, event_ledger_seq,
            event_stream_version, transaction_seq, record_json
        ) VALUES (
            $1, $2, repeat('d', 64),
            $3, 'trace-0353', $4, 'session-a', 'openai',
            'gpt-legacy', repeat('b', 64), 'single_run',
            'no_training_ephemeral', 'redacted_context_only',
            '["legacy-target"]'::jsonb, TRUE, 'operator://legacy',
            '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z',
            '2027-01-01T00:00:00Z', NULL, NULL, 'approved',
            $5, 'WP-1', 'MT-017', 'TASK-1', 'migration-proof',
            'receipt-idempotency-a', '2026-01-01T00:00:00Z',
            'usermanual://legacy', '{}'::jsonb, repeat('e', 64),
            $6, $7, $7, $7, $8
        )
        "#,
    )
    .bind(&receipt_id)
    .bind(plan_id)
    .bind(LEGACY_RUN_ID)
    .bind(lane_id)
    .bind(LEGACY_STREAM_ID)
    .bind(&event_id)
    .bind(sequence)
    .bind(&record)
    .execute(conn)
    .await
    .expect("insert pre-0353 legacy single_run consent receipt");

    (receipt_id, event_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn migration_0353_preserves_legacy_single_run_authority_and_down_fails_closed() {
    let base_url = base_database_url()
        .await
        .expect("0353 migration proof requires real PostgreSQL");
    let schema = format!("model_lane_0353_{}", Uuid::now_v7().simple());
    let mut conn = sqlx::PgConnection::connect(&base_url)
        .await
        .expect("connect to PostgreSQL for 0353 proof");
    conn.execute(format!("CREATE SCHEMA {schema}").as_str())
        .await
        .expect("create 0353 scratch schema");
    conn.execute(format!("SET search_path TO {schema}").as_str())
        .await
        .expect("select 0353 scratch schema");

    sqlx::raw_sql(include_str!("../migrations/0018_kernel_event_ledger.sql"))
        .execute(&mut conn)
        .await
        .expect("apply EventLedger dependency");
    sqlx::raw_sql(
        r#"
        CREATE TABLE model_lane_schema_registry (
            schema_id TEXT PRIMARY KEY,
            schema_version INTEGER NOT NULL,
            record_kind TEXT NOT NULL,
            table_name TEXT NOT NULL,
            source_component TEXT NOT NULL DEFAULT 'dexterity_model_lane',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(&mut conn)
    .await
    .expect("create schema-registry dependency");
    sqlx::raw_sql(include_str!(
        "../migrations/0341_model_lane_cloud_projection_consent.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("apply pre-0353 cloud consent schema");

    let (plan_a, plan_event_a) = insert_legacy_plan(&mut conn, "a", "lane-a").await;
    let (_plan_b, _plan_event_b) = insert_legacy_plan(&mut conn, "b", "lane-b").await;
    let (_receipt_a, receipt_event_a) = insert_legacy_receipt(&mut conn, &plan_a, "lane-a").await;

    let payloads_before: Vec<(String, Value)> =
        sqlx::query("SELECT event_id, payload FROM kernel_event_ledger ORDER BY event_sequence")
            .fetch_all(&mut conn)
            .await
            .expect("read legacy ledger payloads")
            .into_iter()
            .map(|row| (row.get("event_id"), row.get("payload")))
            .collect();
    let plan_record_before: Value = sqlx::query_scalar(
        "SELECT record_json FROM model_lane_cloud_projection_plans WHERE projection_plan_id = $1",
    )
    .bind(&plan_a)
    .fetch_one(&mut conn)
    .await
    .expect("read legacy plan projection");

    sqlx::raw_sql(include_str!(
        "../migrations/0353_model_lane_run_scoped_consent.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("0353 must upgrade populated pre-0353 single_run authority");

    let legacy_plan_shapes: Vec<(String, String, String, String)> = sqlx::query_as(
        r#"
        SELECT lane_id, model_session_id, provider_kind, requested_model_id
        FROM model_lane_cloud_projection_plans
        WHERE run_id = $1 AND consent_scope = 'single_run'
        ORDER BY lane_id
        "#,
    )
    .bind(LEGACY_RUN_ID)
    .fetch_all(&mut conn)
    .await
    .expect("read upgraded legacy plans");
    assert_eq!(legacy_plan_shapes.len(), 2);
    assert_eq!(legacy_plan_shapes[0].0, "lane-a");
    assert_eq!(legacy_plan_shapes[1].0, "lane-b");

    let payloads_after: Vec<(String, Value)> =
        sqlx::query("SELECT event_id, payload FROM kernel_event_ledger ORDER BY event_sequence")
            .fetch_all(&mut conn)
            .await
            .expect("read ledger payloads after 0353")
            .into_iter()
            .map(|row| (row.get("event_id"), row.get("payload")))
            .collect();
    assert_eq!(
        payloads_after, payloads_before,
        "0353 must not rewrite or append EventLedger authority"
    );
    let plan_record_after: Value = sqlx::query_scalar(
        "SELECT record_json FROM model_lane_cloud_projection_plans WHERE projection_plan_id = $1",
    )
    .bind(&plan_a)
    .fetch_one(&mut conn)
    .await
    .expect("read legacy plan projection after 0353");
    assert_eq!(plan_record_after, plan_record_before);

    let partial_legacy_update = sqlx::query(
        "UPDATE model_lane_cloud_projection_plans SET model_session_id = NULL WHERE projection_plan_id = $1",
    )
    .bind(&plan_a)
    .execute(&mut conn)
    .await;
    assert!(
        partial_legacy_update.is_err(),
        "0353 must reject mixed partial legacy/native single_run bindings"
    );

    let v2_event_id = "event-plan-native-v2";
    let v2_sequence = insert_event(
        &mut conn,
        v2_event_id,
        "model_lane_cloud_projection_plan",
        "plan-native-v2",
        &json!({"schema_id": "hsk.model_lane_cloud_projection_plan@2"}),
    )
    .await;
    sqlx::query(
        r#"
        INSERT INTO model_lane_cloud_projection_plans (
            projection_plan_id, run_id, trace_id, scope_hash,
            source_artifact_refs, payload_artifact_ref, payload_sha256,
            redaction_policy_ref, redaction_summary, retention_policy,
            export_posture, provider_profile_ref, fan_out_targets,
            consent_scope, status, event_ledger_stream_id, work_packet_id,
            micro_task_id, task_board_id, owner_session, idempotency_key,
            created_at_utc, user_manual_behavior_ref, diagnostic_payload,
            projection_plan_hash, event_ledger_event_id, event_ledger_seq,
            event_stream_version, transaction_seq, record_json
        ) VALUES (
            'plan-native-v2', $1, 'trace-v2', repeat('1',64),
            '[]'::jsonb, 'artifact://v2', repeat('2',64),
            'redaction://v2', 'v2', 'no_training_ephemeral',
            'redacted_context_only', 'provider://v2', '[]'::jsonb,
            'single_run', 'active', $2, 'WP-1', 'MT-017', 'TASK-1',
            'migration-proof', 'plan-native-v2-idempotency',
            '2026-01-01T00:00:00Z', 'usermanual://v2', '{}'::jsonb,
            repeat('3',64), $3, $4, $4, $4, '{}'::jsonb
        )
        "#,
    )
    .bind(LEGACY_RUN_ID)
    .bind(LEGACY_STREAM_ID)
    .bind(v2_event_id)
    .bind(v2_sequence)
    .execute(&mut conn)
    .await
    .expect("one native v2 run-wide plan may coexist with retained legacy rows");

    let duplicate_v2_event_id = "event-plan-native-v2-duplicate";
    let duplicate_v2_sequence = insert_event(
        &mut conn,
        duplicate_v2_event_id,
        "model_lane_cloud_projection_plan",
        "plan-native-v2-duplicate",
        &json!({"schema_id": "hsk.model_lane_cloud_projection_plan@2"}),
    )
    .await;
    let duplicate_v2 = sqlx::query(
        r#"
        INSERT INTO model_lane_cloud_projection_plans
            (projection_plan_id, run_id, trace_id, scope_hash,
             source_artifact_refs, payload_artifact_ref, payload_sha256,
             redaction_policy_ref, redaction_summary, retention_policy,
             export_posture, provider_profile_ref, fan_out_targets,
             consent_scope, status, event_ledger_stream_id, work_packet_id,
             micro_task_id, task_board_id, owner_session, idempotency_key,
             created_at_utc, user_manual_behavior_ref, diagnostic_payload,
             projection_plan_hash, event_ledger_event_id, event_ledger_seq,
             event_stream_version, transaction_seq, record_json)
        SELECT
            'plan-native-v2-duplicate', run_id, trace_id, scope_hash,
            source_artifact_refs, payload_artifact_ref, payload_sha256,
            redaction_policy_ref, redaction_summary, retention_policy,
            export_posture, provider_profile_ref, fan_out_targets,
            consent_scope, status, event_ledger_stream_id, work_packet_id,
            micro_task_id, task_board_id, owner_session,
            'plan-native-v2-duplicate-idempotency', created_at_utc,
            user_manual_behavior_ref, diagnostic_payload,
            projection_plan_hash, $1, $2, $2, $2, record_json
        FROM model_lane_cloud_projection_plans
        WHERE projection_plan_id = 'plan-native-v2'
        "#,
    )
    .bind(duplicate_v2_event_id)
    .bind(duplicate_v2_sequence)
    .execute(&mut conn)
    .await
    .expect_err("v2 keeps one run-wide plan per run");
    assert_eq!(
        duplicate_v2
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("uq_model_lane_cloud_projection_plans_single_run_v2")
    );

    let down_error = sqlx::raw_sql(include_str!(
        "../migrations/0353_model_lane_run_scoped_consent.down.sql"
    ))
    .execute(&mut conn)
    .await
    .expect_err("0353 down must refuse while legacy or v2 authority exists");
    assert!(
        down_error
            .to_string()
            .contains("cannot downgrade 0353 while any cloud plan/receipt authority exists"),
        "unexpected down-migration error: {down_error}"
    );
    let retained_events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM kernel_event_ledger WHERE event_id IN ($1, $2)")
            .bind(&plan_event_a)
            .bind(&receipt_event_a)
            .fetch_one(&mut conn)
            .await
            .expect("verify authority after refused down migration");
    assert_eq!(retained_events, 2);
    let target_bindings_column_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = $1
              AND table_name = 'model_lane_cloud_projection_plans'
              AND column_name = 'target_bindings'
        )
        "#,
    )
    .bind(&schema)
    .fetch_one(&mut conn)
    .await
    .expect("verify refused down migration remained atomic");
    assert!(target_bindings_column_exists);

    conn.execute("SET search_path TO public")
        .await
        .expect("leave scratch schema");
    conn.execute(format!("DROP SCHEMA {schema} CASCADE").as_str())
        .await
        .expect("drop 0353 scratch schema");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn migration_0353_down_restores_v1_shape_when_authority_is_empty() {
    let base_url = base_database_url()
        .await
        .expect("0353 down-migration proof requires real PostgreSQL");
    let schema = format!("model_lane_0353_down_{}", Uuid::now_v7().simple());
    let mut conn = sqlx::PgConnection::connect(&base_url)
        .await
        .expect("connect to PostgreSQL for 0353 down proof");
    conn.execute(format!("CREATE SCHEMA {schema}").as_str())
        .await
        .expect("create 0353 down scratch schema");
    conn.execute(format!("SET search_path TO {schema}").as_str())
        .await
        .expect("select 0353 down scratch schema");

    sqlx::raw_sql(include_str!("../migrations/0018_kernel_event_ledger.sql"))
        .execute(&mut conn)
        .await
        .expect("apply EventLedger dependency");
    sqlx::raw_sql(
        r#"
        CREATE TABLE model_lane_schema_registry (
            schema_id TEXT PRIMARY KEY,
            schema_version INTEGER NOT NULL,
            record_kind TEXT NOT NULL,
            table_name TEXT NOT NULL,
            source_component TEXT NOT NULL DEFAULT 'dexterity_model_lane',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(&mut conn)
    .await
    .expect("create schema-registry dependency");
    sqlx::raw_sql(include_str!(
        "../migrations/0341_model_lane_cloud_projection_consent.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("apply pre-0353 cloud consent schema");
    sqlx::raw_sql(include_str!(
        "../migrations/0353_model_lane_run_scoped_consent.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("apply 0353 on empty authority");
    sqlx::raw_sql(include_str!(
        "../migrations/0353_model_lane_run_scoped_consent.down.sql"
    ))
    .execute(&mut conn)
    .await
    .expect("0353 down succeeds when no cloud authority exists");

    let target_bindings_column_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_schema = $1
              AND table_name = 'model_lane_cloud_projection_plans'
              AND column_name = 'target_bindings'
        )
        "#,
    )
    .bind(&schema)
    .fetch_one(&mut conn)
    .await
    .expect("inspect restored v1 columns");
    assert!(!target_bindings_column_exists);
    let v1_lane_nullable: String = sqlx::query_scalar(
        r#"
        SELECT is_nullable
        FROM information_schema.columns
        WHERE table_schema = $1
          AND table_name = 'model_lane_cloud_projection_plans'
          AND column_name = 'lane_id'
        "#,
    )
    .bind(&schema)
    .fetch_one(&mut conn)
    .await
    .expect("inspect restored v1 lane nullability");
    assert_eq!(v1_lane_nullable, "NO");
    let v2_registry_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lane_schema_registry WHERE schema_id LIKE '%@2'",
    )
    .fetch_one(&mut conn)
    .await
    .expect("inspect restored v1 registry");
    assert_eq!(v2_registry_rows, 0);

    conn.execute("SET search_path TO public")
        .await
        .expect("leave down scratch schema");
    conn.execute(format!("DROP SCHEMA {schema} CASCADE").as_str())
        .await
        .expect("drop 0353 down scratch schema");
}
