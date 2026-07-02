use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use chrono::{DateTime, Utc};
use handshake_core::{
    flight_recorder::fr_event_registry::FrEventId,
    process_ledger::restart_resume::{PostgresRestartResumeRunner, RestartResumeDbBackoffPolicy},
    session_checkpoint::{
        CheckpointStateKind, EventLedgerRow, ReplayError, ResumableSession, ResumeError,
        ResumeReport, SessionCheckpoint, SessionCheckpointId,
    },
    storage::{postgres::PostgresDatabase, Database},
};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{
    postgres::{PgPoolOptions, PgRow},
    Connection, PgConnection, PgPool, Row,
};
use uuid::Uuid;

const RUNTIME_TABLES: &[&str] = &[
    "kernel_session_checkpoint",
    "kernel_event_ledger",
    "kernel_restart_resume_report",
    "kernel_operator_decision_request",
    "kernel_runtime_process_evidence",
    "kernel_runtime_fr_event",
];

const PRODUCTION_RESTART_RESUME_TABLES: &[&str] = &[
    "kernel_session_queue",
    "kernel_process_lifecycle",
    "kernel_session_checkpoint",
    "kernel_event_ledger",
    "kernel_restart_resume_report",
    "kernel_idempotency_ledger",
];

#[derive(Debug, thiserror::Error)]
pub enum RuntimeTestError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Debug)]
pub struct RuntimePg {
    pub pool: PgPool,
    control_pool: PgPool,
    schema: String,
    schema_url: String,
    artifact_dir: PathBuf,
}

impl RuntimePg {
    pub async fn new(label: &str) -> Self {
        let base_url = postgres_url();
        let schema = format!("mt195_runtime_{}", Uuid::now_v7().simple());
        let mut admin = PgConnection::connect(&base_url)
            .await
            .unwrap_or_else(|err| {
                panic!(
                    "ENVIRONMENT_BLOCKED: live Postgres unavailable for MT-195 runtime tests: {err}"
                )
            });
        sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
            .execute(&mut admin)
            .await
            .expect("create MT-195 isolated schema");
        drop(admin);

        let schema_url = append_schema_search_path(&base_url, &schema);
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&schema_url)
            .await
            .expect("connect MT-195 isolated schema");
        let control_pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&base_url)
            .await
            .expect("connect MT-195 control Postgres pool");
        create_runtime_tables(&pool).await;
        assert_current_schema(&pool, &schema).await;

        let artifact_dir = create_artifact_dir(label);

        Self {
            pool,
            control_pool,
            schema,
            schema_url,
            artifact_dir,
        }
    }

    pub async fn new_production(label: &str) -> Self {
        let base_url = postgres_url();
        let schema = format!("mt195_production_{}", Uuid::now_v7().simple());
        let mut admin = PgConnection::connect(&base_url)
            .await
            .unwrap_or_else(|err| {
                panic!(
                    "ENVIRONMENT_BLOCKED: live Postgres unavailable for MT-195 production runtime tests: {err}"
                )
            });
        sqlx::query(&format!(r#"CREATE SCHEMA "{schema}""#))
            .execute(&mut admin)
            .await
            .expect("create MT-195 production isolated schema");
        drop(admin);

        let schema_url = append_schema_search_path(&base_url, &schema);
        let db = PostgresDatabase::connect(&schema_url, 5)
            .await
            .expect("connect MT-195 production storage");
        db.run_migrations()
            .await
            .expect("run MT-195 production migrations");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&schema_url)
            .await
            .expect("connect MT-195 production isolated schema");
        let control_pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&base_url)
            .await
            .expect("connect MT-195 production control Postgres pool");
        assert_current_schema(&pool, &schema).await;

        let artifact_dir = create_artifact_dir(label);

        Self {
            pool,
            control_pool,
            schema,
            schema_url,
            artifact_dir,
        }
    }

    pub fn schema(&self) -> &str {
        &self.schema
    }

    pub fn schema_url(&self) -> &str {
        &self.schema_url
    }

    pub fn artifact_dir(&self) -> &Path {
        &self.artifact_dir
    }

    async fn schema_isolation_verified(&self) -> bool {
        let current_schema: String = sqlx::query_scalar("SELECT current_schema()")
            .fetch_one(&self.pool)
            .await
            .expect("read current schema");
        let table_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM information_schema.tables
            WHERE table_schema = $1
              AND table_name = ANY($2)
            "#,
        )
        .bind(&self.schema)
        .bind(RUNTIME_TABLES)
        .fetch_one(&self.control_pool)
        .await
        .expect("count isolated schema runtime tables");
        current_schema == self.schema && table_count == RUNTIME_TABLES.len() as i64
    }

    async fn production_schema_isolation_verified(&self) -> bool {
        let current_schema: String = sqlx::query_scalar("SELECT current_schema()")
            .fetch_one(&self.pool)
            .await
            .expect("read current production schema");
        let table_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM information_schema.tables
            WHERE table_schema = $1
              AND table_name = ANY($2)
            "#,
        )
        .bind(&self.schema)
        .bind(PRODUCTION_RESTART_RESUME_TABLES)
        .fetch_one(&self.control_pool)
        .await
        .expect("count isolated production restart-resume tables");
        current_schema == self.schema
            && table_count == PRODUCTION_RESTART_RESUME_TABLES.len() as i64
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FailureScenario {
    EventSeqGap,
    OperatorCancelDuringRecovery,
}

#[derive(Debug, Serialize)]
pub struct FailedRecoveryEvidence {
    pub schema: String,
    pub artifact_report: PathBuf,
    pub report_rows: i64,
    pub resumed_sessions: usize,
    pub failed_sessions: usize,
    pub operator_decision_rows: i64,
    pub failed_checkpoint_rows: i64,
    pub report_contains_recovery_failed_event: bool,
}

#[derive(Debug, Serialize)]
pub struct BackendTerminationEvidence {
    pub schema: String,
    pub artifact_report: PathBuf,
    pub backend_termination_observed: bool,
    pub report_rows: i64,
    pub resumed_sessions: usize,
    pub failed_sessions: usize,
    pub operator_decision_rows: i64,
    pub report_contains_db_unavailable_event: bool,
    pub schema_isolation_verified: bool,
}

#[derive(Debug, Serialize)]
pub struct TransientDbBackoffEvidence {
    pub schema: String,
    pub artifact_report: PathBuf,
    pub db_unavailable_attempts: u32,
    pub backoff_observed: bool,
    pub backoff_delay_ms: Vec<u64>,
    pub report_rows: i64,
    pub resumed_sessions: usize,
    pub failed_sessions: usize,
    pub report_contains_db_unavailable_event: bool,
    pub schema_isolation_verified: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeSeed {
    pub session_id: Uuid,
    model_session_id: Uuid,
    last_seq: i64,
    event_seqs: &'static [i64],
    label: &'static str,
}

impl RuntimeSeed {
    pub fn success(label: &'static str) -> Self {
        Self {
            session_id: Uuid::now_v7(),
            model_session_id: Uuid::now_v7(),
            last_seq: 0,
            event_seqs: &[1, 2],
            label,
        }
    }

    fn event_gap() -> Self {
        Self {
            session_id: Uuid::now_v7(),
            model_session_id: Uuid::now_v7(),
            last_seq: 0,
            event_seqs: &[1, 3],
            label: "event_gap",
        }
    }
}

pub async fn run_failed_recovery(scenario: FailureScenario) -> FailedRecoveryEvidence {
    let label = match scenario {
        FailureScenario::EventSeqGap => "event-gap",
        FailureScenario::OperatorCancelDuringRecovery => "operator-cancel",
    };
    let pg = RuntimePg::new(label).await;
    let seed = match scenario {
        FailureScenario::EventSeqGap => RuntimeSeed::event_gap(),
        FailureScenario::OperatorCancelDuringRecovery => RuntimeSeed::success("operator_cancel"),
    };
    insert_seed(&pg.pool, seed)
        .await
        .expect("insert failed-recovery seed");

    let broker = match scenario {
        FailureScenario::EventSeqGap => recovery_broker(pg.pool.clone()),
        FailureScenario::OperatorCancelDuringRecovery => {
            PostgresRecoveryBroker::operator_cancel(pg.pool.clone())
        }
    };
    let report = handshake_core::session_checkpoint::RestartResumeOrchestrator::run(&broker);

    let artifact_report = pg.artifact_dir().join("report.json");
    let evidence = FailedRecoveryEvidence {
        schema: pg.schema().to_string(),
        artifact_report: artifact_report.clone(),
        report_rows: count_rows(&pg.pool, "kernel_restart_resume_report").await,
        resumed_sessions: report.sessions_resumed.len(),
        failed_sessions: report.sessions_recovery_failed.len(),
        operator_decision_rows: count_rows(&pg.pool, "kernel_operator_decision_request").await,
        failed_checkpoint_rows: count_checkpoint_status(&pg.pool, "recovery_failed").await,
        report_contains_recovery_failed_event: fr_event_exists(
            &pg.pool,
            FrEventId::RestartResumeSessionRecoveryFailed,
        )
        .await,
    };
    write_artifact_json(&artifact_report, &evidence);
    evidence
}

pub async fn run_backend_termination_recovery() -> BackendTerminationEvidence {
    let pg = RuntimePg::new("backend-termination").await;
    let seed = RuntimeSeed::success("backend_termination");
    insert_seed(&pg.pool, seed)
        .await
        .expect("insert backend-termination seed");

    let mut victim = PgConnection::connect(pg.schema_url())
        .await
        .expect("connect MT-195 backend-termination victim");
    let victim_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut victim)
        .await
        .expect("read victim backend pid");
    let terminated = sqlx::query_scalar::<_, bool>("SELECT pg_terminate_backend($1)")
        .bind(victim_pid)
        .fetch_one(&pg.control_pool)
        .await
        .unwrap_or_else(|err| {
            panic!("ENVIRONMENT_BLOCKED: pg_terminate_backend unavailable for MT-195: {err}")
        });
    assert!(
        terminated,
        "ENVIRONMENT_BLOCKED: pg_terminate_backend returned false for MT-195 victim backend"
    );

    let broker = PostgresRecoveryBroker::backend_loss(pg.pool.clone(), victim);
    let observed = Arc::clone(&broker.backend_termination_observed);
    let report = handshake_core::session_checkpoint::RestartResumeOrchestrator::run(&broker);
    let backend_termination_observed = observed.load(Ordering::SeqCst);

    let artifact_report = pg.artifact_dir().join("report.json");
    let evidence = BackendTerminationEvidence {
        schema: pg.schema().to_string(),
        artifact_report: artifact_report.clone(),
        backend_termination_observed,
        report_rows: count_rows(&pg.pool, "kernel_restart_resume_report").await,
        resumed_sessions: report.sessions_resumed.len(),
        failed_sessions: report.sessions_recovery_failed.len(),
        operator_decision_rows: count_rows(&pg.pool, "kernel_operator_decision_request").await,
        report_contains_db_unavailable_event: fr_event_exists(
            &pg.pool,
            FrEventId::RestartResumeDbUnavailable,
        )
        .await,
        schema_isolation_verified: pg.schema_isolation_verified().await,
    };
    write_artifact_json(&artifact_report, &evidence);
    evidence
}

pub async fn run_transient_db_unavailable_backoff_recovery() -> TransientDbBackoffEvidence {
    let pg = RuntimePg::new_production("transient-db-unavailable-backoff").await;
    seed_production_resume_candidate(&pg.pool, "transient_db_backoff").await;

    let closed_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(pg.schema_url())
        .await
        .expect("connect MT-195 closed-pool transient DB fixture");
    closed_pool.close().await;

    let live_pool = pg.pool.clone();
    let mut attempt = 0_u32;
    let backoff = PostgresRestartResumeRunner::run_with_db_backoff(
        || {
            attempt += 1;
            let pool = if attempt == 1 {
                closed_pool.clone()
            } else {
                live_pool.clone()
            };
            async move { Ok(pool) }
        },
        RestartResumeDbBackoffPolicy::new(2, Duration::from_millis(1)),
    )
    .await
    .expect("run transient DB-unavailable restart-resume backoff");

    let artifact_report = pg.artifact_dir().join("transient-db-backoff-report.json");
    let evidence = TransientDbBackoffEvidence {
        schema: pg.schema().to_string(),
        artifact_report: artifact_report.clone(),
        db_unavailable_attempts: backoff.db_unavailable_attempts,
        backoff_observed: backoff.backoff_observed,
        backoff_delay_ms: backoff.backoff_delay_ms,
        report_rows: count_production_report_rows(&pg.pool).await,
        resumed_sessions: backoff.report.sessions_resumed.len(),
        failed_sessions: backoff.report.sessions_recovery_failed.len(),
        report_contains_db_unavailable_event: production_report_contains_fr_event(
            &pg.pool,
            FrEventId::RestartResumeDbUnavailable,
        )
        .await,
        schema_isolation_verified: pg.production_schema_isolation_verified().await,
    };
    write_artifact_json(&artifact_report, &evidence);
    evidence
}

pub fn recovery_broker(pool: PgPool) -> PostgresRecoveryBroker {
    PostgresRecoveryBroker::new(pool)
}

pub struct PostgresRecoveryBroker {
    pool: PgPool,
    operator_cancel: bool,
    terminated_connection: Mutex<Option<PgConnection>>,
    backend_termination_observed: Arc<AtomicBool>,
}

impl PostgresRecoveryBroker {
    fn new(pool: PgPool) -> Self {
        Self {
            pool,
            operator_cancel: false,
            terminated_connection: Mutex::new(None),
            backend_termination_observed: Arc::new(AtomicBool::new(false)),
        }
    }

    fn operator_cancel(pool: PgPool) -> Self {
        Self {
            operator_cancel: true,
            ..Self::new(pool)
        }
    }

    fn backend_loss(pool: PgPool, terminated_connection: PgConnection) -> Self {
        Self {
            pool,
            operator_cancel: false,
            terminated_connection: Mutex::new(Some(terminated_connection)),
            backend_termination_observed: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl ResumableSession for PostgresRecoveryBroker {
    type State = Value;

    fn list_resumable_sessions(&self) -> Vec<(Uuid, SessionCheckpoint, Vec<EventLedgerRow>)> {
        block_on_runtime(async {
            let checkpoints = load_running_checkpoints(&self.pool).await;
            let mut candidates = Vec::with_capacity(checkpoints.len());
            for checkpoint in checkpoints {
                let events = load_events(&self.pool, checkpoint.session_id).await;
                candidates.push((checkpoint.session_id, checkpoint, events));
            }
            candidates
        })
    }

    fn apply_event(&self, state: &mut Value, event: &EventLedgerRow) -> Result<(), ReplayError> {
        if let Some(mut terminated_connection) = self.terminated_connection.lock().unwrap().take() {
            let probe = block_on_runtime(async {
                sqlx::query("SELECT 1")
                    .execute(&mut terminated_connection)
                    .await
            });
            if probe.is_err() {
                self.backend_termination_observed
                    .store(true, Ordering::SeqCst);
                persist_fr_event(
                    &self.pool,
                    None,
                    FrEventId::RestartResumeDbUnavailable,
                    json!({
                        "session_id": event.session_id,
                        "event_seq": event.event_sequence,
                        "source": "pg_terminate_backend",
                    }),
                );
                return Err(ReplayError::StateInvariantViolated {
                    seq: event.event_sequence,
                    invariant: "postgres_backend_terminated".to_string(),
                });
            }
            return Err(ReplayError::StateInvariantViolated {
                seq: event.event_sequence,
                invariant: "terminated backend unexpectedly accepted query".to_string(),
            });
        }

        let counter = state.get("counter").and_then(Value::as_i64).unwrap_or(0)
            + event.payload.get("by").and_then(Value::as_i64).unwrap_or(0);
        *state = json!({ "counter": counter });
        Ok(())
    }

    fn seed_state(&self, checkpoint: &SessionCheckpoint) -> Value {
        checkpoint.compact_state.clone()
    }

    fn resume(&self, session_id: Uuid, final_state: Value) -> Result<(), String> {
        if self.operator_cancel {
            return Err("operator_cancel_during_recovery".to_string());
        }
        block_on_runtime(async {
            sqlx::query(
                r#"
                UPDATE kernel_session_checkpoint
                SET recovery_status = 'resumed', compact_state = $2
                WHERE session_id = $1
                "#,
            )
            .bind(session_id)
            .bind(final_state)
            .execute(&self.pool)
            .await
        })
        .map(|_| ())
        .map_err(|err| err.to_string())
    }

    fn mark_recovery_failed(&self, session_id: Uuid, error: &ResumeError) -> Result<(), String> {
        let error_json = serde_json::to_value(error).map_err(|err| err.to_string())?;
        block_on_runtime(async {
            sqlx::query(
                r#"
                UPDATE kernel_session_checkpoint
                SET recovery_status = 'recovery_failed',
                    compact_state = jsonb_set(compact_state, '{recovery_error}', $2, true)
                WHERE session_id = $1
                "#,
            )
            .bind(session_id)
            .bind(error_json)
            .execute(&self.pool)
            .await
        })
        .map(|_| ())
        .map_err(|err| err.to_string())
    }

    fn request_operator_decision(
        &self,
        session_id: Uuid,
        error: &ResumeError,
    ) -> Result<(), String> {
        let reason = serde_json::to_value(error).map_err(|err| err.to_string())?;
        block_on_runtime(async {
            sqlx::query(
                r#"
                INSERT INTO kernel_operator_decision_request (
                    request_id,
                    session_id,
                    reason
                )
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(session_id)
            .bind(reason)
            .execute(&self.pool)
            .await
        })
        .map(|_| ())
        .map_err(|err| err.to_string())
    }

    fn persist_resume_report(&self, report: &ResumeReport) -> Result<(), String> {
        let report_json = serde_json::to_value(report).map_err(|err| err.to_string())?;
        block_on_runtime(async {
            sqlx::query(
                r#"
                INSERT INTO kernel_restart_resume_report (
                    report_id,
                    report,
                    sessions_examined,
                    sessions_resumed,
                    sessions_recovery_failed
                )
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(report.report_id)
            .bind(report_json)
            .bind(report.sessions_examined as i32)
            .bind(report.sessions_resumed.len() as i32)
            .bind(report.sessions_recovery_failed.len() as i32)
            .execute(&self.pool)
            .await?;
            sqlx::query(
                r#"
                UPDATE kernel_operator_decision_request
                SET report_id = $1
                WHERE report_id IS NULL
                "#,
            )
            .bind(report.report_id)
            .execute(&self.pool)
            .await?;
            Ok::<(), sqlx::Error>(())
        })
        .map_err(|err| err.to_string())
    }

    fn emit_restart_resume_event(&self, event_id: FrEventId, payload: Value) {
        let report_id = payload
            .get("report_id")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok());
        persist_fr_event(&self.pool, report_id, event_id, payload);
    }
}

pub async fn insert_seed(pool: &PgPool, seed: RuntimeSeed) -> Result<(), RuntimeTestError> {
    let checkpoint = SessionCheckpoint::new(
        seed.session_id,
        seed.model_session_id,
        seed.last_seq,
        json!({ "counter": 0, "label": seed.label }),
        CheckpointStateKind::Periodic,
    )
    .expect("build MT-195 runtime checkpoint");
    sqlx::query(
        r#"
        INSERT INTO kernel_session_checkpoint (
            checkpoint_id,
            session_id,
            model_session_id,
            last_event_ledger_seq,
            compact_state,
            state_kind,
            created_at_utc,
            created_by_process,
            schema_version,
            recovery_status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'running')
        "#,
    )
    .bind(checkpoint.checkpoint_id.as_uuid())
    .bind(checkpoint.session_id)
    .bind(checkpoint.model_session_id)
    .bind(checkpoint.last_event_ledger_seq)
    .bind(checkpoint.compact_state)
    .bind(checkpoint.state_kind.as_str())
    .bind(checkpoint.created_at_utc)
    .bind(checkpoint.created_by_process)
    .bind(checkpoint.schema_version as i32)
    .execute(pool)
    .await?;

    for seq in seed.event_seqs {
        sqlx::query(
            r#"
            INSERT INTO kernel_event_ledger (
                event_id,
                event_sequence,
                session_id,
                event_type,
                payload,
                created_at_utc
            )
            VALUES ($1, $2, $3, 'increment', $4, NOW())
            "#,
        )
        .bind(format!("{}-{seq}", seed.label))
        .bind(*seq)
        .bind(seed.session_id)
        .bind(json!({ "by": 1 }))
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn seed_production_resume_candidate(pool: &PgPool, label: &'static str) {
    let session_id = Uuid::now_v7();
    let session_run_id = format!("SR-{session_id}");
    let kernel_task_run_id = format!("KTR-{session_id}");
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
        VALUES ($1, $2, 'mt195-db-backoff', 'RUNNING', 'mt195-previous-worker', NOW() + INTERVAL '30 minutes', 1, NOW(), NOW(), NOW())
        "#,
    )
    .bind(&session_run_id)
    .bind(&kernel_task_run_id)
    .execute(pool)
    .await
    .expect("seed production DB-backoff session queue");

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
        VALUES ($1, $2, $3, 0, $4, 'periodic', '[]'::jsonb, NOW(), $5, 1)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(session_id)
    .bind(Uuid::now_v7())
    .bind(json!({ "counter": 0, "label": label }))
    .bind(std::process::id() as i32)
    .execute(pool)
    .await
    .expect("seed production DB-backoff checkpoint");

    for by in [1_i64, 2_i64] {
        let event_id = format!("KE-{}", Uuid::now_v7());
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
                    'MODEL_RESPONSE_RECORDED', 'session_broker', 'mt195-db-backoff',
                    '0000000000000000000000000000000000000000000000000000000000000000',
                    'mt195-db-backoff', $5)
            "#,
        )
        .bind(&event_id)
        .bind(&kernel_task_run_id)
        .bind(&session_run_id)
        .bind(format!("mt195-db-backoff-{event_id}"))
        .bind(json!({ "by": by }))
        .execute(pool)
        .await
        .expect("seed production DB-backoff event");
    }
}

async fn count_production_report_rows(pool: &PgPool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM kernel_restart_resume_report")
        .fetch_one(pool)
        .await
        .expect("count production restart-resume reports")
}

async fn production_report_contains_fr_event(pool: &PgPool, event_id: FrEventId) -> bool {
    let rows = sqlx::query("SELECT fr_events_emitted FROM kernel_restart_resume_report")
        .fetch_all(pool)
        .await
        .expect("load production restart-resume report FR events");
    rows.into_iter().any(|row| {
        row.get::<Value, _>("fr_events_emitted")
            .as_array()
            .is_some_and(|events| {
                events
                    .iter()
                    .any(|event| event.as_str() == Some(event_id.as_str()))
            })
    })
}

pub async fn count_rows(pool: &PgPool, table: &str) -> i64 {
    assert!(
        RUNTIME_TABLES.contains(&table),
        "unexpected MT-195 runtime table: {table}"
    );
    sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .expect("count MT-195 runtime rows")
}

pub fn write_artifact_json(path: &Path, value: &impl Serialize) {
    let bytes = serde_json::to_vec_pretty(value).expect("serialize MT-195 artifact JSON");
    std::fs::write(path, bytes).expect("write MT-195 artifact JSON");
}

async fn create_runtime_tables(pool: &PgPool) {
    for statement in [
        r#"
        CREATE TABLE kernel_session_checkpoint (
            checkpoint_id UUID PRIMARY KEY,
            session_id UUID NOT NULL,
            model_session_id UUID NOT NULL,
            last_event_ledger_seq BIGINT NOT NULL,
            compact_state JSONB NOT NULL,
            state_kind TEXT NOT NULL,
            created_at_utc TIMESTAMPTZ NOT NULL,
            created_by_process INTEGER NOT NULL,
            schema_version INTEGER NOT NULL,
            recovery_status TEXT NOT NULL
        )
        "#,
        r#"
        CREATE TABLE kernel_event_ledger (
            event_id TEXT PRIMARY KEY,
            event_sequence BIGINT NOT NULL,
            session_id UUID NOT NULL,
            event_type TEXT NOT NULL,
            payload JSONB NOT NULL,
            created_at_utc TIMESTAMPTZ NOT NULL
        )
        "#,
        r#"
        CREATE TABLE kernel_restart_resume_report (
            report_id UUID PRIMARY KEY,
            report JSONB NOT NULL,
            sessions_examined INTEGER NOT NULL,
            sessions_resumed INTEGER NOT NULL,
            sessions_recovery_failed INTEGER NOT NULL,
            persisted_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        r#"
        CREATE TABLE kernel_operator_decision_request (
            request_id UUID PRIMARY KEY,
            report_id UUID NULL,
            session_id UUID NOT NULL,
            reason JSONB NOT NULL,
            requested_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
        r#"
        CREATE TABLE kernel_runtime_process_evidence (
            process_id UUID PRIMARY KEY,
            session_id UUID NOT NULL,
            os_pid INTEGER NOT NULL,
            role TEXT NOT NULL,
            state TEXT NOT NULL,
            started_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            checkpoint_committed_at_utc TIMESTAMPTZ NULL,
            graceful_stop_at_utc TIMESTAMPTZ NULL,
            killed_observed_at_utc TIMESTAMPTZ NULL
        )
        "#,
        r#"
        CREATE TABLE kernel_runtime_fr_event (
            seq BIGSERIAL PRIMARY KEY,
            report_id UUID NULL,
            event_id TEXT NOT NULL,
            payload JSONB NOT NULL,
            emitted_at_utc TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .expect("create MT-195 runtime table");
    }
}

async fn assert_current_schema(pool: &PgPool, expected: &str) {
    let current_schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(pool)
        .await
        .expect("read current schema");
    assert_eq!(current_schema, expected);
}

async fn load_running_checkpoints(pool: &PgPool) -> Vec<SessionCheckpoint> {
    let rows = sqlx::query(
        r#"
        SELECT checkpoint_id,
               session_id,
               model_session_id,
               last_event_ledger_seq,
               compact_state,
               state_kind,
               created_at_utc,
               created_by_process,
               schema_version
        FROM kernel_session_checkpoint
        WHERE recovery_status = 'running'
        ORDER BY created_at_utc, session_id
        "#,
    )
    .fetch_all(pool)
    .await
    .expect("load running checkpoints");
    rows.into_iter().map(row_to_checkpoint).collect()
}

async fn load_events(pool: &PgPool, session_id: Uuid) -> Vec<EventLedgerRow> {
    let rows = sqlx::query(
        r#"
        SELECT event_id,
               event_sequence,
               session_id,
               event_type,
               payload,
               created_at_utc
        FROM kernel_event_ledger
        WHERE session_id = $1
        ORDER BY event_sequence
        "#,
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .expect("load event ledger rows");
    rows.into_iter().map(row_to_event).collect()
}

fn row_to_checkpoint(row: PgRow) -> SessionCheckpoint {
    let state_kind: String = row.get("state_kind");
    let schema_version: i32 = row.get("schema_version");
    SessionCheckpoint {
        checkpoint_id: SessionCheckpointId(row.get("checkpoint_id")),
        session_id: row.get("session_id"),
        model_session_id: row.get("model_session_id"),
        last_event_ledger_seq: row.get("last_event_ledger_seq"),
        compact_state: row.get("compact_state"),
        state_kind: parse_state_kind(&state_kind),
        pending_artifacts: Vec::new(),
        created_at_utc: row.get::<DateTime<Utc>, _>("created_at_utc"),
        created_by_process: row.get("created_by_process"),
        schema_version: schema_version as u16,
    }
}

fn row_to_event(row: PgRow) -> EventLedgerRow {
    EventLedgerRow {
        event_id: row.get("event_id"),
        event_sequence: row.get("event_sequence"),
        session_id: row.get("session_id"),
        event_type: row.get("event_type"),
        payload: row.get("payload"),
        created_at: row.get::<DateTime<Utc>, _>("created_at_utc"),
    }
}

fn parse_state_kind(value: &str) -> CheckpointStateKind {
    match value {
        "periodic" => CheckpointStateKind::Periodic,
        "event_triggered" => CheckpointStateKind::EventTriggered,
        "pre_shutdown" => CheckpointStateKind::PreShutdown,
        "post_failure" => CheckpointStateKind::PostFailure,
        other => panic!("unexpected MT-195 checkpoint state kind: {other}"),
    }
}

async fn count_checkpoint_status(pool: &PgPool, status: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM kernel_session_checkpoint
        WHERE recovery_status = $1
        "#,
    )
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("count checkpoint recovery status")
}

async fn fr_event_exists(pool: &PgPool, event_id: FrEventId) -> bool {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM kernel_runtime_fr_event WHERE event_id = $1
        )
        "#,
    )
    .bind(event_id.as_str())
    .fetch_one(pool)
    .await
    .expect("check runtime FR event evidence")
}

fn persist_fr_event(pool: &PgPool, report_id: Option<Uuid>, event_id: FrEventId, payload: Value) {
    block_on_runtime(async {
        sqlx::query(
            r#"
            INSERT INTO kernel_runtime_fr_event (report_id, event_id, payload)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(report_id)
        .bind(event_id.as_str())
        .bind(payload)
        .execute(pool)
        .await
    })
    .expect("persist MT-195 runtime FR event");
}

fn block_on_runtime<F: Future>(future: F) -> F::Output {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(future))
    } else {
        tokio::runtime::Runtime::new()
            .expect("create MT-195 helper runtime")
            .block_on(future)
    }
}

fn postgres_url() -> String {
    block_on_runtime(handshake_core::storage::tests::postgres_test_base_url())
        .expect("resolve real PostgreSQL test URL")
}

fn append_schema_search_path(url: &str, schema: &str) -> String {
    let sep = if url.contains('?') { "&" } else { "?" };
    format!("{url}{sep}options=-csearch_path%3D{schema}")
}

fn artifact_root() -> PathBuf {
    if let Some(root) = std::env::var_os("HANDSHAKE_ARTIFACT_ROOT") {
        return PathBuf::from(root);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for ancestor in manifest.ancestors() {
        if ancestor.file_name().and_then(|name| name.to_str()) == Some("Handshake Worktrees") {
            if let Some(handshake_root) = ancestor.parent() {
                return handshake_root.join("Handshake_Artifacts");
            }
        }
    }
    manifest
        .join("..")
        .join("..")
        .join("..")
        .join("Handshake_Artifacts")
}

fn create_artifact_dir(label: &str) -> PathBuf {
    let artifact_dir = artifact_root()
        .join("handshake-test")
        .join("mt195-runtime")
        .join(format!(
            "{}-{}-{}",
            chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
            label,
            Uuid::now_v7().simple()
        ));
    std::fs::create_dir_all(&artifact_dir).expect("create MT-195 artifact directory");
    artifact_dir
}
