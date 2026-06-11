use std::{
    error::Error,
    net::TcpListener,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use chrono::{TimeZone, Utc};
use handshake_core::managed_postgres::{ManagedPostgres, ManagedPostgresConfig};
use handshake_core::observability::aggregate_queries::{
    ActivityRow, AggregateQueryFixture, Limit, Offset, SessionAggregateQueries, SessionSummary,
    SessionTimelineEntry,
};
use serde_json::json;
use sqlx::Connection;
use uuid::Uuid;

const POSTGRES_READY_TIMEOUT: Duration = Duration::from_secs(45);

struct FixtureIds {
    model_session_a: Uuid,
    session_a: Uuid,
    session_b: Uuid,
}

struct PostgresFixtureIds {
    model_session_id: Uuid,
    session_id: Uuid,
}

fn uid(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn base_time() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 24, 6, 0, 0).single().unwrap()
}

fn fixture() -> (SessionAggregateQueries, FixtureIds) {
    let base = base_time();
    let model_session_a = uid(1);
    let model_session_b = uid(2);
    let session_a = uid(101);
    let session_b = uid(102);
    let session_c = uid(103);
    let mut activities = Vec::new();
    for idx in 0..12 {
        activities.push(ActivityRow {
            span_id: uid(1_000 + idx),
            parent_span_id: None,
            model_session_id: model_session_a,
            session_id: session_a,
            activity_kind: "mt_iteration".to_string(),
            started_at_utc: base + chrono::Duration::milliseconds(idx as i64),
            ended_at_utc: Some(
                base + chrono::Duration::milliseconds(idx as i64 + (idx as i64 * 10) + 1),
            ),
            status: "completed".to_string(),
        });
    }
    activities.push(ActivityRow {
        span_id: uid(2_000),
        parent_span_id: None,
        model_session_id: model_session_b,
        session_id: session_b,
        activity_kind: "checkpoint_write".to_string(),
        started_at_utc: base + chrono::Duration::seconds(3),
        ended_at_utc: Some(base + chrono::Duration::seconds(4)),
        status: "completed".to_string(),
    });

    let fixture = AggregateQueryFixture {
        sessions: vec![
            SessionSummary {
                session_id: session_a,
                model_session_id: model_session_a,
                wp_id: Some("WP-KERNEL-004".to_string()),
                started_at_utc: base,
                ended_at_utc: None,
            },
            SessionSummary {
                session_id: session_b,
                model_session_id: model_session_b,
                wp_id: Some("WP-KERNEL-004".to_string()),
                started_at_utc: base + chrono::Duration::seconds(1),
                ended_at_utc: Some(base + chrono::Duration::seconds(5)),
            },
            SessionSummary {
                session_id: session_c,
                model_session_id: uid(3),
                wp_id: Some("WP-OTHER".to_string()),
                started_at_utc: base + chrono::Duration::seconds(2),
                ended_at_utc: None,
            },
        ],
        activities,
        timeline_entries: vec![
            (
                session_a,
                SessionTimelineEntry {
                    kind: "mailbox_message".to_string(),
                    at_utc: base + chrono::Duration::seconds(3),
                    summary: "validator handoff".to_string(),
                },
            ),
            (
                session_a,
                SessionTimelineEntry {
                    kind: "event".to_string(),
                    at_utc: base + chrono::Duration::seconds(1),
                    summary: "session claimed".to_string(),
                },
            ),
            (
                session_a,
                SessionTimelineEntry {
                    kind: "checkpoint".to_string(),
                    at_utc: base + chrono::Duration::seconds(2),
                    summary: "checkpoint written".to_string(),
                },
            ),
            (
                session_b,
                SessionTimelineEntry {
                    kind: "span".to_string(),
                    at_utc: base + chrono::Duration::seconds(2),
                    summary: "activity span".to_string(),
                },
            ),
        ],
        active_leases: 2,
        in_flight_micro_tasks: 4,
        pending_mailbox_messages: 7,
    };

    (
        SessionAggregateQueries::from_fixture(fixture),
        FixtureIds {
            model_session_a,
            session_a,
            session_b,
        },
    )
}

async fn postgres_pool(url: &str) -> sqlx::PgPool {
    let mut conn = sqlx::PgConnection::connect(&url)
        .await
        .expect("postgres connect");
    let schema = format!("mt200_{}", Uuid::now_v7().simple());
    sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
        .execute(&mut conn)
        .await
        .expect("create isolated schema");
    drop(conn);

    let sep = if url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");
    sqlx::PgPool::connect(&schema_url)
        .await
        .expect("postgres isolated schema connect")
}

async fn apply_mt200_schema(pool: &sqlx::PgPool) {
    for stmt in [
        include_str!("../migrations/0018_kernel_event_ledger.sql"),
        include_str!("../migrations/0022_role_mailbox_threads_messages.sql"),
        include_str!("../migrations/0023_micro_task_job_queue.sql"),
        include_str!("../migrations/0024_session_checkpoint.sql"),
        include_str!("../migrations/0025_observability_spans.sql"),
    ] {
        sqlx::raw_sql(stmt)
            .execute(pool)
            .await
            .expect("migrate MT-200");
    }
}

async fn seed_mt200_postgres_rows(
    pool: &sqlx::PgPool,
    base: chrono::DateTime<Utc>,
) -> PostgresFixtureIds {
    let model_session_id = Uuid::now_v7();
    let session_id = Uuid::now_v7();
    let session_span_id = Uuid::now_v7();
    let checkpoint_activity_span_id = Uuid::now_v7();
    let mt_activity_span_id = Uuid::now_v7();
    let checkpoint_id = Uuid::now_v7();
    let thread_id = Uuid::now_v7();
    let message_id = Uuid::now_v7();
    let lease_id = Uuid::now_v7();
    let job_id = Uuid::now_v7();
    let outcome_id = Uuid::now_v7();
    let wp_id = format!("WP-KERNEL-004-MT200-{session_id}");

    sqlx::query(
        r#"INSERT INTO kernel_model_session_span
           (span_id, model_session_id, session_id, started_at_utc, ended_at_utc, status, attributes)
           VALUES ($1, $2, $3, $4, $5, 'active', '{"test":"mt200"}'::jsonb)"#,
    )
    .bind(session_span_id)
    .bind(model_session_id)
    .bind(session_id)
    .bind(base)
    .bind(base + chrono::Duration::seconds(20))
    .execute(pool)
    .await
    .expect("insert model session span");

    sqlx::query(
        r#"INSERT INTO kernel_activity_span
           (span_id, parent_span_id, activity_kind, started_at_utc, ended_at_utc, status, attributes, related_event_ledger_seqs)
           VALUES
           ($1, $2, 'checkpoint_write', $3, $4, 'completed', '{}'::jsonb, '[1]'::jsonb),
           ($5, $2, 'mt_iteration', $6, $7, 'completed', '{}'::jsonb, '[1]'::jsonb)"#,
    )
    .bind(checkpoint_activity_span_id)
    .bind(session_span_id)
    .bind(base + chrono::Duration::seconds(1))
    .bind(base + chrono::Duration::seconds(2))
    .bind(mt_activity_span_id)
    .bind(base + chrono::Duration::seconds(3))
    .bind(base + chrono::Duration::seconds(8))
    .execute(pool)
    .await
    .expect("insert activity spans");

    sqlx::query(
        r#"INSERT INTO kernel_event_ledger
           (event_id, event_sequence, event_version, kernel_task_run_id, session_run_id,
            aggregate_type, aggregate_id, idempotency_key, event_type, actor_kind, actor_id,
            payload_hash, source_component, payload, created_at)
           VALUES ($1, 1, 'v1', 'kernel-task-mt200', $2, 'session', $2, $3,
                   'SessionStarted', 'test', 'mt200', 'hash-mt200', 'mt200-test',
                   $4, $5::timestamptz AT TIME ZONE 'UTC')"#,
    )
    .bind(format!("mt200-event-{session_id}"))
    .bind(session_id.to_string())
    .bind(format!("mt200-idempotency-{session_id}"))
    .bind(json!({ "session_id": session_id }))
    .bind(base)
    .execute(pool)
    .await
    .expect("insert event ledger row");

    sqlx::query(
        r#"INSERT INTO kernel_session_checkpoint
           (checkpoint_id, session_id, model_session_id, last_event_ledger_seq, compact_state,
            state_kind, pending_artifacts, created_at_utc, created_by_process)
           VALUES ($1, $2, $3, 1, $4, 'running', '[]'::jsonb, $5, 1)"#,
    )
    .bind(checkpoint_id)
    .bind(session_id)
    .bind(model_session_id)
    .bind(json!({ "phase": "mt200" }))
    .bind(base + chrono::Duration::seconds(2))
    .execute(pool)
    .await
    .expect("insert checkpoint row");

    sqlx::query(
        r#"INSERT INTO role_mailbox_thread
           (thread_id, title, linked_record_kind, linked_record_id, lifecycle_state,
            executor_kind_allowlist, claim_mode, lease_duration_secs, takeover_policy,
            response_authority_scope, created_at_utc, updated_at_utc)
           VALUES ($1, 'MT-200 thread', 'session', $2, 'open', '[]'::jsonb,
                   'exclusive', 600, 'manual', 'thread', $3, $3)"#,
    )
    .bind(thread_id)
    .bind(session_id.to_string())
    .bind(base)
    .execute(pool)
    .await
    .expect("insert mailbox thread");

    sqlx::query(
        r#"INSERT INTO role_mailbox_message
           (message_id, thread_id, message_type, from_role, to_roles, delivery_state, body, created_at_utc)
           VALUES ($1, $2, 'DecisionRequest', 'kernel_builder', '["validator"]'::jsonb,
                   'pending', $3, $4)"#,
    )
    .bind(message_id)
    .bind(thread_id)
    .bind(json!({ "summary": "validator handoff" }))
    .bind(base + chrono::Duration::seconds(4))
    .execute(pool)
    .await
    .expect("insert mailbox message");

    sqlx::query(
        r#"INSERT INTO role_mailbox_claim_lease
           (lease_id, thread_id, holder_executor_kind, holder_role_id, holder_session_id,
            acquired_at_utc, expires_at_utc)
           VALUES ($1, $2, 'model', 'kernel_builder', $3, $4, $5)"#,
    )
    .bind(lease_id)
    .bind(thread_id)
    .bind(session_id)
    .bind(base)
    .bind(base + chrono::Duration::minutes(10))
    .execute(pool)
    .await
    .expect("insert mailbox lease");

    sqlx::query(
        r#"INSERT INTO kernel_micro_task_job
           (job_id, wp_id, mt_id, mt_contract_path, max_iterations, escalation_tier,
            mailbox_thread_id, state, claimed_by_session, claimed_at_utc, transition_reason)
           VALUES ($1, $2, 'MT-200', '.GOV/task_packets/MT-200.json', 3, 'normal',
                   $3, 'running', $4, $5, 'mt200 integration seed')"#,
    )
    .bind(job_id)
    .bind(&wp_id)
    .bind(thread_id)
    .bind(session_id)
    .bind(base + chrono::Duration::seconds(1))
    .execute(pool)
    .await
    .expect("insert micro task job");

    sqlx::query(
        r#"INSERT INTO kernel_mt_outcome
           (outcome_id, job_id, iteration_n, outcome_kind, recorded_at_utc, recorded_by_session)
           VALUES ($1, $2, 1, 'needs_validation', $3, $4)"#,
    )
    .bind(outcome_id)
    .bind(job_id)
    .bind(base + chrono::Duration::seconds(5))
    .bind(session_id)
    .execute(pool)
    .await
    .expect("insert MT outcome");

    PostgresFixtureIds {
        model_session_id,
        session_id,
    }
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test --test observability_aggregate_queries_tests -- --ignored`"]
async fn mt200_postgres_queries_join_spans_mailbox_checkpoints_and_events() {
    let fixture = PostgresFixture::start().await.expect("postgres fixture");
    let pool = postgres_pool(fixture.url()).await;
    apply_mt200_schema(&pool).await;
    let base = base_time();
    let ids = seed_mt200_postgres_rows(&pool, base).await;
    let queries = SessionAggregateQueries::new(pool);

    let activity = queries
        .activity_for_model_session(
            ids.model_session_id,
            base,
            base + chrono::Duration::seconds(10),
            Limit::default(),
        )
        .await
        .unwrap();
    assert_eq!(activity.len(), 2);
    assert!(activity
        .iter()
        .all(|row| row.model_session_id == ids.model_session_id));
    assert_eq!(activity[0].activity_kind, "checkpoint_write");
    assert_eq!(activity[1].activity_kind, "mt_iteration");
    let activity_page = queries
        .activity_for_model_session_page(
            ids.model_session_id,
            base,
            base + chrono::Duration::seconds(10),
            Offset::new(1),
            Limit::new(1),
        )
        .await
        .unwrap();
    assert_eq!(activity_page.len(), 1);
    assert_eq!(activity_page[0].activity_kind, "mt_iteration");

    let wp_id = format!("WP-KERNEL-004-MT200-{}", ids.session_id);
    let sessions = queries
        .sessions_touching_wp(
            &wp_id,
            base,
            base + chrono::Duration::seconds(10),
            Limit::default(),
        )
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, ids.session_id);
    assert_eq!(sessions[0].wp_id.as_deref(), Some(wp_id.as_str()));

    let slowest = queries
        .slowest_spans_by_activity_kind("mt_iteration", Limit::new(1))
        .await
        .unwrap();
    assert_eq!(slowest.len(), 1);
    assert_eq!(slowest[0].session_id, ids.session_id);
    assert_eq!(slowest[0].duration_ms, 5_000);

    let timeline = queries
        .session_timeline(
            ids.session_id,
            base,
            base + chrono::Duration::seconds(10),
            Limit::default(),
        )
        .await
        .unwrap();
    let kinds: Vec<_> = timeline
        .entries
        .iter()
        .map(|entry| entry.kind.as_str())
        .collect();
    assert_eq!(
        kinds,
        vec![
            "event",
            "span",
            "checkpoint",
            "span",
            "mailbox_message",
            "mt_outcome"
        ]
    );
    for window in timeline.entries.windows(2) {
        assert!(window[0].at_utc <= window[1].at_utc);
    }

    let snapshot = queries
        .swarm_concurrency_snapshot(base + chrono::Duration::seconds(4))
        .await
        .unwrap();
    assert!(snapshot.active_sessions >= 1);
    assert!(snapshot.active_leases >= 1);
    assert!(snapshot.in_flight_micro_tasks >= 1);
    assert!(snapshot.pending_mailbox_messages >= 1);
}

struct PostgresFixture {
    url: String,
    managed_data_dir: Option<PathBuf>,
}

impl PostgresFixture {
    async fn start() -> Result<Self, Box<dyn Error>> {
        if let Ok(url) = std::env::var("POSTGRES_TEST_URL") {
            if !url.trim().is_empty() {
                return Ok(Self {
                    url,
                    managed_data_dir: None,
                });
            }
        }

        let data_dir =
            std::env::temp_dir().join(format!("hsk-managed-pg-mt200-{}", Uuid::now_v7().simple()));
        let port = free_local_port()?;
        let managed = ManagedPostgres::ensure_running(ManagedPostgresConfig {
            enabled: true,
            data_dir: data_dir.clone(),
            port,
            bin_dir: PathBuf::new(),
            database: "handshake_test".to_string(),
            superuser: "postgres".to_string(),
            startup_timeout: POSTGRES_READY_TIMEOUT,
        })
        .await?;
        let url = managed.database_url();

        Ok(Self {
            url,
            managed_data_dir: Some(data_dir),
        })
    }

    fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for PostgresFixture {
    fn drop(&mut self) {
        if let Some(data_dir) = &self.managed_data_dir {
            let _ = Command::new(pg_ctl_path())
                .args(["stop", "-D"])
                .arg(data_dir)
                .args(["-m", "fast"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let _ = std::fs::remove_dir_all(data_dir);
        }
    }
}

fn free_local_port() -> Result<u16, Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

fn pg_ctl_path() -> PathBuf {
    std::env::var("HANDSHAKE_MANAGED_PG_BIN")
        .ok()
        .or_else(|| std::env::var("PGBIN").ok())
        .filter(|value| !value.trim().is_empty())
        .map(|dir| {
            let exe = if cfg!(windows) {
                "pg_ctl.exe"
            } else {
                "pg_ctl"
            };
            PathBuf::from(dir).join(exe)
        })
        .unwrap_or_else(|| PathBuf::from("pg_ctl"))
}

#[tokio::test]
async fn mt200_queries_return_typed_shapes() {
    let (queries, ids) = fixture();
    let base = base_time();

    let activity = queries
        .activity_for_model_session(
            ids.model_session_a,
            base,
            base + chrono::Duration::seconds(10),
            Limit::default(),
        )
        .await
        .unwrap();
    assert_eq!(activity.len(), 12);
    assert!(activity
        .iter()
        .all(|row| row.model_session_id == ids.model_session_a));

    let sessions = queries
        .sessions_touching_wp(
            "WP-KERNEL-004",
            base,
            base + chrono::Duration::seconds(10),
            Limit::default(),
        )
        .await
        .unwrap();
    assert_eq!(sessions.len(), 2);
    assert!(sessions
        .iter()
        .all(|row| row.wp_id.as_deref() == Some("WP-KERNEL-004")));

    let slowest = queries
        .slowest_spans_by_activity_kind("mt_iteration", Limit::new(3))
        .await
        .unwrap();
    assert_eq!(slowest.len(), 3);
    assert!(slowest[0].duration_ms >= slowest[1].duration_ms);

    let snapshot = queries
        .swarm_concurrency_snapshot(base + chrono::Duration::seconds(2))
        .await
        .unwrap();
    assert_eq!(snapshot.active_sessions, 3);
    assert_eq!(snapshot.active_leases, 2);
    assert_eq!(snapshot.in_flight_micro_tasks, 4);
    assert_eq!(snapshot.pending_mailbox_messages, 7);
}

#[tokio::test]
async fn mt200_pagination_limit_caps_rows() {
    let (queries, ids) = fixture();
    let base = base_time();

    let rows = queries
        .activity_for_model_session(
            ids.model_session_a,
            base,
            base + chrono::Duration::seconds(10),
            Limit::new(10),
        )
        .await
        .unwrap();

    assert_eq!(rows.len(), 10);
    assert_eq!(Limit::new(10_000).as_usize(), 1000);
}

#[tokio::test]
async fn mt200_offset_pagination_returns_stable_windows() {
    let (queries, ids) = fixture();
    let base = base_time();

    let activity = queries
        .activity_for_model_session_page(
            ids.model_session_a,
            base,
            base + chrono::Duration::seconds(10),
            Offset::new(5),
            Limit::new(3),
        )
        .await
        .unwrap();
    let span_ids: Vec<_> = activity.iter().map(|row| row.span_id).collect();
    assert_eq!(span_ids, vec![uid(1_005), uid(1_006), uid(1_007)]);

    let sessions = queries
        .sessions_touching_wp_page(
            "WP-KERNEL-004",
            base,
            base + chrono::Duration::seconds(10),
            Offset::new(1),
            Limit::new(1),
        )
        .await
        .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, ids.session_b);

    let slowest = queries
        .slowest_spans_by_activity_kind_page("mt_iteration", Offset::new(1), Limit::new(2))
        .await
        .unwrap();
    assert_eq!(slowest.len(), 2);
    assert!(slowest[0].duration_ms >= slowest[1].duration_ms);
    assert!(slowest[0].duration_ms < 111);

    let timeline = queries
        .session_timeline_page(
            ids.session_a,
            base,
            base + chrono::Duration::seconds(10),
            Offset::new(1),
            Limit::new(1),
        )
        .await
        .unwrap();
    assert_eq!(timeline.entries.len(), 1);
    assert_eq!(timeline.entries[0].kind, "checkpoint");
}

#[tokio::test]
async fn mt200_session_timeline_is_strictly_chronological_across_entity_types() {
    let (queries, ids) = fixture();
    let base = base_time();

    let timeline = queries
        .session_timeline(
            ids.session_a,
            base,
            base + chrono::Duration::seconds(10),
            Limit::default(),
        )
        .await
        .unwrap();

    let kinds: Vec<_> = timeline
        .entries
        .iter()
        .map(|entry| entry.kind.as_str())
        .collect();
    assert_eq!(kinds, vec!["event", "checkpoint", "mailbox_message"]);
    for window in timeline.entries.windows(2) {
        assert!(window[0].at_utc <= window[1].at_utc);
    }
}

#[tokio::test]
async fn mt200_fixture_queries_return_under_latency_budget() {
    let base = base_time();
    let mut sessions = Vec::new();
    let mut activities = Vec::new();
    for session_idx in 0..100u128 {
        let session_id = uid(10_000 + session_idx);
        let model_session_id = uid(20_000 + session_idx);
        sessions.push(SessionSummary {
            session_id,
            model_session_id,
            wp_id: Some("WP-KERNEL-004".to_string()),
            started_at_utc: base,
            ended_at_utc: None,
        });
        for event_idx in 0..100u128 {
            activities.push(ActivityRow {
                span_id: uid(30_000 + session_idx * 100 + event_idx),
                parent_span_id: None,
                model_session_id,
                session_id,
                activity_kind: "mt_iteration".to_string(),
                started_at_utc: base + chrono::Duration::milliseconds(event_idx as i64),
                ended_at_utc: Some(base + chrono::Duration::milliseconds(event_idx as i64 + 2)),
                status: "completed".to_string(),
            });
        }
    }
    let queries = SessionAggregateQueries::from_fixture(AggregateQueryFixture {
        sessions,
        activities,
        timeline_entries: Vec::new(),
        active_leases: 100,
        in_flight_micro_tasks: 100,
        pending_mailbox_messages: 0,
    });

    let started = Instant::now();
    let sessions = queries
        .sessions_touching_wp(
            "WP-KERNEL-004",
            base,
            base + chrono::Duration::seconds(1),
            Limit::new(1000),
        )
        .await
        .unwrap();
    let slowest = queries
        .slowest_spans_by_activity_kind("mt_iteration", Limit::new(10))
        .await
        .unwrap();
    let elapsed = started.elapsed();

    assert_eq!(sessions.len(), 100);
    assert_eq!(slowest.len(), 10);
    assert!(
        elapsed < Duration::from_secs(5),
        "aggregate fixture queries took {elapsed:?}"
    );
}
