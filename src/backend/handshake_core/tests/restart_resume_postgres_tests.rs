use chrono::Utc;
use handshake_core::process_ledger::restart_resume::PostgresRestartResumeRunner;
use serde_json::{json, Value};
use sqlx::{Connection, Row};
use uuid::Uuid;

fn expected_postgres_target_key(table: &str) -> String {
    format!(
        "postgres_write|table:len={}:hex={}",
        table.as_bytes().len(),
        hex::encode(table.as_bytes())
    )
}

async fn postgres_pool() -> sqlx::PgPool {
    let url = handshake_core::storage::tests::postgres_test_base_url()
        .await
        .expect("resolve real PostgreSQL for restart_resume_postgres_tests");
    let mut conn = sqlx::PgConnection::connect(&url)
        .await
        .expect("connect postgres");
    let schema = format!("mt193_{}", Uuid::now_v7().simple());
    sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
        .execute(&mut conn)
        .await
        .expect("create isolated schema");
    drop(conn);

    let sep = if url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");
    let pool = sqlx::PgPool::connect(&schema_url)
        .await
        .expect("connect isolated schema");
    apply_schema(&pool).await;
    pool
}

async fn apply_schema(pool: &sqlx::PgPool) {
    for stmt in [
        include_str!("../migrations/0018_kernel_event_ledger.sql"),
        include_str!("../migrations/0019_kernel_session_queue.sql"),
        include_str!("../migrations/0021_kernel_process_lifecycle.sql"),
        include_str!("../migrations/0022_role_mailbox_threads_messages.sql"),
        include_str!("../migrations/0024_session_checkpoint.sql"),
        include_str!("../migrations/0028_restart_resume_report_wiring.sql"),
    ] {
        sqlx::raw_sql(stmt)
            .execute(pool)
            .await
            .expect("apply MT-193 schema");
    }
}

async fn seed_session_queue(pool: &sqlx::PgPool, session_id: Uuid, state: &str) -> String {
    let session_run_id = format!("SR-{session_id}");
    sqlx::query(
        r#"
        INSERT INTO kernel_session_queue (
            session_run_id,
            kernel_task_run_id,
            adapter_id,
            state,
            claimed_by,
            lease_expires_at,
            attempt_count,
            available_at,
            created_at,
            updated_at
        )
        VALUES ($1, $2, 'mt193-test-adapter', $3, 'previous-worker', NOW() + INTERVAL '30 minutes', 1, NOW(), NOW(), NOW())
        "#,
    )
    .bind(&session_run_id)
    .bind(format!("KTR-{session_id}"))
    .bind(state)
    .execute(pool)
    .await
    .expect("seed session queue");
    session_run_id
}

async fn seed_checkpoint(
    pool: &sqlx::PgPool,
    session_id: Uuid,
    last_seq: i64,
    compact_state: Value,
) {
    sqlx::query(
        r#"
        INSERT INTO kernel_session_checkpoint (
            checkpoint_id,
            session_id,
            model_session_id,
            last_event_ledger_seq,
            compact_state,
            state_kind,
            pending_artifacts,
            created_at_utc,
            created_by_process,
            schema_version
        )
        VALUES ($1, $2, $3, $4, $5, 'periodic', '[]'::jsonb, NOW(), 1234, 1)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(session_id)
    .bind(Uuid::now_v7())
    .bind(last_seq)
    .bind(compact_state)
    .execute(pool)
    .await
    .expect("seed checkpoint");
}

async fn seed_event(
    pool: &sqlx::PgPool,
    session_run_id: &str,
    explicit_seq: Option<i64>,
    payload: Value,
) -> i64 {
    let event_id = format!("KE-{}", Uuid::now_v7());
    let idempotency_key = format!("mt193-event-{event_id}");
    let mut query = if explicit_seq.is_some() {
        sqlx::query(
            r#"
            INSERT INTO kernel_event_ledger (
                event_id,
                event_sequence,
                event_version,
                kernel_task_run_id,
                session_run_id,
                aggregate_type,
                aggregate_id,
                idempotency_key,
                event_type,
                actor_kind,
                actor_id,
                payload_hash,
                source_component,
                payload
            )
            VALUES ($1, $2, 'kernel_event_v1', $3, $4, 'session_run', $4, $5,
                    'MODEL_RESPONSE_RECORDED', 'session_broker', 'mt193-test',
                    '0000000000000000000000000000000000000000000000000000000000000000',
                    'mt193-test', $6)
            RETURNING event_sequence
            "#,
        )
        .bind(&event_id)
        .bind(explicit_seq.expect("explicit sequence"))
    } else {
        sqlx::query(
            r#"
            INSERT INTO kernel_event_ledger (
                event_id,
                event_version,
                kernel_task_run_id,
                session_run_id,
                aggregate_type,
                aggregate_id,
                idempotency_key,
                event_type,
                actor_kind,
                actor_id,
                payload_hash,
                source_component,
                payload
            )
            VALUES ($1, 'kernel_event_v1', $2, $3, 'session_run', $3, $4,
                    'MODEL_RESPONSE_RECORDED', 'session_broker', 'mt193-test',
                    '0000000000000000000000000000000000000000000000000000000000000000',
                    'mt193-test', $5)
            RETURNING event_sequence
            "#,
        )
        .bind(&event_id)
    };
    query = query
        .bind(format!("KTR-{session_run_id}"))
        .bind(session_run_id)
        .bind(idempotency_key)
        .bind(payload);
    query
        .fetch_one(pool)
        .await
        .expect("seed event")
        .get("event_sequence")
}

async fn seed_unrelated_event(pool: &sqlx::PgPool) -> i64 {
    let other_session_id = Uuid::now_v7();
    let other_session_run_id = format!("SR-{other_session_id}");
    seed_event(
        pool,
        &other_session_run_id,
        None,
        json!({ "by": 999, "unrelated": true }),
    )
    .await
}

async fn seed_orphan_process(pool: &sqlx::PgPool, session_run_id: &str) -> Uuid {
    let process_uuid = Uuid::now_v7();
    sqlx::query(
        r#"
        INSERT INTO kernel_process_lifecycle (
            process_uuid,
            os_pid,
            parent_session_id,
            sandbox_adapter_id,
            engine_kind,
            started_at,
            owner_role,
            owner_wp,
            metadata_jsonb
        )
        VALUES ($1, 4242, $2, 'mt193-test-adapter', 'helper_subprocess', NOW(), 'coder', 'WP-KERNEL-004', '{}'::jsonb)
        "#,
    )
    .bind(process_uuid)
    .bind(session_run_id)
    .execute(pool)
    .await
    .expect("seed orphan process");
    process_uuid
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt193_postgres_startup_runner_resumes_persists_report_and_is_idempotent() {
    let pool = postgres_pool().await;
    let session_id = Uuid::now_v7();
    let session_run_id = seed_session_queue(&pool, session_id, "RUNNING").await;
    seed_checkpoint(&pool, session_id, 0, json!({ "counter": 10 })).await;
    seed_event(&pool, &session_run_id, None, json!({ "by": 5 })).await;
    seed_event(&pool, &session_run_id, None, json!({ "by": 7 })).await;
    let process_uuid = seed_orphan_process(&pool, &session_run_id).await;

    let runner = PostgresRestartResumeRunner::new(pool.clone());
    let report = runner.run().await.expect("restart resume run");

    assert_eq!(report.sessions_examined, 1);
    assert_eq!(report.sessions_resumed.len(), 1);
    assert_eq!(report.sessions_recovery_failed.len(), 0);
    assert_eq!(report.sessions_resumed[0].session_id, session_id);
    assert_eq!(report.sessions_resumed[0].events_applied, 2);
    assert_eq!(report.sessions_resumed[0].final_seq, 2);
    assert_eq!(report.orphan_reclaims[0].processes_reclaimed, 1);
    assert_eq!(
        report.fr_events_emitted,
        vec![
            "FR-EVT-RESTART-RESUME-STARTED",
            "FR-EVT-RESTART-RESUME-SESSION-RESUMED",
            "FR-EVT-RESTART-RESUME-COMPLETED"
        ]
    );

    let report_row = sqlx::query(
        r#"
        SELECT sessions_examined, sessions_resumed, orphan_reclaims, fr_events_emitted, schema_version
        FROM kernel_restart_resume_report
        WHERE report_id = $1
        "#,
    )
    .bind(report.report_id)
    .fetch_one(&pool)
    .await
    .expect("persisted report row");
    assert_eq!(report_row.get::<i32, _>("sessions_examined"), 1);
    assert_eq!(report_row.get::<i32, _>("schema_version"), 2);
    assert_eq!(
        report_row
            .get::<Value, _>("fr_events_emitted")
            .as_array()
            .expect("fr events json")
            .len(),
        3
    );
    assert_eq!(
        report_row
            .get::<Value, _>("sessions_resumed")
            .as_array()
            .expect("resumed json")
            .len(),
        1
    );
    assert_eq!(
        report_row
            .get::<Value, _>("orphan_reclaims")
            .as_array()
            .expect("orphan json")
            .len(),
        1
    );

    let queue_row = sqlx::query(
        "SELECT state, claimed_by, lease_expires_at FROM kernel_session_queue WHERE session_run_id = $1",
    )
    .bind(&session_run_id)
    .fetch_one(&pool)
    .await
    .expect("queue row");
    assert_eq!(queue_row.get::<String, _>("state"), "RETRY_SCHEDULED");
    assert!(queue_row.get::<Option<String>, _>("claimed_by").is_none());
    assert!(queue_row
        .get::<Option<chrono::NaiveDateTime>, _>("lease_expires_at")
        .is_none());

    let stopped_at: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
        "SELECT stopped_at FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(process_uuid)
    .fetch_one(&pool)
    .await
    .expect("reclaimed process row");
    assert!(stopped_at.is_some());

    let latest_checkpoint = sqlx::query(
        r#"
        SELECT compact_state, last_event_ledger_seq, state_kind
        FROM kernel_session_checkpoint
        WHERE session_id = $1
        ORDER BY created_at_utc DESC
        LIMIT 1
        "#,
    )
    .bind(session_id)
    .fetch_one(&pool)
    .await
    .expect("latest checkpoint");
    assert_eq!(
        latest_checkpoint
            .get::<Value, _>("compact_state")
            .get("counter")
            .and_then(Value::as_i64),
        Some(22)
    );
    assert_eq!(latest_checkpoint.get::<i64, _>("last_event_ledger_seq"), 2);
    assert_eq!(
        latest_checkpoint.get::<String, _>("state_kind"),
        "post_failure"
    );

    let idempotency_keys: Vec<String> = sqlx::query_scalar(
        "SELECT side_effect_kind FROM kernel_idempotency_ledger WHERE session_id = $1 ORDER BY side_effect_kind",
    )
    .bind(session_id)
    .fetch_all(&pool)
    .await
    .expect("idempotency target keys");
    assert!(idempotency_keys.contains(&expected_postgres_target_key("kernel_process_lifecycle")));
    assert!(idempotency_keys.contains(&expected_postgres_target_key("kernel_session_checkpoint")));
    assert!(idempotency_keys.contains(&expected_postgres_target_key("kernel_session_queue")));

    let second = runner.run().await.expect("restart resume second run");
    assert_eq!(second.sessions_examined, 0);
    assert_eq!(second.sessions_resumed.len(), 0);
    assert_eq!(second.sessions_recovery_failed.len(), 0);
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt193_postgres_startup_runner_replays_session_events_when_global_event_sequence_interleaves_other_sessions(
) {
    let pool = postgres_pool().await;
    let session_id = Uuid::now_v7();
    let session_run_id = seed_session_queue(&pool, session_id, "RUNNING").await;
    seed_checkpoint(&pool, session_id, 0, json!({ "counter": 10 })).await;
    let seq_1 = seed_event(&pool, &session_run_id, None, json!({ "by": 5 })).await;
    let unrelated_seq = seed_unrelated_event(&pool).await;
    let seq_2 = seed_event(&pool, &session_run_id, None, json!({ "by": 7 })).await;
    assert_eq!(seq_1 + 1, unrelated_seq);
    assert_eq!(unrelated_seq + 1, seq_2);

    let runner = PostgresRestartResumeRunner::new(pool.clone());
    let report = runner.run().await.expect("restart resume run");

    assert_eq!(report.sessions_examined, 1);
    assert_eq!(report.sessions_resumed.len(), 1);
    assert_eq!(report.sessions_recovery_failed.len(), 0);
    assert_eq!(report.sessions_resumed[0].events_applied, 2);
    assert_eq!(report.sessions_resumed[0].final_seq, seq_2);

    let latest_checkpoint = sqlx::query(
        r#"
        SELECT compact_state, last_event_ledger_seq
        FROM kernel_session_checkpoint
        WHERE session_id = $1
        ORDER BY created_at_utc DESC
        LIMIT 1
        "#,
    )
    .bind(session_id)
    .fetch_one(&pool)
    .await
    .expect("latest checkpoint");
    assert_eq!(
        latest_checkpoint
            .get::<Value, _>("compact_state")
            .get("counter")
            .and_then(Value::as_i64),
        Some(22)
    );
    assert_eq!(
        latest_checkpoint.get::<i64, _>("last_event_ledger_seq"),
        seq_2
    );
}

#[tokio::test]
#[ignore = "requires real PostgreSQL; auto-resolves POSTGRES_TEST_URL > DATABASE_URL > managed PostgreSQL; run with `cargo test -- --ignored`"]
async fn mt193_postgres_startup_runner_marks_recovery_failed_and_posts_operator_decision() {
    let pool = postgres_pool().await;
    let session_id = Uuid::now_v7();
    let session_run_id = seed_session_queue(&pool, session_id, "RUNNING").await;
    seed_checkpoint(&pool, session_id, 0, json!({ "counter": 1 })).await;
    seed_event(&pool, &session_run_id, Some(1), json!({ "by": 1 })).await;
    seed_event(&pool, &session_run_id, Some(3), json!({ "by": 1 })).await;

    let runner = PostgresRestartResumeRunner::new(pool.clone());
    let report = runner.run().await.expect("restart resume run");

    assert_eq!(report.sessions_examined, 1);
    assert_eq!(report.sessions_resumed.len(), 0);
    assert_eq!(report.sessions_recovery_failed.len(), 1);
    assert_eq!(report.operator_decision_requests.len(), 1);

    let state: String =
        sqlx::query_scalar("SELECT state FROM kernel_session_queue WHERE session_run_id = $1")
            .bind(&session_run_id)
            .fetch_one(&pool)
            .await
            .expect("queue state");
    assert_eq!(state, "FAILED");

    let mailbox = sqlx::query(
        r#"
        SELECT t.linked_record_id, m.message_type, m.from_role, m.to_roles, m.body
        FROM role_mailbox_thread t
        JOIN role_mailbox_message m ON m.thread_id = t.thread_id
        WHERE t.linked_record_id = $1
        "#,
    )
    .bind(session_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("mailbox decision request");

    assert_eq!(
        mailbox.get::<String, _>("linked_record_id"),
        session_id.to_string()
    );
    assert_eq!(mailbox.get::<String, _>("message_type"), "decision_request");
    assert_eq!(mailbox.get::<String, _>("from_role"), "orchestrator");
    assert!(mailbox
        .get::<Value, _>("to_roles")
        .as_array()
        .expect("to_roles")
        .iter()
        .any(|role| role.as_str() == Some("operator")));
    let body = mailbox.get::<Value, _>("body");
    assert_eq!(
        body.get("family").and_then(Value::as_str),
        Some("decision_request")
    );
    assert_eq!(
        body.pointer("/body/decision_authority_role")
            .and_then(Value::as_str),
        Some("operator")
    );
    assert!(body
        .get("resume_error")
        .expect("resume error")
        .to_string()
        .contains("missing_event"));

    let idempotency_keys: Vec<String> = sqlx::query_scalar(
        "SELECT side_effect_kind FROM kernel_idempotency_ledger WHERE session_id = $1 ORDER BY side_effect_kind",
    )
    .bind(session_id)
    .fetch_all(&pool)
    .await
    .expect("idempotency target keys");
    assert!(idempotency_keys.contains(&expected_postgres_target_key("kernel_session_queue")));
    assert!(idempotency_keys.contains(&expected_postgres_target_key("role_mailbox_message")));
}
