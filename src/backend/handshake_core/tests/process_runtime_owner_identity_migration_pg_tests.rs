//! Cast-safety regression proof for migration 0359 legacy JSON backfill.

#[allow(dead_code)]
mod knowledge_pg_support;

use serde_json::json;
use sqlx::{Connection, Executor, Row};
use uuid::Uuid;

use handshake_core::process_ledger::{
    LedgerEvent, PostgresProcessLedgerStore, ProcessEngineKind, ProcessLedgerStore,
    ProcessRuntimeOwner, ProcessStart, ProcessStop,
};

#[tokio::test]
async fn migration_0359_skips_malformed_and_oversized_legacy_owner_metadata() {
    let Some(database_url) = knowledge_pg_support::base_database_url().await else {
        eprintln!("SKIPPED migration_0359 cast-safety proof: PostgreSQL unavailable");
        return;
    };
    let mut connection = sqlx::PgConnection::connect(&database_url)
        .await
        .expect("connect migration 0359 cast-safety proof");
    sqlx::raw_sql(
        r#"
        CREATE TEMP TABLE kernel_process_lifecycle (
            process_uuid UUID PRIMARY KEY,
            parent_session_id TEXT,
            started_at TIMESTAMPTZ NOT NULL,
            stopped_at TIMESTAMPTZ,
            metadata_jsonb JSONB NOT NULL DEFAULT '{}'::jsonb
        )
        "#,
    )
    .execute(&mut connection)
    .await
    .expect("create pre-0359 lifecycle table");

    let valid_process = Uuid::now_v7();
    let valid_runtime = Uuid::now_v7();
    let nonnumeric_process = Uuid::now_v7();
    let oversized_process = Uuid::now_v7();
    let malformed_uuid_process = Uuid::now_v7();
    let owner_metadata = |runtime_id: String, port: serde_json::Value| {
        json!({
            "runtime_instance_id": runtime_id,
            "runtime_host_scope_id": "host-scope-proof",
            "runtime_instance_schema_id": "hsk.embedded_runtime.instance@2",
            "runtime_lease_protocol": "tcp-loopback-connect-v1",
            "runtime_lease_address": "127.0.0.1",
            "runtime_lease_port": port,
        })
    };
    for (process_id, metadata) in [
        (
            valid_process,
            owner_metadata(valid_runtime.to_string(), json!(32123)),
        ),
        (
            nonnumeric_process,
            owner_metadata(Uuid::now_v7().to_string(), json!("not-a-number")),
        ),
        (
            oversized_process,
            owner_metadata(
                Uuid::now_v7().to_string(),
                json!("999999999999999999999999999999999999999999999999"),
            ),
        ),
        (
            malformed_uuid_process,
            owner_metadata("not-a-uuid".to_string(), json!(32124)),
        ),
    ] {
        sqlx::query(
            r#"
            INSERT INTO kernel_process_lifecycle (
                process_uuid, parent_session_id, started_at, metadata_jsonb
            ) VALUES ($1, 'migration-0359-proof', NOW(), $2)
            "#,
        )
        .bind(process_id)
        .bind(metadata)
        .execute(&mut connection)
        .await
        .expect("insert legacy owner metadata row");
    }

    sqlx::raw_sql(include_str!(
        "../migrations/0359_process_runtime_owner_identity.sql"
    ))
    .execute(&mut connection)
    .await
    .expect("migration 0359 must never abort on malformed legacy casts");

    let valid = sqlx::query(
        r#"
        SELECT owner_runtime_instance_id, owner_lease_port
        FROM kernel_process_lifecycle
        WHERE process_uuid = $1
        "#,
    )
    .bind(valid_process)
    .fetch_one(&mut connection)
    .await
    .expect("read valid backfill row");
    assert_eq!(
        valid.get::<Option<Uuid>, _>("owner_runtime_instance_id"),
        Some(valid_runtime)
    );
    assert_eq!(valid.get::<Option<i32>, _>("owner_lease_port"), Some(32123));

    for process_id in [
        nonnumeric_process,
        oversized_process,
        malformed_uuid_process,
    ] {
        let invalid = sqlx::query(
            r#"
            SELECT owner_runtime_instance_id, owner_lease_port
            FROM kernel_process_lifecycle
            WHERE process_uuid = $1
            "#,
        )
        .bind(process_id)
        .fetch_one(&mut connection)
        .await
        .expect("read rejected legacy backfill row");
        assert_eq!(
            invalid.get::<Option<Uuid>, _>("owner_runtime_instance_id"),
            None
        );
        assert_eq!(invalid.get::<Option<i32>, _>("owner_lease_port"), None);
    }
}

#[tokio::test]
async fn migration_0359_rejects_conflicting_descriptor_for_one_runtime_uuid() {
    let Some(database_url) = knowledge_pg_support::base_database_url().await else {
        eprintln!("SKIPPED migration_0359 descriptor guard proof: PostgreSQL unavailable");
        return;
    };
    let mut connection = sqlx::PgConnection::connect(&database_url)
        .await
        .expect("connect migration 0359 descriptor guard proof");
    sqlx::raw_sql(
        r#"
        CREATE TEMP TABLE kernel_process_lifecycle (
            process_uuid UUID PRIMARY KEY,
            parent_session_id TEXT,
            started_at TIMESTAMPTZ NOT NULL,
            stopped_at TIMESTAMPTZ,
            metadata_jsonb JSONB NOT NULL DEFAULT '{}'::jsonb
        )
        "#,
    )
    .execute(&mut connection)
    .await
    .expect("create pre-0359 lifecycle table");
    sqlx::raw_sql(include_str!(
        "../migrations/0359_process_runtime_owner_identity.sql"
    ))
    .execute(&mut connection)
    .await
    .expect("apply migration 0359 descriptor guard");

    let runtime_id = Uuid::now_v7();
    let insert = r#"
        INSERT INTO kernel_process_lifecycle (
            process_uuid, parent_session_id, started_at,
            owner_runtime_instance_id, owner_host_scope_id,
            owner_lease_schema_id, owner_lease_protocol,
            owner_lease_address, owner_lease_port
        ) VALUES ($1, 'descriptor-guard-proof', NOW(), $2, 'host-scope-proof',
                  'hsk.embedded_runtime.instance@2', 'udp-loopback-bind-v1',
                  '127.0.0.1', $3)
    "#;
    sqlx::query(insert)
        .bind(Uuid::now_v7())
        .bind(runtime_id)
        .bind(32123_i32)
        .execute(&mut connection)
        .await
        .expect("insert canonical runtime descriptor");

    let conflicting = sqlx::query(insert)
        .bind(Uuid::now_v7())
        .bind(runtime_id)
        .bind(32124_i32)
        .execute(&mut connection)
        .await
        .expect_err("same runtime UUID with a different lease descriptor must be rejected");
    assert_eq!(
        conflicting
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some("23514")
    );
}

#[tokio::test]
async fn migration_0359_quarantines_and_repairs_preexisting_descriptor_conflicts() {
    let database_url = knowledge_pg_support::base_database_url()
        .await
        .expect("migration 0359 pre-existing conflict proof requires real PostgreSQL");
    let mut connection = sqlx::PgConnection::connect(&database_url)
        .await
        .expect("connect migration 0359 pre-existing conflict proof");
    sqlx::raw_sql(
        r#"
        CREATE TEMP TABLE kernel_process_lifecycle (
            process_uuid UUID PRIMARY KEY,
            parent_session_id TEXT,
            started_at TIMESTAMPTZ NOT NULL,
            stopped_at TIMESTAMPTZ,
            metadata_jsonb JSONB NOT NULL DEFAULT '{}'::jsonb
        )
        "#,
    )
    .execute(&mut connection)
    .await
    .expect("create pre-0359 lifecycle table");

    let runtime_id = Uuid::now_v7();
    let first_process = Uuid::now_v7();
    let second_process = Uuid::now_v7();
    let metadata = |port: i32| {
        json!({
            "runtime_instance_id": runtime_id,
            "runtime_host_scope_id": "host-scope-proof",
            "runtime_instance_schema_id": "hsk.embedded_runtime.instance@2",
            "runtime_lease_protocol": "tcp-loopback-connect-v1",
            "runtime_lease_address": "127.0.0.1",
            "runtime_lease_port": port,
        })
    };
    for (process_uuid, descriptor) in [
        (first_process, metadata(32123)),
        (second_process, metadata(32124)),
    ] {
        sqlx::query(
            r#"
            INSERT INTO kernel_process_lifecycle (
                process_uuid, parent_session_id, started_at, metadata_jsonb
            ) VALUES ($1, 'migration-0359-preexisting-conflict', NOW(), $2)
            "#,
        )
        .bind(process_uuid)
        .bind(descriptor)
        .execute(&mut connection)
        .await
        .expect("insert conflicting pre-0359 owner metadata");
    }

    sqlx::raw_sql(include_str!(
        "../migrations/0359_process_runtime_owner_identity.sql"
    ))
    .execute(&mut connection)
    .await
    .expect("migration 0359 quarantines pre-existing conflicts");

    let quarantined: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_process_runtime_owner_legacy_quarantine
        WHERE runtime_instance_id = $1
          AND pg_catalog.jsonb_array_length(conflicting_descriptors_jsonb) = 2
          AND repair_hint LIKE '%rerun migration 0359%'
        "#,
    )
    .bind(runtime_id)
    .fetch_one(&mut connection)
    .await
    .expect("read migration 0359 quarantine diagnostics");
    assert_eq!(quarantined, 2);
    let typed_while_quarantined: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_process_lifecycle WHERE owner_runtime_instance_id = $1",
    )
    .bind(runtime_id)
    .fetch_one(&mut connection)
    .await
    .expect("read quarantined typed-owner count");
    assert_eq!(typed_while_quarantined, 0);

    sqlx::query(
        r#"
        UPDATE kernel_process_lifecycle
        SET metadata_jsonb = jsonb_set(metadata_jsonb, '{runtime_lease_port}', '32123'::jsonb)
        WHERE process_uuid = $1
        "#,
    )
    .bind(second_process)
    .execute(&mut connection)
    .await
    .expect("repair conflicting legacy descriptor");
    sqlx::raw_sql(include_str!(
        "../migrations/0359_process_runtime_owner_identity.sql"
    ))
    .execute(&mut connection)
    .await
    .expect("rerun migration 0359 after repairing legacy descriptors");

    let quarantine_after_repair: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_process_runtime_owner_legacy_quarantine WHERE runtime_instance_id = $1",
    )
    .bind(runtime_id)
    .fetch_one(&mut connection)
    .await
    .expect("read repaired quarantine count");
    assert_eq!(quarantine_after_repair, 0);
    let typed_after_repair: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_process_lifecycle
        WHERE owner_runtime_instance_id = $1
          AND owner_lease_port = 32123
        "#,
    )
    .bind(runtime_id)
    .fetch_one(&mut connection)
    .await
    .expect("read repaired typed-owner count");
    assert_eq!(typed_after_repair, 2);
}

#[tokio::test]
async fn migration_0359_exact_guard_allows_real_postgres_start_and_stop() {
    let pg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("migration 0359 START/STOP proof requires real PostgreSQL");
    let pool = sqlx::PgPool::connect(&pg.schema_url)
        .await
        .expect("connect migration 0359 START/STOP proof pool");
    let store = PostgresProcessLedgerStore::new(pool.clone());
    let start = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "migration-0359-start-stop-proof",
        Some("WP-1".to_string()),
    )
    .with_parent_session_id("migration-0359-start-stop-proof")
    .with_runtime_owner(ProcessRuntimeOwner {
        runtime_instance_id: Uuid::now_v7(),
        host_scope_id: "migration-0359-host".to_string(),
        lease_schema_id: "hsk.embedded_runtime.instance@2".to_string(),
        lease_protocol: "tcp-loopback-connect-v1".to_string(),
        lease_address: "127.0.0.1".to_string(),
        lease_port: 32123,
    });
    let stop = ProcessStop::from_start(&start, Some(0)).with_stop_reason("proof-completed");
    store
        .write_batch(vec![LedgerEvent::Start(start.clone())])
        .await
        .expect("exact migration-0359 authority accepts durable START");
    store
        .write_batch(vec![LedgerEvent::Stop(stop)])
        .await
        .expect("exact migration-0359 authority accepts durable STOP");

    let row = sqlx::query(
        r#"
        SELECT stopped_at, exit_code, stop_reason, owner_runtime_instance_id
        FROM kernel_process_lifecycle
        WHERE process_uuid = $1
        "#,
    )
    .bind(start.process_uuid)
    .fetch_one(&pool)
    .await
    .expect("read migration-0359 START/STOP proof row");
    assert!(row
        .get::<Option<chrono::DateTime<chrono::Utc>>, _>("stopped_at")
        .is_some());
    assert_eq!(row.get::<Option<i32>, _>("exit_code"), Some(0));
    assert_eq!(
        row.get::<Option<String>, _>("stop_reason").as_deref(),
        Some("proof-completed")
    );
    assert_eq!(
        row.get::<Option<Uuid>, _>("owner_runtime_instance_id"),
        start.runtime_owner.map(|owner| owner.runtime_instance_id)
    );
}
