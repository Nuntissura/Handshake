use std::{
    collections::{HashMap, HashSet},
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

#[derive(Clone, Default)]
struct SharedTraceBuffer(Arc<Mutex<Vec<u8>>>);

impl Write for SharedTraceBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedTraceBuffer {
    type Writer = SharedTraceBuffer;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

use async_trait::async_trait;
use chrono::Utc;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::time::{sleep, timeout};
use uuid::Uuid;

use handshake_core::process_ledger::reclaim::StaleSessionProcessSet;
use handshake_core::process_ledger::{
    spawn_staleness_reclaim_task, KillError, KillOutcome, LedgerEvent, LedgerOverflowEvent,
    PostgresProcessLedgerStore, ProcessEngineKind, ProcessLedgerError, ProcessLedgerOverflowSink,
    ProcessLedgerStore, ProcessLedgerWriter, ProcessStart, ProcessStop, Reclaim, ReclaimClaim,
    ReclaimKillOperation, ReclaimKillOperationCandidate, ReclaimKillOperationStatus,
    ReclaimKillOperationSweepOutcome, ReclaimProcessStore, ReclaimStopReservation,
    ReclaimStopWriter, ReclaimTrigger, ReclaimableProcess, SandboxKill, StaleSessionSource,
    StalenessReclaimConfig, WriterConfig, POSTGRES_ACTIVE_RECLAIM_QUERY_SQL,
    PROCESS_STOP_UPSERT_SQL,
};

mod knowledge_pg_support;

async fn reclaim_pg_pool(max_connections: u32) -> PgPool {
    let pg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("real task-owned PostgreSQL is required for reclaim proof");
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(&pg.schema_url)
        .await
        .expect("connect reclaim proof to isolated managed PostgreSQL schema")
}

struct ModelLaneStalenessSeed {
    lane_id: String,
    process_uuid: Uuid,
    status: &'static str,
    heartbeat_at_utc: Option<String>,
    reclaim_after_utc: Option<String>,
}

async fn insert_model_lane_staleness_event(
    pool: &PgPool,
    session_id: &str,
    aggregate_id: &str,
) -> (String, i64) {
    let event_id = format!("EVT-STALE-{}", Uuid::now_v7());
    let event_sequence = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO kernel_event_ledger (
            event_id, event_version, kernel_task_run_id, session_run_id,
            aggregate_type, aggregate_id, idempotency_key, event_type,
            actor_kind, actor_id, payload_hash, source_component, payload
        )
        VALUES ($1, '1', $2, $2, 'model_lane_staleness_test', $3, $4,
                'model_lane_staleness_test', 'test', 'process-ledger-reclaim-test',
                $5, 'process_ledger_reclaim_tests', '{}'::jsonb)
        RETURNING event_sequence
        "#,
    )
    .bind(&event_id)
    .bind(session_id)
    .bind(aggregate_id)
    .bind(format!("IDEM-STALE-{}", Uuid::now_v7()))
    .bind("0".repeat(64))
    .fetch_one(pool)
    .await
    .expect("insert model-lane staleness fixture event");
    (event_id, event_sequence)
}

async fn insert_model_lane_staleness_fixture(
    pool: &PgPool,
    session_id: &str,
    lanes: Vec<ModelLaneStalenessSeed>,
) {
    let run_id = format!("RUN-STALE-{}", Uuid::now_v7());
    let trace_id = format!("TRACE-STALE-{}", Uuid::now_v7());
    let event_stream_id = format!("STREAM-STALE-{}", Uuid::now_v7());
    let (run_event_id, run_event_sequence) =
        insert_model_lane_staleness_event(pool, session_id, &run_id).await;
    sqlx::query(
        r#"
        INSERT INTO model_lane_runs (
            run_id, trace_id, run_span_id, coordinator_session_id,
            work_packet_id, micro_task_id, task_board_id, owner_session,
            idempotency_key, replay_order_key, event_ledger_stream_id,
            event_ledger_event_id, event_ledger_seq, record_json
        )
        VALUES ($1,$2,$3,$4,'WP-STALE-TEST','MT-STALE-TEST','TB-STALE-TEST',
                'OWNER-STALE-TEST',$5,$6,$7,$8,$9,$10)
        "#,
    )
    .bind(&run_id)
    .bind(&trace_id)
    .bind(format!("SPAN-{run_id}"))
    .bind(session_id)
    .bind(format!("IDEM-RUN-{run_id}"))
    .bind(format!("REPLAY-RUN-{run_id}"))
    .bind(&event_stream_id)
    .bind(&run_event_id)
    .bind(run_event_sequence)
    .bind(serde_json::json!({
        "run_id": run_id.clone(),
        "coordinator_session_id": session_id,
    }))
    .execute(pool)
    .await
    .expect("insert model-lane staleness fixture run");

    for lane in lanes {
        let (lane_event_id, lane_event_sequence) =
            insert_model_lane_staleness_event(pool, session_id, &lane.lane_id).await;
        let lane_id = lane.lane_id.clone();
        let record = serde_json::json!({
            "lane_id": lane_id.clone(),
            "run_id": run_id.clone(),
            "coordinator_session_id": session_id,
            "process_ownership_ref": format!("process-ledger://{}", lane.process_uuid),
            "status": lane.status,
            "heartbeat_at_utc": lane.heartbeat_at_utc,
            "reclaim_after_utc": lane.reclaim_after_utc,
        });
        sqlx::query(
            r#"
            INSERT INTO model_lanes (
                lane_id, run_id, trace_id, lane_span_id, kind, runtime_binding,
                launch_authority, status, work_packet_id, micro_task_id,
                task_board_id, owner_session, event_ledger_stream_id,
                event_ledger_event_id, event_ledger_seq, record_json
            )
            VALUES ($1,$2,$3,$4,'worker','local','session_broker',$5,
                    'WP-STALE-TEST','MT-STALE-TEST','TB-STALE-TEST','OWNER-STALE-TEST',
                    $6,$7,$8,$9)
            "#,
        )
        .bind(&lane_id)
        .bind(&run_id)
        .bind(&trace_id)
        .bind(format!("SPAN-{lane_id}"))
        .bind(lane.status)
        .bind(&event_stream_id)
        .bind(&lane_event_id)
        .bind(lane_event_sequence)
        .bind(record)
        .execute(pool)
        .await
        .expect("insert model-lane staleness fixture lane");
    }
}

#[cfg(target_os = "windows")]
#[tokio::test]
async fn production_reclaim_kills_real_process_and_durably_closes_postgres_lifecycle() {
    use std::{
        os::windows::process::CommandExt,
        process::{Child, Command, Stdio},
    };

    use handshake_core::process_ledger::{
        NoopOverflowSink, PostgresProcessLedgerStore, ProductionSandboxKill,
    };
    use sha2::{Digest, Sha256};
    use sqlx::postgres::PgPoolOptions;

    struct ChildCleanup(Child);

    impl Drop for ChildCleanup {
        fn drop(&mut self) {
            if matches!(self.0.try_wait(), Ok(None)) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let pg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("managed PostgreSQL is required for the production reclaim proof");
    let pool = PgPoolOptions::new()
        .max_connections(6)
        .connect(&pg.schema_url)
        .await
        .expect("connect production reclaim proof to isolated PostgreSQL schema");
    let store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));

    let system_root = std::env::var_os("SystemRoot").expect("Windows SystemRoot must be set");
    let executable = std::path::PathBuf::from(system_root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    assert!(
        executable.is_file(),
        "production reclaim proof executable must exist: {}",
        executable.display()
    );
    let executable_sha256 = hex::encode(Sha256::digest(
        std::fs::read(&executable).expect("read proof executable for immutable identity"),
    ));
    let mut child = ChildCleanup(
        Command::new(&executable)
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 120",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("spawn hidden real process for production reclaim proof"),
    );

    let session = format!("SR-PRODUCTION-RECLAIM-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    let os_pid = child.0.id();
    let os_creation_time_100ns =
        handshake_core::sandbox::handshake_native::process_creation_time_100ns(os_pid)
            .expect("attest exact proof-process generation");
    let start = ProcessStart::new(ProcessEngineKind::HelperSubprocess, "RECLAIM_PROOF", None)
        .with_process_uuid(process_uuid)
        .with_parent_session_id(session.clone())
        .with_sandbox_adapter_id("handshake_native")
        .with_sandbox_internal_id(format!("handshake.native.reclaim-proof.{process_uuid}"))
        .with_os_pid(os_pid)
        .with_metadata_jsonb(serde_json::json!({
            "effective_executable_sha256": executable_sha256,
            "os_creation_time_100ns": os_creation_time_100ns,
            "execution_policy_ref": "hsk.execution_policy.cli_bridge.effective@1",
            "proof_kind": "real_process_real_postgres_production_reclaim"
        }));
    store
        .write_batch(vec![LedgerEvent::Start(start)])
        .await
        .expect("durably seed real child START");

    let config = WriterConfig {
        capacity: 8,
        batch_size: 1,
        flush_interval: Duration::from_millis(5),
    };
    let (writer, writer_join) = ProcessLedgerWriter::spawn(
        Arc::clone(&store) as Arc<dyn ProcessLedgerStore>,
        Arc::new(NoopOverflowSink),
        config,
    );
    let writer = Arc::new(writer);
    let killer = Arc::new(ProductionSandboxKill::new(
        pool.clone(),
        tokio::runtime::Handle::current(),
    ));
    let reclaim = Reclaim::new(Arc::clone(&store), killer, Arc::clone(&writer));
    let report = reclaim
        .run(&session, ReclaimTrigger::OperatorCancel)
        .await
        .expect("production reclaim must complete");

    assert_eq!(report.processes_reclaimed.len(), 1);
    assert_eq!(report.processes_reclaimed[0].process_uuid, process_uuid);
    assert_eq!(
        report.processes_reclaimed[0].kill_result,
        KillOutcome::Killed
    );
    assert_eq!(
        report.processes_reclaimed[0].stop_event_kind,
        Some(handshake_core::process_ledger::LedgerEventKind::Stop)
    );
    let exit = timeout(Duration::from_secs(2), async {
        loop {
            if let Some(status) = child.0.try_wait().expect("query reclaimed child status") {
                break status;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("production reclaimer must terminate and reap the real child");
    assert!(
        !exit.success(),
        "reclaimed child must not report successful natural exit"
    );

    let row: (
        Option<chrono::DateTime<Utc>>,
        Option<i32>,
        Option<String>,
        i64,
        serde_json::Value,
    ) = sqlx::query_as(
        "SELECT stopped_at, exit_code, stop_reason, os_pid, metadata_jsonb FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(process_uuid)
    .fetch_one(&pool)
    .await
    .expect("read durable production reclaim lifecycle");
    assert!(
        row.0.is_some(),
        "production reclaim must durably write STOP"
    );
    assert_eq!(row.1, Some(-1));
    assert_eq!(row.2.as_deref(), Some("reclaim"));
    assert_eq!(row.3, i64::from(os_pid));
    assert_eq!(row.4["reclaim_last_kill_operation"]["status"], "succeeded");

    drop(reclaim);
    drop(writer);
    writer_join
        .await
        .expect("production reclaim writer task joins")
        .expect("production reclaim writer drains");
}

#[tokio::test]
async fn postgres_stale_source_does_not_cross_reclaim_healthy_sibling_lane() {
    use handshake_core::process_ledger::{
        PostgresModelLaneStaleSessionSource, PostgresProcessLedgerStore,
    };
    use sqlx::postgres::PgPoolOptions;

    let pg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("managed PostgreSQL is required for the sibling staleness proof");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&pg.schema_url)
        .await
        .expect("connect sibling staleness proof to isolated PostgreSQL schema");
    let store = PostgresProcessLedgerStore::new(pool.clone());
    let runtime_lease = handshake_core::process_ledger::acquire_embedded_runtime_instance_lease(
        Uuid::now_v7(),
        "stale-source-test-host",
    )
    .expect("acquire stale-source runtime lease");
    let runtime_owner = runtime_lease.descriptor().process_runtime_owner();
    let session_id = format!("SR-STALE-SIBLING-{}", Uuid::now_v7());
    let terminal_process_uuid = Uuid::now_v7();
    let healthy_process_uuid = Uuid::now_v7();
    let terminal_start = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "STALE_SIBLING_TEST",
        None,
    )
    .with_process_uuid(terminal_process_uuid)
    .with_parent_session_id(session_id.clone())
    // `ProcessStart::new` defaults `sandbox_adapter_id` to None, and
    // `stale_sessions` has always filtered on `sandbox_adapter_id IS NOT NULL`.
    // Without this both rows are invisible to the scan, the session can never be
    // returned, and the negative assertion below is UNFALSIFIABLE: it would keep
    // passing even if cross-reclaim protection were deleted outright.
    .with_sandbox_adapter_id("handshake_native")
    .with_runtime_owner(runtime_owner.clone())
    .with_os_pid(41_001);
    let healthy_start = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "STALE_SIBLING_TEST",
        None,
    )
    .with_process_uuid(healthy_process_uuid)
    .with_parent_session_id(session_id.clone())
    .with_sandbox_adapter_id("handshake_native")
    .with_runtime_owner(runtime_owner)
    .with_os_pid(41_002);
    store
        .write_batch(vec![
            LedgerEvent::Start(terminal_start.clone()),
            LedgerEvent::Start(healthy_start),
        ])
        .await
        .expect("seed sibling process lifecycles");
    let now = Utc::now();
    insert_model_lane_staleness_fixture(
        &pool,
        &session_id,
        vec![
            ModelLaneStalenessSeed {
                lane_id: format!("LANE-TERMINAL-{}", Uuid::now_v7()),
                process_uuid: terminal_process_uuid,
                status: "completed",
                heartbeat_at_utc: Some(now.to_rfc3339()),
                reclaim_after_utc: None,
            },
            ModelLaneStalenessSeed {
                lane_id: format!("LANE-HEALTHY-{}", Uuid::now_v7()),
                process_uuid: healthy_process_uuid,
                status: "ready",
                heartbeat_at_utc: Some(now.to_rfc3339()),
                reclaim_after_utc: Some((now + chrono::Duration::minutes(6)).to_rfc3339()),
            },
        ],
    )
    .await;

    let source = PostgresModelLaneStaleSessionSource::new(pool, runtime_lease.descriptor().clone());
    let stale_sessions = source
        .stale_sessions(Duration::from_secs(300))
        .await
        .expect("scan exact lane ownership");

    // FALSIFIABILITY (MT-020 AC-6b): with `sandbox_adapter_id` seeded above both
    // rows are genuinely visible to the scan, so the ONLY reason this session is
    // withheld is the healthy sibling's non-reclaimable lane state. Verified by
    // temporarily flipping the LANE-HEALTHY seed's `status` from "ready" to
    // "failed" and re-running: the assertion below then FAILS
    // ("a terminal open sibling must not make the healthy open lane's process
    // reclaimable"). Before the `sandbox_adapter_id` fix the same flip changed
    // nothing and the test still passed — that is what made it vacuous.
    assert!(
        !stale_sessions.contains(&session_id),
        "a terminal open sibling must not make the healthy open lane's process reclaimable"
    );
}

#[tokio::test]
async fn postgres_stale_source_selects_terminal_lane_exact_process_owner() {
    use handshake_core::process_ledger::{
        PostgresModelLaneStaleSessionSource, PostgresProcessLedgerStore,
    };
    use sqlx::postgres::PgPoolOptions;

    let pg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("managed PostgreSQL is required for the exact-owner staleness proof");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&pg.schema_url)
        .await
        .expect("connect exact-owner staleness proof to isolated PostgreSQL schema");
    let store = PostgresProcessLedgerStore::new(pool.clone());
    let runtime_lease = handshake_core::process_ledger::acquire_embedded_runtime_instance_lease(
        Uuid::now_v7(),
        "stale-source-test-host",
    )
    .expect("acquire stale-source runtime lease");
    let session_id = format!("SR-STALE-EXACT-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    store
        .write_batch(vec![LedgerEvent::Start(
            ProcessStart::new(
                ProcessEngineKind::HelperSubprocess,
                "STALE_EXACT_OWNER_TEST",
                None,
            )
            .with_process_uuid(process_uuid)
            .with_parent_session_id(session_id.clone())
            // `ProcessStart::new` defaults `sandbox_adapter_id` to None and
            // `stale_sessions` has always filtered on
            // `sandbox_adapter_id IS NOT NULL`, so without this the row is
            // never visible to the scan and this session can never be
            // selected — the assertion below could only ever fail.
            .with_sandbox_adapter_id("handshake_native")
            .with_runtime_owner(runtime_lease.descriptor().process_runtime_owner())
            .with_os_pid(42_001),
        )])
        .await
        .expect("seed exact-owner open process lifecycle");
    insert_model_lane_staleness_fixture(
        &pool,
        &session_id,
        vec![ModelLaneStalenessSeed {
            lane_id: format!("LANE-EXACT-{}", Uuid::now_v7()),
            process_uuid,
            status: "failed",
            heartbeat_at_utc: Some(Utc::now().to_rfc3339()),
            reclaim_after_utc: None,
        }],
    )
    .await;

    let source = PostgresModelLaneStaleSessionSource::new(pool, runtime_lease.descriptor().clone());
    let stale_sessions = source
        .stale_sessions(Duration::from_secs(300))
        .await
        .expect("scan exact terminal lane ownership");

    assert!(
        stale_sessions.contains(&session_id),
        "a terminal lane must select its own still-open process session"
    );
}

/// MT-019 P-1 regression: an open, adapter-owned, self-owned lifecycle row with
/// a NULL `parent_session_id` must not abort the stale-session scan.
///
/// `parent_session_id` is nullable (migration 0021) and production writes such
/// rows: the official-CLI auth-status probe
/// (`model_runtime/cloud/access_config.rs`) sets only `session_id`, while the
/// row still carries `sandbox_adapter_id` and the live
/// `owner_runtime_instance_id`. Before the fix, `stale_sessions` selected that
/// row and decoded it with the panicking `row.get::<String>`, raising
/// `UnexpectedNullError` inside the spawned staleness task and silently killing
/// the periodic reclaimer for the rest of the process lifetime -- every later
/// boot re-armed it while the row stayed open.
///
/// The scan must return `Ok`, skip the session-less row, and still select a
/// genuinely reclaimable sibling session in the same sweep.
#[tokio::test]
async fn postgres_stale_source_skips_null_parent_session_row_without_aborting_scan() {
    use handshake_core::process_ledger::{
        PostgresModelLaneStaleSessionSource, PostgresProcessLedgerStore,
    };
    use sqlx::postgres::PgPoolOptions;

    let pg = knowledge_pg_support::knowledge_pg()
        .await
        .expect("managed PostgreSQL is required for the NULL-parent-session staleness proof");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&pg.schema_url)
        .await
        .expect("connect NULL-parent-session proof to isolated PostgreSQL schema");
    let store = PostgresProcessLedgerStore::new(pool.clone());
    let runtime_lease = handshake_core::process_ledger::acquire_embedded_runtime_instance_lease(
        Uuid::now_v7(),
        "stale-source-test-host",
    )
    .expect("acquire stale-source runtime lease");
    let runtime_owner = runtime_lease.descriptor().process_runtime_owner();

    // The auth-status-probe shape: open, adapter-owned, owned by THIS live
    // runtime instance, and deliberately carrying no parent session.
    let session_less_process_uuid = Uuid::now_v7();
    store
        .write_batch(vec![LedgerEvent::Start(
            ProcessStart::new(
                ProcessEngineKind::HelperSubprocess,
                "STALE_NULL_PARENT_SESSION_TEST",
                None,
            )
            .with_process_uuid(session_less_process_uuid)
            .with_sandbox_adapter_id("handshake_native")
            .with_runtime_owner(runtime_owner.clone())
            .with_os_pid(43_001),
        )])
        .await
        .expect("seed session-less adapter-owned open process lifecycle");

    // A genuinely reclaimable session in the same sweep, to prove the scan is
    // still functional rather than merely non-panicking.
    let session_id = format!("SR-STALE-NULLPARENT-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    store
        .write_batch(vec![LedgerEvent::Start(
            ProcessStart::new(
                ProcessEngineKind::HelperSubprocess,
                "STALE_NULL_PARENT_SESSION_TEST",
                None,
            )
            .with_process_uuid(process_uuid)
            .with_parent_session_id(session_id.clone())
            // `ProcessStart::new` defaults `sandbox_adapter_id` to None and the
            // staleness fixture only writes model_lane_runs/model_lanes, so
            // without this the row is filtered out by the scan's
            // `sandbox_adapter_id IS NOT NULL` predicate and the session could
            // never be returned -- making the assertion below unfalsifiable.
            .with_sandbox_adapter_id("handshake_native")
            .with_runtime_owner(runtime_owner)
            .with_os_pid(43_002),
        )])
        .await
        .expect("seed reclaimable sibling open process lifecycle");
    insert_model_lane_staleness_fixture(
        &pool,
        &session_id,
        vec![ModelLaneStalenessSeed {
            lane_id: format!("LANE-NULLPARENT-{}", Uuid::now_v7()),
            process_uuid,
            status: "failed",
            heartbeat_at_utc: Some(Utc::now().to_rfc3339()),
            reclaim_after_utc: None,
        }],
    )
    .await;

    let source = PostgresModelLaneStaleSessionSource::new(pool, runtime_lease.descriptor().clone());
    let stale_sessions = source
        .stale_sessions(Duration::from_secs(300))
        .await
        .expect("a NULL parent_session_id row must not abort the stale-session scan");

    assert!(
        stale_sessions.contains(&session_id),
        "the session-less row must be skipped without suppressing a genuinely reclaimable session"
    );
}

#[tokio::test]
async fn close_reclaim_with_no_open_processes_kills_nothing() {
    let fixture = Fixture::new(HashMap::new(), HashSet::new());

    let report = fixture
        .reclaim
        .run("SR-CLEAN-CLOSE", ReclaimTrigger::Close)
        .await
        .expect("clean close reclaim");

    assert_eq!(report.session_id, "SR-CLEAN-CLOSE");
    assert_eq!(report.trigger, ReclaimTrigger::Close);
    assert!(report.processes_reclaimed.is_empty());
    assert!(fixture.killer.killed().is_empty());
    assert!(fixture.stop_writer.stops().is_empty());
}

#[tokio::test]
async fn failure_reclaim_writes_stop_only_after_kill_succeeds() {
    let process_a = reclaimable("SR-FAIL", ProcessEngineKind::SandboxContainer);
    let process_b = reclaimable("SR-FAIL", ProcessEngineKind::HelperSubprocess);
    let fixture = Fixture::new(
        HashMap::from([(
            "SR-FAIL".to_string(),
            vec![process_a.clone(), process_b.clone()],
        )]),
        HashSet::from([process_b.process_uuid]),
    );

    let report = fixture
        .reclaim
        .run("SR-FAIL", ReclaimTrigger::Failure)
        .await
        .expect("failure reclaim");

    assert_eq!(report.processes_reclaimed.len(), 2);
    assert_eq!(
        report
            .processes_reclaimed
            .iter()
            .filter(|entry| entry.kill_result == KillOutcome::Killed)
            .count(),
        1
    );
    assert_eq!(
        report
            .processes_reclaimed
            .iter()
            .filter(|entry| matches!(entry.kill_result, KillOutcome::Failed { .. }))
            .count(),
        1
    );

    let stops = fixture.stop_writer.stops();
    assert_eq!(stops.len(), 1);
    assert!(stops.iter().all(|stop| stop.exit_code == Some(-1)));
    assert!(stops
        .iter()
        .any(|stop| stop.process_uuid == process_a.process_uuid));
    assert!(!stops
        .iter()
        .any(|stop| stop.process_uuid == process_b.process_uuid));
    let failed = report
        .processes_reclaimed
        .iter()
        .find(|entry| entry.process_uuid == process_b.process_uuid)
        .expect("failed kill remains in reclaim report");
    assert_eq!(failed.stop_event_kind, None);
}

#[tokio::test]
async fn operator_cancel_reclaim_kills_immediately() {
    let process = reclaimable("SR-CANCEL", ProcessEngineKind::PluginProcess);
    let fixture = Fixture::new(
        HashMap::from([("SR-CANCEL".to_string(), vec![process.clone()])]),
        HashSet::new(),
    );

    let report = fixture
        .reclaim
        .run("SR-CANCEL", ReclaimTrigger::OperatorCancel)
        .await
        .expect("operator cancel reclaim");

    assert_eq!(report.trigger, ReclaimTrigger::OperatorCancel);
    assert_eq!(fixture.killer.killed(), vec![process.process_uuid]);
    assert_eq!(fixture.stop_writer.stops().len(), 1);
}

#[tokio::test]
async fn staleness_background_task_reclaims_after_ttl_scan() {
    let process = reclaimable("SR-STALE", ProcessEngineKind::MechanicalJob);
    let fixture = Fixture::new(
        HashMap::from([("SR-STALE".to_string(), vec![process.clone()])]),
        HashSet::new(),
    );
    let stale_source = Arc::new(FakeStaleSource::scoped(
        vec!["SR-STALE".to_string()],
        vec![process.process_uuid],
    ));
    let handle = spawn_staleness_reclaim_task(
        Arc::clone(&fixture.reclaim),
        stale_source,
        StalenessReclaimConfig {
            ttl: Duration::from_millis(20),
            scan_interval: Duration::from_millis(10),
        },
    );

    timeout(Duration::from_secs(2), async {
        loop {
            if fixture.stop_writer.stops().len() == 1 {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("staleness reclaim should fire");
    handle.abort();

    let stops = fixture.stop_writer.stops();
    assert_eq!(stops[0].process_uuid, process.process_uuid);
    assert_eq!(stops[0].exit_code, Some(-1));
}

#[tokio::test]
async fn staleness_background_task_without_owner_scope_fails_closed_before_kill() {
    let session_id = "SR-STALE-UNSCOPED";
    let process = reclaimable(session_id, ProcessEngineKind::MechanicalJob);
    let fixture = Fixture::new(
        HashMap::from([(session_id.to_string(), vec![process])]),
        HashSet::new(),
    );
    let stale_source = Arc::new(FakeStaleSource::unscoped(vec![session_id.to_string()]));
    let scope_error = stale_source
        .require_runtime_owner_scope()
        .expect_err("unscoped source must return the stable fail-closed reason");
    assert_eq!(
        scope_error.to_string(),
        "PROCESS_LEDGER_INVALID_CONFIG: STALE_RECLAIM_OWNER_SCOPE_REQUIRED"
    );
    assert!(!scope_error.to_string().contains(session_id));
    let stale_source_for_task: Arc<dyn StaleSessionSource> = stale_source.clone();
    let trace_buffer = SharedTraceBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(trace_buffer.clone())
        .with_ansi(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("install isolated stale-reclaim tracing capture");
    let handle = spawn_staleness_reclaim_task(
        Arc::clone(&fixture.reclaim),
        stale_source_for_task,
        StalenessReclaimConfig {
            ttl: Duration::from_millis(20),
            scan_interval: Duration::from_millis(10),
        },
    );

    timeout(Duration::from_secs(2), async {
        while stale_source.scan_count() == 0 {
            sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the unscoped source must be scanned");
    sleep(Duration::from_millis(25)).await;
    handle.abort();

    let captured_logs = String::from_utf8(trace_buffer.0.lock().unwrap().clone())
        .expect("captured stale-reclaim logs are UTF-8");
    assert!(captured_logs.contains("STALE_RECLAIM_OWNER_SCOPE_REQUIRED"));
    assert!(
        !captured_logs.contains(session_id),
        "pre-authority stale-reclaim logs must not disclose the source-provided session id: {captured_logs}"
    );

    assert!(
        fixture.killer.killed().is_empty(),
        "missing stale-owner scope must fail closed before any kill"
    );
    assert!(
        fixture.stop_writer.stops().is_empty(),
        "missing stale-owner scope must never fabricate STOP"
    );
}

#[tokio::test]
async fn full_stop_queue_prevents_kill_and_releases_exact_claim() {
    let process = reclaimable("SR-FULL", ProcessEngineKind::SandboxContainer);
    let expected_claim = process.reclaim_claim.clone();
    let store = Arc::new(TrackingReclaimStore::new(process.clone()));
    let killer = Arc::new(RecordingKill {
        killed: Mutex::new(Vec::new()),
        failures: HashSet::new(),
    });
    let (writer, _drain) = ProcessLedgerWriter::new_manual(1, Arc::new(NoopOverflowSink))
        .expect("create one-slot writer");
    writer
        .append_start(ProcessStart::new(
            ProcessEngineKind::MechanicalJob,
            "queue-filler",
            None,
        ))
        .expect("fill writer queue");
    let reclaim = Reclaim::new(Arc::clone(&store), Arc::clone(&killer), Arc::new(writer));

    let error = reclaim
        .run("SR-FULL", ReclaimTrigger::Failure)
        .await
        .expect_err("full STOP queue must fail before kill");

    assert!(matches!(error, ProcessLedgerError::EnqueueDropped(_)));
    assert!(killer.killed().is_empty(), "full queue must prevent kill");
    assert_eq!(
        store.releases(),
        vec![(process.process_uuid, expected_claim)]
    );
}

#[tokio::test]
async fn closed_stop_queue_prevents_kill_and_releases_exact_claim() {
    let process = reclaimable("SR-CLOSED", ProcessEngineKind::SandboxContainer);
    let expected_claim = process.reclaim_claim.clone();
    let store = Arc::new(TrackingReclaimStore::new(process.clone()));
    let killer = Arc::new(RecordingKill {
        killed: Mutex::new(Vec::new()),
        failures: HashSet::new(),
    });
    let (writer, drain) = ProcessLedgerWriter::new_manual(1, Arc::new(NoopOverflowSink))
        .expect("create one-slot writer");
    drop(drain);
    let reclaim = Reclaim::new(Arc::clone(&store), Arc::clone(&killer), Arc::new(writer));

    let error = reclaim
        .run("SR-CLOSED", ReclaimTrigger::Failure)
        .await
        .expect_err("closed STOP queue must fail before kill");

    assert!(matches!(error, ProcessLedgerError::EnqueueDropped(_)));
    assert!(killer.killed().is_empty(), "closed queue must prevent kill");
    assert_eq!(
        store.releases(),
        vec![(process.process_uuid, expected_claim)]
    );
}

#[tokio::test]
async fn pending_full_queue_preserves_original_error_and_releases_only_unprocessed_kill_claims() {
    let mut pending = reclaimable("SR-PENDING-FULL", ProcessEngineKind::SandboxContainer);
    pending.kill_succeeded_pending_stop = true;
    let unprocessed_a = reclaimable("SR-PENDING-FULL", ProcessEngineKind::HelperSubprocess);
    let unprocessed_b = reclaimable("SR-PENDING-FULL", ProcessEngineKind::PluginProcess);
    let store = Arc::new(AbortCleanupStore::new(vec![
        pending.clone(),
        unprocessed_a.clone(),
        unprocessed_b.clone(),
    ]));
    let killer = Arc::new(RecordingKill {
        killed: Mutex::new(Vec::new()),
        failures: HashSet::new(),
    });
    let (writer, _drain) = ProcessLedgerWriter::new_manual(1, Arc::new(NoopOverflowSink))
        .expect("create one-slot writer");
    writer
        .append_start(ProcessStart::new(
            ProcessEngineKind::MechanicalJob,
            "queue-filler",
            None,
        ))
        .expect("fill writer queue");
    let reclaim = Reclaim::new(Arc::clone(&store), Arc::clone(&killer), Arc::new(writer));

    let error = reclaim
        .run("SR-PENDING-FULL", ReclaimTrigger::Failure)
        .await
        .expect_err("full STOP queue remains the primary error");

    assert!(matches!(error, ProcessLedgerError::EnqueueDropped(_)));
    assert!(killer.killed().is_empty());
    assert_eq!(
        store.releases(),
        vec![
            (unprocessed_a.process_uuid, unprocessed_a.reclaim_claim),
            (unprocessed_b.process_uuid, unprocessed_b.reclaim_claim),
        ],
        "pending ownership stays retryable while every unprocessed kill claim is released"
    );
    assert!(
        !store
            .releases()
            .iter()
            .any(|(process_uuid, _)| *process_uuid == pending.process_uuid),
        "pending STOP claim must not be released by queue preflight failure"
    );

    let retry_store = Arc::new(TrackingReclaimStore::new(pending));
    let retry_killer = Arc::new(RecordingKill {
        killed: Mutex::new(Vec::new()),
        failures: HashSet::new(),
    });
    let retry_writer = Arc::new(RecordingStopWriter {
        stops: Arc::new(Mutex::new(Vec::new())),
    });
    let retry = Reclaim::new(
        retry_store,
        Arc::clone(&retry_killer),
        Arc::clone(&retry_writer),
    );
    let retry_report = retry
        .run("SR-PENDING-FULL", ReclaimTrigger::Stale)
        .await
        .expect("pending claim remains immediately recoverable");
    assert!(
        retry_killer.killed().is_empty(),
        "pending retry must not re-kill"
    );
    assert_eq!(retry_writer.stops().len(), 1);
    assert_eq!(
        retry_report.processes_reclaimed[0].kill_result,
        KillOutcome::Killed
    );
}

#[tokio::test]
async fn pending_closed_queue_preserves_original_error_and_remains_no_rekill_recoverable() {
    let mut pending = reclaimable("SR-PENDING-CLOSED", ProcessEngineKind::SandboxContainer);
    pending.kill_succeeded_pending_stop = true;
    let unprocessed = reclaimable("SR-PENDING-CLOSED", ProcessEngineKind::HelperSubprocess);
    let store = Arc::new(AbortCleanupStore::new(vec![
        pending.clone(),
        unprocessed.clone(),
    ]));
    let killer = Arc::new(RecordingKill {
        killed: Mutex::new(Vec::new()),
        failures: HashSet::new(),
    });
    let (writer, drain) = ProcessLedgerWriter::new_manual(1, Arc::new(NoopOverflowSink))
        .expect("create one-slot writer");
    drop(drain);
    let reclaim = Reclaim::new(Arc::clone(&store), Arc::clone(&killer), Arc::new(writer));

    let error = reclaim
        .run("SR-PENDING-CLOSED", ReclaimTrigger::Failure)
        .await
        .expect_err("closed STOP queue remains the primary error");
    assert!(matches!(error, ProcessLedgerError::EnqueueDropped(_)));
    assert!(killer.killed().is_empty());
    assert_eq!(
        store.releases(),
        vec![(unprocessed.process_uuid, unprocessed.reclaim_claim)]
    );
    assert!(
        !store
            .releases()
            .iter()
            .any(|(process_uuid, _)| *process_uuid == pending.process_uuid),
        "pending STOP claim must remain immediately retryable"
    );

    let retry_store = Arc::new(TrackingReclaimStore::new(pending));
    let retry_killer = Arc::new(RecordingKill {
        killed: Mutex::new(Vec::new()),
        failures: HashSet::new(),
    });
    let retry_writer = Arc::new(RecordingStopWriter {
        stops: Arc::new(Mutex::new(Vec::new())),
    });
    let retry = Reclaim::new(
        retry_store,
        Arc::clone(&retry_killer),
        Arc::clone(&retry_writer),
    );
    let retry_report = retry
        .run("SR-PENDING-CLOSED", ReclaimTrigger::Stale)
        .await
        .expect("closed-queue pending claim retry");
    assert!(retry_killer.killed().is_empty());
    assert_eq!(retry_writer.stops().len(), 1);
    assert_eq!(
        retry_report.processes_reclaimed[0].kill_result,
        KillOutcome::Killed
    );
}

#[tokio::test]
async fn post_kill_persistence_failure_is_typed_and_retry_does_not_rekill() {
    let process = reclaimable("SR-PENDING", ProcessEngineKind::SandboxContainer);
    let first_store = Arc::new(TrackingReclaimStore::new(process.clone()));
    let first_killer = Arc::new(RecordingKill {
        killed: Mutex::new(Vec::new()),
        failures: HashSet::new(),
    });
    let first = Reclaim::new(
        Arc::clone(&first_store),
        Arc::clone(&first_killer),
        Arc::new(FailingStopWriter),
    );

    let first_report = first
        .run("SR-PENDING", ReclaimTrigger::Failure)
        .await
        .expect("post-kill durability failure is reported, not lost");
    assert_eq!(first_killer.killed(), vec![process.process_uuid]);
    assert!(matches!(
        first_report.processes_reclaimed[0].kill_result,
        KillOutcome::KilledPendingStop { .. }
    ));
    assert_eq!(first_report.processes_reclaimed[0].stop_event_kind, None);
    assert_eq!(first_store.pending_marks(), 1);

    let mut pending = process.clone();
    pending.kill_succeeded_pending_stop = true;
    let retry_store = Arc::new(TrackingReclaimStore::new(pending));
    let retry_killer = Arc::new(RecordingKill {
        killed: Mutex::new(Vec::new()),
        failures: HashSet::new(),
    });
    let retry_writer = Arc::new(RecordingStopWriter {
        stops: Arc::new(Mutex::new(Vec::new())),
    });
    let retry = Reclaim::new(
        retry_store,
        Arc::clone(&retry_killer),
        Arc::clone(&retry_writer),
    );
    let retry_report = retry
        .run("SR-PENDING", ReclaimTrigger::Stale)
        .await
        .expect("pending STOP retry");

    assert!(retry_killer.killed().is_empty(), "retry must not re-kill");
    assert_eq!(retry_writer.stops().len(), 1);
    assert_eq!(
        retry_report.processes_reclaimed[0].kill_result,
        KillOutcome::Killed
    );
    assert_eq!(
        retry_report.processes_reclaimed[0].stop_event_kind,
        Some(handshake_core::process_ledger::LedgerEventKind::Stop)
    );
}

#[tokio::test]
async fn kill_failure_with_claim_release_failure_clears_process_fence_for_retry() {
    let process = reclaimable("SR-FENCE-RETRY", ProcessEngineKind::SandboxContainer);
    let store = Arc::new(ReleaseFailsOnceStore {
        process: process.clone(),
        release_attempts: Mutex::new(0),
    });
    let killer = Arc::new(FailThenSucceedKill {
        attempts: Mutex::new(Vec::new()),
    });
    let stop_writer = Arc::new(RecordingStopWriter {
        stops: Arc::new(Mutex::new(Vec::new())),
    });
    let reclaim = Reclaim::new(
        Arc::clone(&store),
        Arc::clone(&killer),
        Arc::clone(&stop_writer),
    );

    let first = reclaim
        .run("SR-FENCE-RETRY", ReclaimTrigger::Failure)
        .await
        .expect_err("first claim release failure must surface");
    assert!(first.to_string().contains("injected claim release failure"));
    assert_eq!(killer.attempts(), vec![process.process_uuid]);
    assert!(stop_writer.stops().is_empty());

    let retry = reclaim
        .run("SR-FENCE-RETRY", ReclaimTrigger::Failure)
        .await
        .expect("retry must invoke the adapter instead of replaying a completed fence failure");
    assert_eq!(
        killer.attempts(),
        vec![process.process_uuid, process.process_uuid],
        "the completed process-global failure fence must be cleared even when claim release fails"
    );
    assert!(matches!(
        retry.processes_reclaimed[0].kill_result,
        KillOutcome::Killed
    ));
    assert_eq!(stop_writer.stops().len(), 1);
}

#[tokio::test]
async fn postgres_release_failure_recovers_in_progress_state_and_retries_without_false_stop() {
    let pool = reclaim_pg_pool(4).await;
    let postgres = PostgresProcessLedgerStore::new(pool.clone());
    postgres.apply_migration().await.expect("apply migration");
    postgres
        .preflight()
        .await
        .expect("preflight PostgreSQL store");

    let session = format!("SR-PG-FENCE-RETRY-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    let start = ProcessStart::new(ProcessEngineKind::SandboxContainer, "RECLAIM_TEST", None)
        .with_process_uuid(process_uuid)
        .with_parent_session_id(session.clone())
        .with_sandbox_adapter_id("sandbox-adapter-test")
        .with_sandbox_internal_id("sandbox-internal-test");
    postgres
        .write_batch(vec![LedgerEvent::Start(start)])
        .await
        .expect("seed PostgreSQL lifecycle");

    let writer_store: Arc<dyn ProcessLedgerStore> =
        Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    let (writer, writer_join) = ProcessLedgerWriter::spawn(
        writer_store,
        Arc::new(NoopOverflowSink),
        WriterConfig {
            capacity: 8,
            batch_size: 1,
            flush_interval: Duration::from_millis(5),
        },
    );
    let writer = Arc::new(writer);
    let store = Arc::new(PostgresReleaseFailsOnceStore {
        inner: postgres,
        fail_release: AtomicBool::new(true),
    });
    let killer = Arc::new(FailThenSucceedKill {
        attempts: Mutex::new(Vec::new()),
    });
    let reclaim = Reclaim::new(Arc::clone(&store), Arc::clone(&killer), Arc::clone(&writer));

    let first = reclaim
        .run(&session, ReclaimTrigger::Failure)
        .await
        .expect_err("the injected PostgreSQL release failure must surface");
    assert!(first
        .to_string()
        .contains("injected PostgreSQL claim-release transport failure"));
    let before_recovery: (Option<chrono::DateTime<Utc>>, Option<String>) = sqlx::query_as(
        "SELECT stopped_at, stop_reason FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(process_uuid)
    .fetch_one(&pool)
    .await
    .expect("read crash-left PostgreSQL state");
    assert!(
        before_recovery.0.is_none(),
        "failed kill must not write STOP"
    );
    assert_eq!(
        before_recovery.1.as_deref(),
        Some("reclaim_kill_in_progress")
    );

    let sweep = reclaim
        .reconcile_in_progress_for_session(&session)
        .await
        .expect("recover NotStarted operation and retry");
    assert_eq!(sweep.operations.len(), 1);
    assert!(sweep.reclaim_error.is_none(), "{sweep:?}");
    let retry_report = sweep
        .reclaim_report
        .expect("NotStarted recovery must immediately retry the durable open row");
    assert!(matches!(
        retry_report.processes_reclaimed[0].kill_result,
        KillOutcome::Killed
    ));
    assert_eq!(
        killer.attempts(),
        vec![process_uuid, process_uuid],
        "recovery must invoke the adapter again rather than replaying the cleared in-memory failure fence"
    );
    let after_recovery: (Option<chrono::DateTime<Utc>>, Option<String>, i64) = sqlx::query_as(
        "SELECT stopped_at, stop_reason, COUNT(*) OVER () FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(process_uuid)
    .fetch_one(&pool)
    .await
    .expect("read recovered PostgreSQL STOP");
    assert!(after_recovery.0.is_some());
    assert_eq!(after_recovery.1.as_deref(), Some("reclaim"));
    assert_eq!(after_recovery.2, 1, "exactly one lifecycle row may exist");

    drop(reclaim);
    drop(writer);
    writer_join
        .await
        .expect("PostgreSQL writer task joins")
        .expect("PostgreSQL writer drains");
}

#[cfg(feature = "test-utils")]
#[tokio::test]
async fn slow_kill_renews_claim_before_stop() {
    let process = reclaimable("SR-SLOW", ProcessEngineKind::SandboxContainer);
    let store = Arc::new(TrackingReclaimStore::new(process.clone()));
    let killer = Arc::new(SlowKill {
        delay: Duration::from_millis(75),
        killed: Mutex::new(Vec::new()),
    });
    let stop_writer = Arc::new(RecordingStopWriter {
        stops: Arc::new(Mutex::new(Vec::new())),
    });
    let reclaim = Reclaim::new(Arc::clone(&store), killer, stop_writer)
        .with_reclaim_timings_for_test(Duration::from_millis(10), Duration::from_secs(1));

    let report = reclaim
        .run("SR-SLOW", ReclaimTrigger::Close)
        .await
        .expect("slow kill reclaim");

    assert_eq!(
        report.processes_reclaimed[0].kill_result,
        KillOutcome::Killed
    );
    assert!(
        store.renewals() >= 2,
        "slow termination must renew its fenced claim while kill is running"
    );
}

#[cfg(feature = "test-utils")]
#[tokio::test]
async fn unresponsive_kill_is_bounded_and_never_writes_false_stop() {
    let process = reclaimable("SR-KILL-TIMEOUT", ProcessEngineKind::SandboxContainer);
    let store = Arc::new(TrackingReclaimStore::new(process));
    let killer = Arc::new(SlowKill {
        delay: Duration::from_secs(5),
        killed: Mutex::new(Vec::new()),
    });
    let stop_writer = Arc::new(RecordingStopWriter {
        stops: Arc::new(Mutex::new(Vec::new())),
    });
    let reclaim = Reclaim::new(
        Arc::clone(&store),
        Arc::clone(&killer),
        Arc::clone(&stop_writer),
    )
    .with_reclaim_timings_for_test(Duration::from_millis(5), Duration::from_secs(1))
    .with_kill_timeout_for_test(Duration::from_millis(25));

    let started = std::time::Instant::now();
    let report = reclaim
        .run("SR-KILL-TIMEOUT", ReclaimTrigger::Close)
        .await
        .expect("kill timeout is a typed reclaim outcome");

    assert!(
        started.elapsed() < Duration::from_secs(1),
        "unresponsive adapter kill must remain globally bounded"
    );
    assert!(matches!(
        &report.processes_reclaimed[0].kill_result,
        KillOutcome::Failed { error } if error.contains("exceeded 25ms")
    ));
    assert_eq!(report.processes_reclaimed[0].stop_event_kind, None);
    assert!(
        stop_writer.stops().is_empty(),
        "failed kill cannot emit STOP"
    );
    assert!(
        killer.killed().is_empty(),
        "aborted async kill must not later report completion"
    );
}

#[cfg(feature = "test-utils")]
#[tokio::test]
async fn ownership_loss_during_slow_kill_cannot_report_killed() {
    let process = reclaimable("SR-TAKEOVER", ProcessEngineKind::SandboxContainer);
    let store = Arc::new(LostOwnershipStore::new(process.clone()));
    let killer = Arc::new(SlowKill {
        delay: Duration::from_millis(40),
        killed: Mutex::new(Vec::new()),
    });
    let stop_writer = Arc::new(RecordingStopWriter {
        stops: Arc::new(Mutex::new(Vec::new())),
    });
    let reclaim = Reclaim::new(store, killer, Arc::clone(&stop_writer))
        .with_reclaim_timings_for_test(Duration::from_millis(5), Duration::from_millis(25));

    let report = reclaim
        .run("SR-TAKEOVER", ReclaimTrigger::Close)
        .await
        .expect("ownership loss is a typed post-kill result");

    let KillOutcome::KilledPendingStop { error } = &report.processes_reclaimed[0].kill_result
    else {
        panic!("ownership loss must not report ordinary Killed");
    };
    assert!(error.contains("claim renewal failed"));
    assert!(error.contains("pending-stop marker failed"));
    assert!(error.contains("STOP was durable but reclaim ownership continuity was not proven"));
    assert_eq!(
        stop_writer.stops().len(),
        1,
        "the best-effort STOP stays durable"
    );
    assert_eq!(report.processes_reclaimed[0].stop_event_kind, None);
}

#[tokio::test]
async fn stale_claimant_stop_cannot_poison_new_claimants_valid_stop() {
    let mut stale_process = reclaimable("SR-STOP-FENCE", ProcessEngineKind::SandboxContainer);
    stale_process.kill_succeeded_pending_stop = true;
    stale_process.metadata_jsonb["reclaim_pending_stop"] = serde_json::json!({
        "claimant_uuid": stale_process.reclaim_claim.claimant_uuid,
        "generation": stale_process.reclaim_claim.generation,
        "exit_code": -1,
        "stop_reason": "reclaim",
    });
    let mut current_process = stale_process.clone();
    current_process.reclaim_claim = ReclaimClaim {
        claimant_uuid: Uuid::now_v7(),
        kill_operation_uuid: stale_process.reclaim_claim.kill_operation_uuid,
        generation: stale_process.reclaim_claim.generation + 1,
        claimed_at_unix_ms: Utc::now().timestamp_millis(),
        lease_expires_at_unix_ms: Utc::now().timestamp_millis() + 30_000,
    };
    current_process.metadata_jsonb["reclaim_claim"] =
        serde_json::to_value(&current_process.reclaim_claim).expect("serialize current claim");

    let store = Arc::new(FencedStopStore::new(current_process.reclaim_claim.clone()));
    let config = WriterConfig {
        capacity: 4,
        batch_size: 4,
        flush_interval: Duration::from_millis(5),
    };
    let (writer, join) = ProcessLedgerWriter::spawn(
        Arc::clone(&store) as Arc<dyn ProcessLedgerStore>,
        Arc::new(NoopOverflowSink),
        config,
    );
    let stale_reservation = writer
        .try_reserve_reclaim_stop()
        .expect("reserve stale claimant STOP");
    let current_reservation = writer
        .try_reserve_reclaim_stop()
        .expect("reserve current claimant STOP");
    let stale_stop = stale_process.reclaim_stop(-1);
    let current_stop = current_process.reclaim_stop(-1);
    assert_eq!(
        current_stop.metadata_jsonb["reclaim_pending_stop"]["claimant_uuid"],
        current_process.reclaim_claim.claimant_uuid.to_string(),
        "takeover STOP must replace A's pending marker with B's fenced metadata"
    );
    assert!(
        PROCESS_STOP_UPSERT_SQL.contains("metadata_jsonb = EXCLUDED.metadata_jsonb"),
        "fenced final STOP must replace the old pending metadata for exact readback"
    );
    let stale_ack = stale_reservation
        .commit_with_durable_ack(stale_stop)
        .expect("queue stale STOP");
    let current_ack = current_reservation
        .commit_with_durable_ack(current_stop)
        .expect("queue current STOP behind stale STOP");

    let stale_error = stale_ack
        .wait(Duration::from_secs(1))
        .await
        .expect_err("stale STOP must be permanently rejected");
    assert!(matches!(
        stale_error,
        ProcessLedgerError::DurabilityRejected { .. }
    ));
    current_ack
        .wait(Duration::from_secs(1))
        .await
        .expect("current claimant STOP must persist after stale STOP removal");

    assert_eq!(
        store.persisted_claims(),
        vec![current_process.reclaim_claim]
    );
    drop(writer);
    join.await
        .expect("writer task joins")
        .expect("writer drains after permanent stale STOP rejection");
}

#[cfg(feature = "test-utils")]
#[tokio::test]
async fn blocked_beyond_expired_lease_shared_authority_prevents_takeover_kill() {
    let mut process = reclaimable("SR-BLOCKED", ProcessEngineKind::SandboxContainer);
    process.reclaim_claim.lease_expires_at_unix_ms = Utc::now().timestamp_millis() + 20;
    process.metadata_jsonb["reclaim_claim"] =
        serde_json::to_value(&process.reclaim_claim).expect("serialize short claim");
    let store = Arc::new(DurableKillFenceStore::new(process.clone()));
    let killer = Arc::new(SlowKill {
        delay: Duration::from_millis(100),
        killed: Mutex::new(Vec::new()),
    });
    let stop_writer = Arc::new(FailingStopWriter);
    let reclaim_a = Arc::new(
        Reclaim::new(
            Arc::clone(&store),
            Arc::clone(&killer),
            Arc::clone(&stop_writer),
        )
        .with_reclaim_timings_for_test(Duration::from_millis(5), Duration::from_secs(1)),
    );
    let reclaim_b = Arc::new(
        Reclaim::new(
            Arc::clone(&store),
            Arc::clone(&killer),
            Arc::clone(&stop_writer),
        )
        .with_reclaim_timings_for_test(Duration::from_millis(5), Duration::from_secs(1)),
    );

    let a = tokio::spawn(async move { reclaim_a.run("SR-BLOCKED", ReclaimTrigger::Close).await });
    store.wait_until_kill_started().await;
    sleep(Duration::from_millis(35)).await;
    assert!(
        Utc::now().timestamp_millis() > process.reclaim_claim.lease_expires_at_unix_ms,
        "takeover query must occur after the original lease expired"
    );
    let b_report = reclaim_b
        .run("SR-BLOCKED", ReclaimTrigger::Stale)
        .await
        .expect("second reclaimer performs a real post-expiry authority query");
    let a_report = a.await.unwrap().expect("original reclaimer completes");

    assert!(
        b_report.processes_reclaimed.is_empty(),
        "durable kill-in-progress phase must exclude takeover after lease expiry"
    );
    assert_eq!(
        store.query_count(),
        2,
        "A and B must both query the same authoritative store"
    );
    assert_eq!(
        killer.killed(),
        vec![process.process_uuid],
        "two reclaimers sharing one authority must issue exactly one kill"
    );
    assert!(matches!(
        a_report.processes_reclaimed[0].kill_result,
        KillOutcome::KilledPendingStop { .. }
    ));
    assert_eq!(a_report.processes_reclaimed[0].stop_event_kind, None);
    assert_eq!(
        store.phase(),
        "kill_succeeded_pending_stop",
        "failed STOP persistence must leave one truthful durable pending state"
    );
}

#[tokio::test]
async fn recovery_sweep_continues_after_independent_status_failures() {
    let query_failed = reclaimable("SR-RECOVERY-MIXED", ProcessEngineKind::SandboxContainer);
    let transition_failed = reclaimable("SR-RECOVERY-MIXED", ProcessEngineKind::LlamaCpp);
    let succeeded = reclaimable("SR-RECOVERY-MIXED", ProcessEngineKind::HelperSubprocess);
    let not_started = reclaimable("SR-RECOVERY-MIXED", ProcessEngineKind::PluginProcess);
    let failed = reclaimable("SR-RECOVERY-MIXED", ProcessEngineKind::MechanicalJob);
    let unknown = reclaimable("SR-RECOVERY-MIXED", ProcessEngineKind::ExternalCompat);
    let in_progress = reclaimable("SR-RECOVERY-MIXED", ProcessEngineKind::SandboxContainer);
    let processes = vec![
        query_failed.clone(),
        transition_failed.clone(),
        succeeded.clone(),
        not_started.clone(),
        failed.clone(),
        unknown.clone(),
        in_progress.clone(),
    ];
    let store = Arc::new(
        RecoverySweepStore::new(processes).with_transition_failure(transition_failed.process_uuid),
    );
    let killer = Arc::new(StatusKill::new(HashMap::from([
        (
            query_failed.process_uuid,
            Err("adapter status endpoint unavailable".to_string()),
        ),
        (
            transition_failed.process_uuid,
            Ok(ReclaimKillOperationStatus::Succeeded),
        ),
        (
            succeeded.process_uuid,
            Ok(ReclaimKillOperationStatus::Succeeded),
        ),
        (
            not_started.process_uuid,
            Ok(ReclaimKillOperationStatus::NotStarted),
        ),
        (failed.process_uuid, Ok(ReclaimKillOperationStatus::Failed)),
        (
            unknown.process_uuid,
            Ok(ReclaimKillOperationStatus::Unknown),
        ),
        (
            in_progress.process_uuid,
            Ok(ReclaimKillOperationStatus::InProgress),
        ),
    ])));
    let stop_writer = Arc::new(RecordingStopWriter {
        stops: Arc::new(Mutex::new(Vec::new())),
    });
    let reclaim = Reclaim::new(
        Arc::clone(&store),
        Arc::clone(&killer),
        Arc::clone(&stop_writer),
    );

    let sweep = reclaim
        .reconcile_in_progress_for_session("SR-RECOVERY-MIXED")
        .await
        .expect("mixed recovery sweep");

    assert_eq!(sweep.operations.len(), 7);
    assert!(sweep.reclaim_error.is_none());
    let outcomes: HashMap<Uuid, ReclaimKillOperationSweepOutcome> = sweep
        .operations
        .into_iter()
        .filter_map(|entry| match entry.candidate {
            ReclaimKillOperationCandidate::Operation { operation } => {
                Some((operation.process_uuid, entry.outcome))
            }
            ReclaimKillOperationCandidate::Malformed { .. } => None,
        })
        .collect();
    assert!(matches!(
        outcomes[&query_failed.process_uuid],
        ReclaimKillOperationSweepOutcome::StatusQueryFailed { .. }
    ));
    assert!(matches!(
        outcomes[&transition_failed.process_uuid],
        ReclaimKillOperationSweepOutcome::StateTransitionFailed {
            status: ReclaimKillOperationStatus::Succeeded,
            ..
        }
    ));
    assert_eq!(
        outcomes[&succeeded.process_uuid],
        ReclaimKillOperationSweepOutcome::StateAdvanced {
            status: ReclaimKillOperationStatus::Succeeded,
        }
    );
    assert_eq!(
        outcomes[&not_started.process_uuid],
        ReclaimKillOperationSweepOutcome::StateAdvanced {
            status: ReclaimKillOperationStatus::NotStarted,
        }
    );
    assert_eq!(
        outcomes[&failed.process_uuid],
        ReclaimKillOperationSweepOutcome::StateAdvanced {
            status: ReclaimKillOperationStatus::Failed,
        }
    );
    assert_eq!(
        outcomes[&unknown.process_uuid],
        ReclaimKillOperationSweepOutcome::StateOpen {
            status: ReclaimKillOperationStatus::Unknown,
        }
    );
    assert_eq!(
        outcomes[&in_progress.process_uuid],
        ReclaimKillOperationSweepOutcome::StateOpen {
            status: ReclaimKillOperationStatus::InProgress,
        }
    );
    let report = sweep
        .reclaim_report
        .expect("terminal evidence must trigger normal reclaim finalization");
    assert_eq!(report.processes_reclaimed.len(), 3);
    assert_eq!(stop_writer.stops().len(), 3);
    let killed = killer.killed();
    assert_eq!(killed.len(), 2, "succeeded recovery must not re-kill");
    assert!(killed.contains(&(
        not_started.process_uuid,
        not_started.reclaim_claim.kill_operation_uuid
    )));
    assert!(killed.contains(&(
        failed.process_uuid,
        failed.reclaim_claim.kill_operation_uuid
    )));
    assert_eq!(
        store.phase(query_failed.process_uuid),
        "reclaim_kill_in_progress"
    );
    assert_eq!(
        store.phase(transition_failed.process_uuid),
        "reclaim_kill_in_progress"
    );
    assert_eq!(
        store.phase(unknown.process_uuid),
        "reclaim_kill_in_progress"
    );
    assert_eq!(
        store.phase(in_progress.process_uuid),
        "reclaim_kill_in_progress"
    );
    assert_eq!(
        store.last_limit(),
        64,
        "the recovery query must stay bounded"
    );
}

#[test]
fn postgres_reclaim_query_uses_row_lock_and_open_process_filter() {
    let sql = POSTGRES_ACTIVE_RECLAIM_QUERY_SQL;
    assert!(sql.contains("FROM kernel_process_lifecycle"));
    assert!(sql.contains("parent_session_id = $1"));
    assert!(sql.contains("stopped_at IS NULL"));
    assert!(sql.contains("FOR UPDATE"));
    assert!(!sql.to_ascii_lowercase().contains("sqlite"));
}

// MT-008: the FOR UPDATE row lock must cover the rows being reclaimed for the
// duration of the reclaim decision. The fix collapses the read-modify-write into
// a single atomic `UPDATE ... RETURNING` guarded by `FOR UPDATE`, so a concurrent
// reclaim sees the rows already carrying a fresh reclaim marker and cannot
// double-act. The lifecycle row remains open until a successful kill produces
// the authoritative STOP event. This test asserts that atomic claim shape.
#[test]
fn postgres_reclaim_query_atomically_claims_rows_under_lock() {
    let sql = POSTGRES_ACTIVE_RECLAIM_QUERY_SQL.to_ascii_lowercase();
    // Lock the candidate rows...
    assert!(sql.contains("for update"), "must take row locks");
    // ...and write a recoverable claim in the SAME statement so the claim is atomic.
    assert!(
        sql.contains("update kernel_process_lifecycle"),
        "claim must be an UPDATE, not a bare SELECT that releases the lock on commit"
    );
    assert!(
        sql.contains("'reclaim_claimed'")
            && sql.contains("claimant_uuid")
            && sql.contains("kill_operation_uuid")
            && sql.contains("generation")
            && sql.contains("lease_expires_at_unix_ms"),
        "claim must remain open while carrying a UUID-plus-generation fenced lease"
    );
    assert!(
        !sql.contains("set stopped_at"),
        "claim must not fabricate a terminal lifecycle before kill succeeds"
    );
    assert!(
        sql.contains("returning"),
        "claimed rows must be RETURNING-ed so the caller acts on exactly what it claimed"
    );
    // The candidate filter still only targets un-stopped rows.
    assert!(sql.contains("stopped_at is null"));
    assert!(
        sql.contains("stop_reason not in ('reclaim_claimed', 'reclaim_kill_in_progress')"),
        "a durable kill-in-progress row must remain ineligible after lease expiry"
    );
}

#[test]
fn reclaim_stop_finalize_is_open_row_and_token_fenced() {
    let sql = PROCESS_STOP_UPSERT_SQL.to_ascii_lowercase();
    assert!(
        sql.contains("kernel_process_lifecycle.stopped_at is null"),
        "reclaim STOP must close an open claimed row"
    );
    assert!(sql.contains("'reclaim_claimed'"));
    assert!(sql.contains("'kill_succeeded_pending_stop'"));
    assert!(
        sql.contains("metadata_jsonb->'reclaim_claim'->>'claimant_uuid'")
            && sql.contains("metadata_jsonb->'reclaim_claim'->>'kill_operation_uuid'")
            && sql.contains("metadata_jsonb->'reclaim_claim'->>'generation'"),
        "reclaim STOP finalization must match claimant UUID, kill-operation UUID, and generation"
    );
}

// MT-008 (logic-level serialization proof): model the atomic-claim semantics with
// an in-memory store that drains each session's active rows under a lock on the
// FIRST call (mirroring the Postgres UPDATE...RETURNING claim), and prove that two
// concurrent reclaims of the same session reclaim each process exactly once: no
// double-reclaim, no missed row.
#[tokio::test]
async fn concurrent_reclaims_claim_each_process_exactly_once() {
    let processes = vec![
        reclaimable("SR-RACE", ProcessEngineKind::SandboxContainer),
        reclaimable("SR-RACE", ProcessEngineKind::HelperSubprocess),
        reclaimable("SR-RACE", ProcessEngineKind::PluginProcess),
    ];
    let expected: HashSet<Uuid> = processes.iter().map(|p| p.process_uuid).collect();

    let store = Arc::new(ClaimingReclaimStore::new(HashMap::from([(
        "SR-RACE".to_string(),
        processes,
    )])));
    let killer = Arc::new(RecordingKill {
        killed: Mutex::new(Vec::new()),
        failures: HashSet::new(),
    });
    let stop_writer = Arc::new(RecordingStopWriter {
        stops: Arc::new(Mutex::new(Vec::new())),
    });
    let reclaim = Arc::new(Reclaim::new(
        Arc::clone(&store),
        Arc::clone(&killer),
        Arc::clone(&stop_writer),
    ));

    // Fire two reclaims of the same session concurrently.
    let r1 = {
        let reclaim = Arc::clone(&reclaim);
        tokio::spawn(async move { reclaim.run("SR-RACE", ReclaimTrigger::Close).await })
    };
    let r2 = {
        let reclaim = Arc::clone(&reclaim);
        tokio::spawn(async move { reclaim.run("SR-RACE", ReclaimTrigger::Close).await })
    };
    let report1 = r1.await.unwrap().expect("reclaim 1");
    let report2 = r2.await.unwrap().expect("reclaim 2");

    // Across both reclaims, each process was claimed exactly once: the total
    // reclaimed equals the active set, with no duplicates (no double-reclaim).
    let mut all: Vec<Uuid> = Vec::new();
    all.extend(report1.processes_reclaimed.iter().map(|p| p.process_uuid));
    all.extend(report2.processes_reclaimed.iter().map(|p| p.process_uuid));

    let unique: HashSet<Uuid> = all.iter().copied().collect();
    assert_eq!(
        all.len(),
        unique.len(),
        "no process may be reclaimed twice (double-reclaim)"
    );
    assert_eq!(
        unique, expected,
        "every active process must be reclaimed once (no missed row)"
    );

    // Stop events: exactly one per process, no duplicates.
    let stops = stop_writer.stops();
    let stop_ids: HashSet<Uuid> = stops.iter().map(|s| s.process_uuid).collect();
    assert_eq!(stops.len(), expected.len(), "exactly one stop per process");
    assert_eq!(stop_ids, expected);

    // The killer fired exactly once per process.
    let killed = killer.killed();
    let killed_ids: HashSet<Uuid> = killed.iter().copied().collect();
    assert_eq!(killed.len(), expected.len(), "exactly one kill per process");
    assert_eq!(killed_ids, expected);
}

// MT-008 (Postgres-gated): exercises the real atomic-claim SQL against a
// task-owned managed PostgreSQL database. The shared helper creates one
// isolated migrated database per test and joins identity-gated cluster cleanup
// at normal process exit.
#[tokio::test]
async fn postgres_failed_kill_releases_open_reclaim_claim_without_stop() {
    use handshake_core::process_ledger::{
        LedgerEvent, PostgresProcessLedgerStore, ProcessLedgerStore, ProcessStart,
    };
    let pool = reclaim_pg_pool(4).await;
    let store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    store.apply_migration().await.expect("apply migration");

    let session = format!("SR-PG-KILL-FAIL-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    let start = ProcessStart::new(ProcessEngineKind::SandboxContainer, "RECLAIM_TEST", None)
        .with_process_uuid(process_uuid)
        .with_parent_session_id(session.clone())
        .with_sandbox_adapter_id("sandbox-adapter-test")
        .with_sandbox_internal_id("sandbox-internal-test");
    store
        .write_batch(vec![LedgerEvent::Start(start)])
        .await
        .expect("seed open process lifecycle");

    let killer = Arc::new(RecordingKill {
        killed: Mutex::new(Vec::new()),
        failures: HashSet::from([process_uuid]),
    });
    let stop_writer = Arc::new(RecordingStopWriter {
        stops: Arc::new(Mutex::new(Vec::new())),
    });
    let reclaim = Reclaim::new(Arc::clone(&store), killer, Arc::clone(&stop_writer));
    let report = reclaim
        .run(&session, ReclaimTrigger::Failure)
        .await
        .expect("failed kill releases its claim");

    assert_eq!(report.processes_reclaimed.len(), 1);
    assert!(matches!(
        report.processes_reclaimed[0].kill_result,
        KillOutcome::Failed { .. }
    ));
    assert_eq!(report.processes_reclaimed[0].stop_event_kind, None);
    assert!(stop_writer.stops().is_empty());

    let row: (
        Option<chrono::DateTime<Utc>>,
        Option<String>,
        serde_json::Value,
    ) = sqlx::query_as(
        "SELECT stopped_at, stop_reason, metadata_jsonb FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(process_uuid)
    .fetch_one(&pool)
    .await
    .expect("read lifecycle after failed kill");
    assert_eq!(row.0, None, "failed kill must leave lifecycle open");
    assert_eq!(row.1, None, "failed kill must release the reclaim marker");
    assert!(
        row.2.get("reclaim_claim").is_none(),
        "failed kill must remove reclaim claim metadata"
    );
}

#[tokio::test]
async fn postgres_crash_after_kill_recovery_finalizes_stop_without_rekill() {
    use handshake_core::process_ledger::{
        LedgerEvent, PostgresProcessLedgerStore, ProcessLedgerStore, ProcessStart,
    };
    let pool = reclaim_pg_pool(4).await;
    let store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    store.apply_migration().await.expect("apply migration");

    let session = format!("SR-PG-KILL-RECOVERY-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    let start = ProcessStart::new(ProcessEngineKind::SandboxContainer, "RECLAIM_TEST", None)
        .with_process_uuid(process_uuid)
        .with_parent_session_id(session.clone())
        .with_sandbox_adapter_id("sandbox-adapter-test")
        .with_sandbox_internal_id("sandbox-internal-recovery-test");
    store
        .write_batch(vec![LedgerEvent::Start(start)])
        .await
        .expect("seed open process lifecycle");
    let claimed = store
        .active_processes_for_session(&session)
        .await
        .expect("claim lifecycle before simulated crash");
    let process = claimed.first().expect("one claimed lifecycle").clone();
    store
        .mark_reclaim_kill_started(process_uuid, &process.reclaim_claim)
        .await
        .expect("persist kill-in-progress before simulated crash");

    let killer = Arc::new(StatusKill::new(HashMap::from([(
        process_uuid,
        Ok(ReclaimKillOperationStatus::Succeeded),
    )])));
    let config = WriterConfig {
        capacity: 4,
        batch_size: 1,
        flush_interval: Duration::from_millis(5),
    };
    let (writer, join) = ProcessLedgerWriter::spawn(
        Arc::clone(&store) as Arc<dyn ProcessLedgerStore>,
        Arc::new(NoopOverflowSink),
        config,
    );
    let writer = Arc::new(writer);
    let reclaim = Reclaim::new(Arc::clone(&store), Arc::clone(&killer), Arc::clone(&writer));
    let sweep = reclaim
        .reconcile_in_progress_for_session(&session)
        .await
        .expect("recover crash-left kill operation");

    assert_eq!(sweep.operations.len(), 1);
    assert!(matches!(
        &sweep.operations[0].outcome,
        ReclaimKillOperationSweepOutcome::StateAdvanced {
            status: ReclaimKillOperationStatus::Succeeded
        }
    ));
    assert!(sweep.reclaim_error.is_none());
    assert!(sweep.reclaim_report.is_some());
    assert!(
        killer.killed().is_empty(),
        "succeeded recovery must not re-kill"
    );
    let row: (Option<chrono::DateTime<Utc>>, Option<String>, serde_json::Value) =
        sqlx::query_as(
            "SELECT stopped_at, stop_reason, metadata_jsonb FROM kernel_process_lifecycle WHERE process_uuid = $1",
        )
        .bind(process_uuid)
        .fetch_one(&pool)
        .await
        .expect("read lifecycle after recovered STOP");
    assert!(row.0.is_some(), "recovered STOP must close the lifecycle");
    assert_eq!(row.1.as_deref(), Some("reclaim"));
    assert_eq!(
        row.2["reclaim_last_kill_operation"]["kill_operation_uuid"],
        process.reclaim_claim.kill_operation_uuid.to_string()
    );
    assert_eq!(row.2["reclaim_last_kill_operation"]["status"], "succeeded");

    drop(reclaim);
    drop(writer);
    join.await
        .expect("writer task joins")
        .expect("writer drains recovered STOP");
}

#[tokio::test]
async fn postgres_recovery_scan_continues_after_first_malformed_operation_id() {
    use handshake_core::process_ledger::{
        LedgerEvent, PostgresProcessLedgerStore, ProcessLedgerStore, ProcessStart,
    };
    let pool = reclaim_pg_pool(4).await;
    let store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    store.apply_migration().await.expect("apply migration");

    let session = format!("SR-PG-MALFORMED-RECOVERY-{}", Uuid::now_v7());
    let malformed_uuid = Uuid::now_v7();
    let succeeded_uuid = Uuid::now_v7();
    let not_started_uuid = Uuid::now_v7();
    let base_started_at = Utc::now();
    let starts = [
        (malformed_uuid, 0_i64),
        (succeeded_uuid, 1_i64),
        (not_started_uuid, 2_i64),
    ]
    .into_iter()
    .map(|(process_uuid, offset_ms)| {
        let mut start =
            ProcessStart::new(ProcessEngineKind::SandboxContainer, "RECLAIM_TEST", None)
                .with_process_uuid(process_uuid)
                .with_parent_session_id(session.clone());
        start.started_at = base_started_at + chrono::Duration::milliseconds(offset_ms);
        LedgerEvent::Start(start)
    })
    .collect();
    store
        .write_batch(starts)
        .await
        .expect("seed ordered open process lifecycles");
    let claimed = store
        .active_processes_for_session(&session)
        .await
        .expect("claim lifecycles before malformed metadata probe");
    assert_eq!(claimed.len(), 3);
    let claims: HashMap<Uuid, ReclaimClaim> = claimed
        .iter()
        .map(|process| (process.process_uuid, process.reclaim_claim.clone()))
        .collect();
    for process in &claimed {
        store
            .mark_reclaim_kill_started(process.process_uuid, &process.reclaim_claim)
            .await
            .expect("persist kill-in-progress before malformed metadata probe");
    }
    sqlx::query(
        "UPDATE kernel_process_lifecycle SET metadata_jsonb = metadata_jsonb - 'reclaim_last_kill_operation' WHERE process_uuid = $1",
    )
    .bind(malformed_uuid)
    .execute(&pool)
    .await
    .expect("remove operation evidence for fail-closed probe");

    let killer = Arc::new(StatusKill::new(HashMap::from([
        (succeeded_uuid, Ok(ReclaimKillOperationStatus::Succeeded)),
        (not_started_uuid, Ok(ReclaimKillOperationStatus::NotStarted)),
    ])));
    let config = WriterConfig {
        capacity: 4,
        batch_size: 1,
        flush_interval: Duration::from_millis(5),
    };
    let (writer, join) = ProcessLedgerWriter::spawn(
        Arc::clone(&store) as Arc<dyn ProcessLedgerStore>,
        Arc::new(NoopOverflowSink),
        config,
    );
    let writer = Arc::new(writer);
    let reclaim = Reclaim::new(Arc::clone(&store), Arc::clone(&killer), Arc::clone(&writer));
    let sweep = reclaim
        .reconcile_in_progress_for_session(&session)
        .await
        .expect("malformed row must not abort later independent recovery rows");
    assert_eq!(sweep.operations.len(), 3);
    assert!(matches!(
        &sweep.operations[0],
        handshake_core::process_ledger::ReclaimKillOperationSweepEntry {
            candidate: ReclaimKillOperationCandidate::Malformed {
                process_identity,
                kill_operation_identity: None,
                ..
            },
            outcome: ReclaimKillOperationSweepOutcome::MalformedRecoveryRow { .. },
        } if process_identity == &malformed_uuid.to_string()
    ));
    assert!(matches!(
        &sweep.operations[1].outcome,
        ReclaimKillOperationSweepOutcome::StateAdvanced {
            status: ReclaimKillOperationStatus::Succeeded
        }
    ));
    assert!(matches!(
        &sweep.operations[2].outcome,
        ReclaimKillOperationSweepOutcome::StateAdvanced {
            status: ReclaimKillOperationStatus::NotStarted
        }
    ));
    assert!(sweep.reclaim_error.is_none());
    assert_eq!(
        killer.killed(),
        vec![(
            not_started_uuid,
            claims[&not_started_uuid].kill_operation_uuid
        )],
        "not-started retries the exact stable operation; succeeded never re-kills"
    );
    let rows: Vec<(Uuid, Option<chrono::DateTime<Utc>>, Option<String>)> = sqlx::query_as(
        "SELECT process_uuid, stopped_at, stop_reason FROM kernel_process_lifecycle WHERE process_uuid = ANY($1) ORDER BY started_at, process_uuid",
    )
    .bind(vec![malformed_uuid, succeeded_uuid, not_started_uuid])
    .fetch_all(&pool)
    .await
    .expect("read recovery rows after mixed sweep");
    assert_eq!(
        rows[0],
        (
            malformed_uuid,
            None,
            Some("reclaim_kill_in_progress".to_string())
        ),
        "malformed recovery scan must not close the row"
    );
    assert!(rows[1].1.is_some(), "later succeeded row must close");
    assert_eq!(rows[1].2.as_deref(), Some("reclaim"));
    assert!(rows[2].1.is_some(), "later not-started row must close");
    assert_eq!(rows[2].2.as_deref(), Some("reclaim"));

    drop(reclaim);
    drop(writer);
    join.await
        .expect("writer task joins")
        .expect("writer drains mixed recovery STOPs");
}

#[tokio::test]
async fn postgres_concurrent_reclaim_claims_each_row_once() {
    use handshake_core::process_ledger::{
        LedgerEvent, PostgresProcessLedgerStore, ProcessLedgerStore, ProcessStart,
    };
    let pool = reclaim_pg_pool(8).await;
    let store = Arc::new(PostgresProcessLedgerStore::new(pool));
    store.apply_migration().await.expect("apply migration");

    let session = format!("SR-PG-RACE-{}", Uuid::now_v7());
    let expected = [Uuid::now_v7(), Uuid::now_v7()]
        .into_iter()
        .collect::<HashSet<_>>();
    let starts = expected
        .iter()
        .enumerate()
        .map(|(index, process_uuid)| {
            LedgerEvent::Start(
                ProcessStart::new(
                    ProcessEngineKind::SandboxContainer,
                    "RECLAIM_RACE_TEST",
                    None,
                )
                .with_process_uuid(*process_uuid)
                .with_parent_session_id(session.clone())
                .with_sandbox_adapter_id("sandbox-adapter-test")
                .with_sandbox_internal_id(format!("sandbox-race-{index}")),
            )
        })
        .collect();
    store
        .write_batch(starts)
        .await
        .expect("seed concurrent reclaim rows");

    let a = Arc::clone(&store);
    let b = Arc::clone(&store);
    let sa = session.clone();
    let sb = session.clone();
    let ra = tokio::spawn(async move { a.active_processes_for_session(&sa).await });
    let rb = tokio::spawn(async move { b.active_processes_for_session(&sb).await });
    let claimed_a = ra.await.unwrap().expect("claim a");
    let claimed_b = rb.await.unwrap().expect("claim b");

    let mut ids: HashSet<Uuid> = HashSet::new();
    for p in claimed_a.iter().chain(claimed_b.iter()) {
        assert!(
            ids.insert(p.process_uuid),
            "process {} was claimed by both concurrent reclaims (double-claim)",
            p.process_uuid
        );
    }
    assert_eq!(
        ids, expected,
        "concurrent claims must cover every seeded open process exactly once"
    );
    for process in claimed_a.iter().chain(claimed_b.iter()) {
        store
            .release_reclaim_claim(process.process_uuid, &process.reclaim_claim)
            .await
            .expect("release concurrency-test reclaim claim");
    }
}

#[tokio::test]
async fn postgres_stale_claimant_cannot_release_or_finalize_newer_claim() {
    use handshake_core::process_ledger::{
        LedgerEvent, PostgresProcessLedgerStore, ProcessLedgerStore, ProcessStart,
    };
    let pool = reclaim_pg_pool(4).await;
    let store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    store.apply_migration().await.expect("apply migration");

    let session = format!("SR-PG-STALE-TOKEN-{}", Uuid::now_v7());
    let process_uuid = Uuid::now_v7();
    let start = ProcessStart::new(ProcessEngineKind::SandboxContainer, "RECLAIM_TEST", None)
        .with_process_uuid(process_uuid)
        .with_parent_session_id(session.clone())
        .with_sandbox_adapter_id("sandbox-adapter-test")
        .with_sandbox_internal_id("sandbox-internal-test");
    store
        .write_batch(vec![LedgerEvent::Start(start)])
        .await
        .expect("seed open process lifecycle");

    let claimed = store
        .active_processes_for_session(&session)
        .await
        .expect("claim lifecycle");
    let current = claimed.first().expect("one claimed lifecycle").clone();
    let stale_claim = ReclaimClaim {
        claimant_uuid: Uuid::now_v7(),
        kill_operation_uuid: current.reclaim_claim.kill_operation_uuid,
        generation: current.reclaim_claim.generation.saturating_sub(1),
        claimed_at_unix_ms: current.reclaim_claim.claimed_at_unix_ms,
        lease_expires_at_unix_ms: current.reclaim_claim.lease_expires_at_unix_ms,
    };

    store
        .release_reclaim_claim(process_uuid, &stale_claim)
        .await
        .expect_err("stale claimant must not release the current claim");

    let mut stale_process = current.clone();
    stale_process.reclaim_claim = stale_claim.clone();
    stale_process.metadata_jsonb["reclaim_claim"] =
        serde_json::to_value(&stale_claim).expect("serialize stale claim");
    store
        .write_batch(vec![LedgerEvent::Stop(stale_process.reclaim_stop(-1))])
        .await
        .expect_err("stale claimant must not finalize the current claim");

    let row: (Option<chrono::DateTime<Utc>>, String, serde_json::Value) = sqlx::query_as(
        "SELECT stopped_at, stop_reason, metadata_jsonb FROM kernel_process_lifecycle WHERE process_uuid = $1",
    )
    .bind(process_uuid)
    .fetch_one(&pool)
    .await
    .expect("read lifecycle after stale claimant attempts");
    assert_eq!(row.0, None, "stale claimant must leave lifecycle open");
    assert_eq!(row.1, "reclaim_claimed");
    assert_eq!(
        row.2["reclaim_claim"]["claimant_uuid"],
        current.reclaim_claim.claimant_uuid.to_string()
    );

    store
        .release_reclaim_claim(process_uuid, &current.reclaim_claim)
        .await
        .expect("current claimant can release its own claim");
}

fn reclaimable(session_id: &str, engine_kind: ProcessEngineKind) -> ReclaimableProcess {
    let now_ms = Utc::now().timestamp_millis();
    let reclaim_claim = ReclaimClaim {
        claimant_uuid: Uuid::now_v7(),
        kill_operation_uuid: Uuid::now_v7(),
        generation: 1,
        claimed_at_unix_ms: now_ms,
        lease_expires_at_unix_ms: now_ms + 30_000,
    };
    ReclaimableProcess {
        process_uuid: Uuid::now_v7(),
        os_pid: None,
        parent_session_id: Some(session_id.to_string()),
        parent_process_id: None,
        sandbox_adapter_id: Some("sandbox-adapter-test".to_string()),
        sandbox_internal_id: Some("sandbox-internal-test".to_string()),
        engine_kind,
        started_at: Utc::now(),
        model_artifact_sha256: None,
        work_profile_id: Some("work-profile-test".to_string()),
        owner_role: "KERNEL_BUILDER".to_string(),
        owner_wp: Some("WP-KERNEL-004".to_string()),
        role_id: Some("KERNEL_BUILDER".to_string()),
        wp_id: Some("WP-KERNEL-004".to_string()),
        mt_id: Some("MT-053".to_string()),
        runtime_owner: None,
        sandbox_capabilities_snapshot: serde_json::json!({"adapter_id": "sandbox-adapter-test"}),
        metadata_jsonb: serde_json::json!({"reclaim_claim": reclaim_claim}),
        reclaim_claim,
        kill_succeeded_pending_stop: false,
    }
}

struct Fixture {
    reclaim: Arc<Reclaim>,
    killer: Arc<RecordingKill>,
    stop_writer: Arc<RecordingStopWriter>,
}

impl Fixture {
    fn new(active: HashMap<String, Vec<ReclaimableProcess>>, kill_failures: HashSet<Uuid>) -> Self {
        let store = Arc::new(MemoryReclaimStore {
            active: Mutex::new(active),
        });
        let killer = Arc::new(RecordingKill {
            killed: Mutex::new(Vec::new()),
            failures: kill_failures,
        });
        let stop_writer = Arc::new(RecordingStopWriter {
            stops: Arc::new(Mutex::new(Vec::new())),
        });
        let reclaim = Arc::new(Reclaim::new(
            store,
            Arc::clone(&killer),
            Arc::clone(&stop_writer),
        ));
        Self {
            reclaim,
            killer,
            stop_writer,
        }
    }
}

struct MemoryReclaimStore {
    active: Mutex<HashMap<String, Vec<ReclaimableProcess>>>,
}

#[async_trait]
impl ReclaimProcessStore for MemoryReclaimStore {
    async fn active_processes_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, handshake_core::process_ledger::ProcessLedgerError> {
        Ok(self
            .active
            .lock()
            .unwrap()
            .get(session_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn active_stale_owned_processes_for_session(
        &self,
        session_id: &str,
        _owner_runtime_instance_id: Uuid,
        _owner_host_scope_id: &str,
        authorized_process_uuids: &[Uuid],
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        Ok(self
            .active_processes_for_session(session_id)
            .await?
            .into_iter()
            .filter(|process| authorized_process_uuids.contains(&process.process_uuid))
            .collect())
    }

    async fn renew_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError> {
        Ok(claim.clone())
    }
    async fn mark_reclaim_kill_succeeded(
        &self,
        _stop: &ProcessStop,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn mark_reclaim_kill_started(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn release_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn resolve_reclaim_kill_operation(
        &self,
        _process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
        _status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn in_progress_kill_operations_for_session(
        &self,
        _session_id: &str,
        _limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        Ok(Vec::new())
    }

    async fn in_progress_kill_operations_for_stale_owner(
        &self,
        session_id: &str,
        _owner_runtime_instance_id: Uuid,
        _owner_host_scope_id: &str,
        _authorized_process_uuids: &[Uuid],
        limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        self.in_progress_kill_operations_for_session(session_id, limit)
            .await
    }
}

struct TrackingReclaimStore {
    active: Mutex<Option<ReclaimableProcess>>,
    releases: Mutex<Vec<(Uuid, ReclaimClaim)>>,
    renewals: Mutex<usize>,
    pending_marks: Mutex<usize>,
}

struct AbortCleanupStore {
    active: Mutex<Option<Vec<ReclaimableProcess>>>,
    releases: Mutex<Vec<(Uuid, ReclaimClaim)>>,
}

impl AbortCleanupStore {
    fn new(active: Vec<ReclaimableProcess>) -> Self {
        Self {
            active: Mutex::new(Some(active)),
            releases: Mutex::new(Vec::new()),
        }
    }

    fn releases(&self) -> Vec<(Uuid, ReclaimClaim)> {
        self.releases.lock().unwrap().clone()
    }
}

#[async_trait]
impl ReclaimProcessStore for AbortCleanupStore {
    async fn active_processes_for_session(
        &self,
        _session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        Ok(self.active.lock().unwrap().take().unwrap_or_default())
    }

    async fn renew_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError> {
        Ok(claim.clone())
    }
    async fn mark_reclaim_kill_succeeded(
        &self,
        _stop: &ProcessStop,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn mark_reclaim_kill_started(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }

    async fn release_reclaim_claim(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        self.releases
            .lock()
            .unwrap()
            .push((process_uuid, claim.clone()));
        Ok(())
    }
    async fn resolve_reclaim_kill_operation(
        &self,
        _process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
        _status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn in_progress_kill_operations_for_session(
        &self,
        _session_id: &str,
        _limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        Ok(Vec::new())
    }
}

struct RecoverySweepProcess {
    process: ReclaimableProcess,
    phase: &'static str,
}

struct RecoverySweepStore {
    processes: Mutex<Vec<RecoverySweepProcess>>,
    transition_failures: HashSet<Uuid>,
    last_limit: Mutex<usize>,
}

impl RecoverySweepStore {
    fn new(processes: Vec<ReclaimableProcess>) -> Self {
        Self {
            processes: Mutex::new(
                processes
                    .into_iter()
                    .map(|process| RecoverySweepProcess {
                        process,
                        phase: "reclaim_kill_in_progress",
                    })
                    .collect(),
            ),
            transition_failures: HashSet::new(),
            last_limit: Mutex::new(0),
        }
    }

    fn with_transition_failure(mut self, process_uuid: Uuid) -> Self {
        self.transition_failures.insert(process_uuid);
        self
    }

    fn phase(&self, process_uuid: Uuid) -> &'static str {
        self.processes
            .lock()
            .unwrap()
            .iter()
            .find(|entry| entry.process.process_uuid == process_uuid)
            .expect("known recovery process")
            .phase
    }

    fn last_limit(&self) -> usize {
        *self.last_limit.lock().unwrap()
    }
}

#[async_trait]
impl ReclaimProcessStore for RecoverySweepStore {
    async fn active_processes_for_session(
        &self,
        _session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        let mut processes = self.processes.lock().unwrap();
        let mut active = Vec::new();
        for entry in processes.iter_mut() {
            match entry.phase {
                "not_started" | "failed" => {
                    entry.phase = "reclaim_claimed";
                    active.push(entry.process.clone());
                }
                "kill_succeeded_pending_stop" => {
                    entry.phase = "finalizing_pending_stop";
                    let mut pending = entry.process.clone();
                    pending.kill_succeeded_pending_stop = true;
                    active.push(pending);
                }
                _ => {}
            }
        }
        Ok(active)
    }

    async fn renew_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError> {
        Ok(claim.clone())
    }

    async fn mark_reclaim_kill_succeeded(
        &self,
        stop: &ProcessStop,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        let mut processes = self.processes.lock().unwrap();
        let entry = processes
            .iter_mut()
            .find(|entry| entry.process.process_uuid == stop.process_uuid)
            .ok_or_else(|| ProcessLedgerError::Store("unknown recovery process".to_string()))?;
        if entry.phase != "reclaim_kill_in_progress"
            || entry.process.reclaim_claim.kill_operation_uuid != claim.kill_operation_uuid
        {
            return Err(ProcessLedgerError::Store(
                "recovery pending marker lost operation ownership".to_string(),
            ));
        }
        entry.phase = "kill_succeeded_pending_stop";
        Ok(())
    }

    async fn mark_reclaim_kill_started(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        let mut processes = self.processes.lock().unwrap();
        let entry = processes
            .iter_mut()
            .find(|entry| entry.process.process_uuid == process_uuid)
            .ok_or_else(|| ProcessLedgerError::Store("unknown recovery process".to_string()))?;
        if entry.phase != "reclaim_claimed"
            || entry.process.reclaim_claim.kill_operation_uuid != claim.kill_operation_uuid
        {
            return Err(ProcessLedgerError::Store(
                "recovery kill-start lost operation ownership".to_string(),
            ));
        }
        entry.phase = "reclaim_kill_in_progress";
        Ok(())
    }

    async fn release_reclaim_claim(
        &self,
        process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        let mut processes = self.processes.lock().unwrap();
        let entry = processes
            .iter_mut()
            .find(|entry| entry.process.process_uuid == process_uuid)
            .ok_or_else(|| ProcessLedgerError::Store("unknown recovery process".to_string()))?;
        entry.phase = "released";
        Ok(())
    }

    async fn resolve_reclaim_kill_operation(
        &self,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
        status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        if self.transition_failures.contains(&process_uuid) {
            return Err(ProcessLedgerError::Store(
                "simulated recovery state-transition failure".to_string(),
            ));
        }
        let mut processes = self.processes.lock().unwrap();
        let entry = processes
            .iter_mut()
            .find(|entry| entry.process.process_uuid == process_uuid)
            .ok_or_else(|| ProcessLedgerError::Store("unknown recovery process".to_string()))?;
        if entry.phase != "reclaim_kill_in_progress"
            || entry.process.reclaim_claim.kill_operation_uuid != kill_operation_uuid
        {
            return Err(ProcessLedgerError::Store(
                "recovery resolution lost operation ownership".to_string(),
            ));
        }
        entry.phase = match status {
            ReclaimKillOperationStatus::Succeeded => "kill_succeeded_pending_stop",
            ReclaimKillOperationStatus::Failed => "failed",
            ReclaimKillOperationStatus::NotStarted => "not_started",
            ReclaimKillOperationStatus::InProgress | ReclaimKillOperationStatus::Unknown => {
                return Err(ProcessLedgerError::InvalidConfig(
                    "open recovery evidence must not mutate the row".to_string(),
                ));
            }
        };
        Ok(())
    }

    async fn in_progress_kill_operations_for_session(
        &self,
        _session_id: &str,
        limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        *self.last_limit.lock().unwrap() = limit;
        Ok(self
            .processes
            .lock()
            .unwrap()
            .iter()
            .filter(|entry| entry.phase == "reclaim_kill_in_progress")
            .take(limit)
            .map(|entry| ReclaimKillOperationCandidate::Operation {
                operation: ReclaimKillOperation {
                    process_uuid: entry.process.process_uuid,
                    kill_operation_uuid: entry.process.reclaim_claim.kill_operation_uuid,
                },
            })
            .collect())
    }
}

struct DurableKillFenceStore {
    process: ReclaimableProcess,
    phase: Mutex<&'static str>,
    query_count: Mutex<usize>,
    kill_started: tokio::sync::Notify,
}

impl DurableKillFenceStore {
    fn new(process: ReclaimableProcess) -> Self {
        Self {
            process,
            phase: Mutex::new("unclaimed"),
            query_count: Mutex::new(0),
            kill_started: tokio::sync::Notify::new(),
        }
    }

    async fn wait_until_kill_started(&self) {
        loop {
            let notified = self.kill_started.notified();
            if self.phase() == "reclaim_kill_in_progress" {
                return;
            }
            notified.await;
        }
    }

    fn phase(&self) -> &'static str {
        *self.phase.lock().unwrap()
    }

    fn query_count(&self) -> usize {
        *self.query_count.lock().unwrap()
    }
}

#[async_trait]
impl ReclaimProcessStore for DurableKillFenceStore {
    async fn active_processes_for_session(
        &self,
        _session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        *self.query_count.lock().unwrap() += 1;
        let mut phase = self.phase.lock().unwrap();
        match *phase {
            "unclaimed" | "failed" | "not_started" => {
                *phase = "reclaim_claimed";
                Ok(vec![self.process.clone()])
            }
            "kill_succeeded_pending_stop" => {
                *phase = "finalizing_pending_stop";
                let mut pending = self.process.clone();
                pending.kill_succeeded_pending_stop = true;
                Ok(vec![pending])
            }
            _ => Ok(Vec::new()),
        }
    }

    async fn mark_reclaim_kill_started(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        assert_eq!(process_uuid, self.process.process_uuid);
        assert_eq!(claim, &self.process.reclaim_claim);
        let mut phase = self.phase.lock().unwrap();
        if *phase != "reclaim_claimed" {
            return Err(ProcessLedgerError::Store(
                "shared authority rejected kill-start transition".to_string(),
            ));
        }
        *phase = "reclaim_kill_in_progress";
        drop(phase);
        self.kill_started.notify_waiters();
        Ok(())
    }

    async fn renew_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError> {
        Err(ProcessLedgerError::Store(
            "simulated renewal outage across lease expiry".to_string(),
        ))
    }

    async fn mark_reclaim_kill_succeeded(
        &self,
        _stop: &ProcessStop,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        let mut phase = self.phase.lock().unwrap();
        assert_eq!(*phase, "reclaim_kill_in_progress");
        *phase = "kill_succeeded_pending_stop";
        Ok(())
    }

    async fn release_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        *self.phase.lock().unwrap() = "released";
        Ok(())
    }
    async fn resolve_reclaim_kill_operation(
        &self,
        _process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
        status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        *self.phase.lock().unwrap() = match status {
            ReclaimKillOperationStatus::Succeeded => "kill_succeeded_pending_stop",
            ReclaimKillOperationStatus::Failed => "failed",
            ReclaimKillOperationStatus::NotStarted => "not_started",
            ReclaimKillOperationStatus::InProgress => "reclaim_kill_in_progress",
            ReclaimKillOperationStatus::Unknown => "unknown",
        };
        Ok(())
    }
    async fn in_progress_kill_operations_for_session(
        &self,
        _session_id: &str,
        _limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        if self.phase() == "reclaim_kill_in_progress" {
            Ok(vec![ReclaimKillOperationCandidate::Operation {
                operation: ReclaimKillOperation {
                    process_uuid: self.process.process_uuid,
                    kill_operation_uuid: self.process.reclaim_claim.kill_operation_uuid,
                },
            }])
        } else {
            Ok(Vec::new())
        }
    }
}

impl TrackingReclaimStore {
    fn new(process: ReclaimableProcess) -> Self {
        Self {
            active: Mutex::new(Some(process)),
            releases: Mutex::new(Vec::new()),
            renewals: Mutex::new(0),
            pending_marks: Mutex::new(0),
        }
    }

    fn releases(&self) -> Vec<(Uuid, ReclaimClaim)> {
        self.releases.lock().unwrap().clone()
    }

    fn renewals(&self) -> usize {
        *self.renewals.lock().unwrap()
    }

    fn pending_marks(&self) -> usize {
        *self.pending_marks.lock().unwrap()
    }
}

#[async_trait]
impl ReclaimProcessStore for TrackingReclaimStore {
    async fn active_processes_for_session(
        &self,
        _session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        Ok(self.active.lock().unwrap().take().into_iter().collect())
    }

    async fn renew_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError> {
        *self.renewals.lock().unwrap() += 1;
        let mut renewed = claim.clone();
        renewed.claimed_at_unix_ms = Utc::now().timestamp_millis();
        renewed.lease_expires_at_unix_ms = renewed.claimed_at_unix_ms + 30_000;
        Ok(renewed)
    }

    async fn mark_reclaim_kill_succeeded(
        &self,
        _stop: &ProcessStop,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        *self.pending_marks.lock().unwrap() += 1;
        Ok(())
    }

    async fn mark_reclaim_kill_started(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }

    async fn release_reclaim_claim(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        self.releases
            .lock()
            .unwrap()
            .push((process_uuid, claim.clone()));
        Ok(())
    }
    async fn resolve_reclaim_kill_operation(
        &self,
        _process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
        _status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn in_progress_kill_operations_for_session(
        &self,
        _session_id: &str,
        _limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        Ok(Vec::new())
    }
}

struct LostOwnershipStore {
    active: Mutex<Option<ReclaimableProcess>>,
}

impl LostOwnershipStore {
    fn new(process: ReclaimableProcess) -> Self {
        Self {
            active: Mutex::new(Some(process)),
        }
    }
}

#[async_trait]
impl ReclaimProcessStore for LostOwnershipStore {
    async fn active_processes_for_session(
        &self,
        _session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        Ok(self.active.lock().unwrap().take().into_iter().collect())
    }

    async fn renew_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError> {
        Err(ProcessLedgerError::Store(
            "reclaim ownership was replaced by a newer claimant".to_string(),
        ))
    }

    async fn mark_reclaim_kill_succeeded(
        &self,
        _stop: &ProcessStop,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Err(ProcessLedgerError::Store(
            "stale reclaim token cannot mark pending STOP".to_string(),
        ))
    }

    async fn mark_reclaim_kill_started(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn release_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn resolve_reclaim_kill_operation(
        &self,
        _process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
        _status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn in_progress_kill_operations_for_session(
        &self,
        _session_id: &str,
        _limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        Ok(Vec::new())
    }
}

/// In-memory model of the MT-008 atomic-claim semantics: the FIRST reclaim to
/// reach a session under the lock drains that session's active rows; any
/// concurrent reclaim then observes an empty active set (the rows already have
/// an in-progress reclaim claim). This mirrors the Postgres `UPDATE ...
/// RETURNING` guarded by `FOR UPDATE` and lets the serialization decision be
/// proven without a live database.
struct ClaimingReclaimStore {
    active: Mutex<HashMap<String, Vec<ReclaimableProcess>>>,
}

impl ClaimingReclaimStore {
    fn new(active: HashMap<String, Vec<ReclaimableProcess>>) -> Self {
        Self {
            active: Mutex::new(active),
        }
    }
}

#[async_trait]
impl ReclaimProcessStore for ClaimingReclaimStore {
    async fn active_processes_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, handshake_core::process_ledger::ProcessLedgerError> {
        // The lock guard models the row lock held for the duration of the atomic
        // claim; `remove` models marking the rows as claimed so a concurrent
        // reclaim cannot see them again.
        let mut guard = self.active.lock().unwrap();
        Ok(guard.remove(session_id).unwrap_or_default())
    }

    async fn renew_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError> {
        Ok(claim.clone())
    }
    async fn mark_reclaim_kill_succeeded(
        &self,
        _stop: &ProcessStop,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn mark_reclaim_kill_started(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn release_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn resolve_reclaim_kill_operation(
        &self,
        _process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
        _status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
    async fn in_progress_kill_operations_for_session(
        &self,
        _session_id: &str,
        _limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        Ok(Vec::new())
    }
}

struct RecordingKill {
    killed: Mutex<Vec<Uuid>>,
    failures: HashSet<Uuid>,
}

struct FailThenSucceedKill {
    attempts: Mutex<Vec<Uuid>>,
}

impl FailThenSucceedKill {
    fn attempts(&self) -> Vec<Uuid> {
        self.attempts.lock().unwrap().clone()
    }
}

#[async_trait]
impl SandboxKill for FailThenSucceedKill {
    async fn kill(&self, process_uuid: Uuid, _kill_operation_uuid: Uuid) -> Result<(), KillError> {
        let mut attempts = self.attempts.lock().unwrap();
        attempts.push(process_uuid);
        if attempts.len() == 1 {
            Err(KillError::new("injected first kill failure"))
        } else {
            Ok(())
        }
    }

    async fn kill_operation_status(
        &self,
        _process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, KillError> {
        Ok(ReclaimKillOperationStatus::NotStarted)
    }
}

struct ReleaseFailsOnceStore {
    process: ReclaimableProcess,
    release_attempts: Mutex<usize>,
}

struct PostgresReleaseFailsOnceStore {
    inner: PostgresProcessLedgerStore,
    fail_release: AtomicBool,
}

#[async_trait]
impl ReclaimProcessStore for PostgresReleaseFailsOnceStore {
    async fn active_processes_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        self.inner.active_processes_for_session(session_id).await
    }

    async fn renew_reclaim_claim(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError> {
        self.inner.renew_reclaim_claim(process_uuid, claim).await
    }

    async fn mark_reclaim_kill_succeeded(
        &self,
        stop: &ProcessStop,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        self.inner.mark_reclaim_kill_succeeded(stop, claim).await
    }

    async fn mark_reclaim_kill_started(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        self.inner
            .mark_reclaim_kill_started(process_uuid, claim)
            .await
    }

    async fn release_reclaim_claim(
        &self,
        process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        if self.fail_release.swap(false, Ordering::SeqCst) {
            return Err(ProcessLedgerError::Store(
                "injected PostgreSQL claim-release transport failure".to_string(),
            ));
        }
        self.inner.release_reclaim_claim(process_uuid, claim).await
    }

    async fn resolve_reclaim_kill_operation(
        &self,
        process_uuid: Uuid,
        kill_operation_uuid: Uuid,
        status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        self.inner
            .resolve_reclaim_kill_operation(process_uuid, kill_operation_uuid, status)
            .await
    }

    async fn in_progress_kill_operations_for_session(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        self.inner
            .in_progress_kill_operations_for_session(session_id, limit)
            .await
    }
}

#[async_trait]
impl ReclaimProcessStore for ReleaseFailsOnceStore {
    async fn active_processes_for_session(
        &self,
        _session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        Ok(vec![self.process.clone()])
    }

    async fn renew_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError> {
        Ok(claim.clone())
    }

    async fn mark_reclaim_kill_succeeded(
        &self,
        _stop: &ProcessStop,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }

    async fn mark_reclaim_kill_started(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }

    async fn release_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        let mut attempts = self.release_attempts.lock().unwrap();
        *attempts += 1;
        if *attempts == 1 {
            Err(ProcessLedgerError::Store(
                "injected claim release failure".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    async fn resolve_reclaim_kill_operation(
        &self,
        _process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
        _status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }

    async fn in_progress_kill_operations_for_session(
        &self,
        _session_id: &str,
        _limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        Ok(Vec::new())
    }
}

struct StatusKill {
    statuses: HashMap<Uuid, Result<ReclaimKillOperationStatus, String>>,
    killed: Mutex<Vec<(Uuid, Uuid)>>,
}

impl StatusKill {
    fn new(statuses: HashMap<Uuid, Result<ReclaimKillOperationStatus, String>>) -> Self {
        Self {
            statuses,
            killed: Mutex::new(Vec::new()),
        }
    }

    fn killed(&self) -> Vec<(Uuid, Uuid)> {
        self.killed.lock().unwrap().clone()
    }
}

#[async_trait]
impl SandboxKill for StatusKill {
    async fn kill(&self, process_uuid: Uuid, kill_operation_uuid: Uuid) -> Result<(), KillError> {
        self.killed
            .lock()
            .unwrap()
            .push((process_uuid, kill_operation_uuid));
        Ok(())
    }

    async fn kill_operation_status(
        &self,
        process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, KillError> {
        self.statuses
            .get(&process_uuid)
            .cloned()
            .unwrap_or(Ok(ReclaimKillOperationStatus::Unknown))
            .map_err(KillError::new)
    }
}

impl RecordingKill {
    fn killed(&self) -> Vec<Uuid> {
        self.killed.lock().unwrap().clone()
    }
}

#[async_trait]
impl SandboxKill for RecordingKill {
    async fn kill(&self, process_uuid: Uuid, _kill_operation_uuid: Uuid) -> Result<(), KillError> {
        self.killed.lock().unwrap().push(process_uuid);
        if self.failures.contains(&process_uuid) {
            return Err(KillError::new("mock kill failure"));
        }
        Ok(())
    }

    async fn kill_operation_status(
        &self,
        _process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, KillError> {
        Ok(ReclaimKillOperationStatus::Succeeded)
    }
}

struct SlowKill {
    delay: Duration,
    killed: Mutex<Vec<Uuid>>,
}

impl SlowKill {
    fn killed(&self) -> Vec<Uuid> {
        self.killed.lock().unwrap().clone()
    }
}

#[async_trait]
impl SandboxKill for SlowKill {
    async fn kill(&self, process_uuid: Uuid, _kill_operation_uuid: Uuid) -> Result<(), KillError> {
        tokio::time::sleep(self.delay).await;
        self.killed.lock().unwrap().push(process_uuid);
        Ok(())
    }

    async fn kill_operation_status(
        &self,
        _process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
    ) -> Result<ReclaimKillOperationStatus, KillError> {
        Ok(ReclaimKillOperationStatus::Succeeded)
    }
}

struct RecordingStopWriter {
    stops: Arc<Mutex<Vec<ProcessStop>>>,
}

impl RecordingStopWriter {
    fn stops(&self) -> Vec<ProcessStop> {
        self.stops.lock().unwrap().clone()
    }
}

impl ReclaimStopWriter for RecordingStopWriter {
    fn reserve_reclaim_stop(
        &self,
    ) -> Result<Box<dyn ReclaimStopReservation>, handshake_core::process_ledger::ProcessLedgerError>
    {
        Ok(Box::new(RecordingStopReservation {
            stops: Arc::clone(&self.stops),
        }))
    }
}

struct RecordingStopReservation {
    stops: Arc<Mutex<Vec<ProcessStop>>>,
}

#[async_trait]
impl ReclaimStopReservation for RecordingStopReservation {
    async fn persist(
        self: Box<Self>,
        stop: ProcessStop,
        _timeout: Duration,
    ) -> Result<(), handshake_core::process_ledger::ProcessLedgerError> {
        self.stops.lock().unwrap().push(stop);
        Ok(())
    }
}

struct FailingStopWriter;

impl ReclaimStopWriter for FailingStopWriter {
    fn reserve_reclaim_stop(&self) -> Result<Box<dyn ReclaimStopReservation>, ProcessLedgerError> {
        Ok(Box::new(FailingStopReservation))
    }
}

struct FailingStopReservation;

#[async_trait]
impl ReclaimStopReservation for FailingStopReservation {
    async fn persist(
        self: Box<Self>,
        _stop: ProcessStop,
        _timeout: Duration,
    ) -> Result<(), ProcessLedgerError> {
        Err(ProcessLedgerError::Store(
            "simulated post-kill STOP persistence failure".to_string(),
        ))
    }
}

struct NoopOverflowSink;

impl ProcessLedgerOverflowSink for NoopOverflowSink {
    fn emit_overflow(&self, _event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        Ok(())
    }
}

struct FencedStopStore {
    current_claim: ReclaimClaim,
    persisted_claims: Mutex<Vec<ReclaimClaim>>,
}

impl FencedStopStore {
    fn new(current_claim: ReclaimClaim) -> Self {
        Self {
            current_claim,
            persisted_claims: Mutex::new(Vec::new()),
        }
    }

    fn persisted_claims(&self) -> Vec<ReclaimClaim> {
        self.persisted_claims.lock().unwrap().clone()
    }
}

#[async_trait]
impl ProcessLedgerStore for FencedStopStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        let mut accepted = Vec::new();
        for event in &events {
            let LedgerEvent::Stop(stop) = event else {
                continue;
            };
            let claim: ReclaimClaim = serde_json::from_value(
                stop.metadata_jsonb
                    .get("reclaim_claim")
                    .cloned()
                    .expect("test STOP carries reclaim claim"),
            )
            .expect("decode test reclaim claim");
            if claim.claimant_uuid != self.current_claim.claimant_uuid
                || claim.generation != self.current_claim.generation
            {
                return Err(ProcessLedgerError::StopIdentityConflict {
                    process_uuid: stop.process_uuid,
                    conflicting_stop: Box::new(stop.clone()),
                });
            }
            accepted.push(claim);
        }
        self.persisted_claims.lock().unwrap().extend(accepted);
        Ok(())
    }
}

struct FakeStaleSource {
    sessions: Mutex<Vec<String>>,
    authorized_process_uuids: Vec<Uuid>,
    owner_scope: Option<(Uuid, String)>,
    scans: Mutex<usize>,
}

impl FakeStaleSource {
    fn scoped(sessions: Vec<String>, authorized_process_uuids: Vec<Uuid>) -> Self {
        Self {
            sessions: Mutex::new(sessions),
            authorized_process_uuids,
            owner_scope: Some((Uuid::now_v7(), "fake-stale-owner-host".to_string())),
            scans: Mutex::new(0),
        }
    }

    fn unscoped(sessions: Vec<String>) -> Self {
        Self {
            sessions: Mutex::new(sessions),
            authorized_process_uuids: Vec::new(),
            owner_scope: None,
            scans: Mutex::new(0),
        }
    }

    fn scan_count(&self) -> usize {
        *self.scans.lock().unwrap()
    }
}

#[async_trait]
impl StaleSessionSource for FakeStaleSource {
    async fn stale_sessions(
        &self,
        _ttl: Duration,
    ) -> Result<Vec<String>, handshake_core::process_ledger::ProcessLedgerError> {
        *self.scans.lock().unwrap() += 1;
        Ok(std::mem::take(&mut *self.sessions.lock().unwrap()))
    }

    async fn stale_session_process_sets(
        &self,
        _ttl: Duration,
    ) -> Result<Vec<StaleSessionProcessSet>, ProcessLedgerError> {
        *self.scans.lock().unwrap() += 1;
        Ok(std::mem::take(&mut *self.sessions.lock().unwrap())
            .into_iter()
            .map(|session_id| StaleSessionProcessSet {
                session_id,
                authorized_process_uuids: self.authorized_process_uuids.clone(),
            })
            .collect())
    }

    fn self_runtime_owner_scope(&self) -> Option<(Uuid, String)> {
        self.owner_scope.clone()
    }
}

// ---------------------------------------------------------------------------
// MT-019 F5 + F6: boot-reconcile report honesty.
//
// This file uses mock stores and has no file-level cfg gate, so these proofs run
// on every host. They cover the counting/surfacing contract; the real-kill and
// real-PostgreSQL behaviour is proven in
// `process_reclaim_real_lifecycle_pg_tests`.
// ---------------------------------------------------------------------------

struct RestartOnlyStaleSource {
    sessions: Vec<String>,
}

#[async_trait]
impl StaleSessionSource for RestartOnlyStaleSource {
    async fn stale_sessions(&self, _ttl: Duration) -> Result<Vec<String>, ProcessLedgerError> {
        Ok(Vec::new())
    }

    async fn restart_sessions(&self) -> Result<Vec<String>, ProcessLedgerError> {
        Ok(self.sessions.clone())
    }
}

/// Advances one in-progress kill operation (so the sweep runs its follow-up
/// reclaim) and then fails that follow-up reclaim exactly once.
struct SweepReclaimErrorStore {
    session_claims: Mutex<usize>,
    process_uuid: Uuid,
    kill_operation_uuid: Uuid,
}

#[async_trait]
impl ReclaimProcessStore for SweepReclaimErrorStore {
    async fn active_processes_for_session(
        &self,
        _session_id: &str,
    ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
        let mut claims = self.session_claims.lock().unwrap();
        *claims += 1;
        if *claims == 1 {
            // The sweep's own follow-up reclaim fails. The sweep still returns
            // Ok, carrying the failure in `reclaim_error`.
            return Err(ProcessLedgerError::Store(
                "simulated in-progress sweep follow-up reclaim failure".to_string(),
            ));
        }
        Ok(Vec::new())
    }

    async fn renew_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        claim: &ReclaimClaim,
    ) -> Result<ReclaimClaim, ProcessLedgerError> {
        Ok(claim.clone())
    }

    async fn mark_reclaim_kill_succeeded(
        &self,
        _stop: &ProcessStop,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }

    async fn mark_reclaim_kill_started(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }

    async fn release_reclaim_claim(
        &self,
        _process_uuid: Uuid,
        _claim: &ReclaimClaim,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }

    async fn resolve_reclaim_kill_operation(
        &self,
        _process_uuid: Uuid,
        _kill_operation_uuid: Uuid,
        _status: ReclaimKillOperationStatus,
    ) -> Result<(), ProcessLedgerError> {
        Ok(())
    }

    async fn in_progress_kill_operations_for_session(
        &self,
        _session_id: &str,
        _limit: usize,
    ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
        Ok(vec![ReclaimKillOperationCandidate::Operation {
            operation: ReclaimKillOperation {
                process_uuid: self.process_uuid,
                kill_operation_uuid: self.kill_operation_uuid,
            },
        }])
    }
}

#[tokio::test]
async fn mt019_boot_reconcile_surfaces_in_progress_sweep_reclaim_error() {
    let process_uuid = Uuid::now_v7();
    let store = Arc::new(SweepReclaimErrorStore {
        session_claims: Mutex::new(0),
        process_uuid,
        kill_operation_uuid: Uuid::now_v7(),
    });
    // Terminal evidence, so the sweep advances state and runs its follow-up reclaim.
    let killer = Arc::new(StatusKill::new(HashMap::from([(
        process_uuid,
        Ok(ReclaimKillOperationStatus::Succeeded),
    )])));
    let stop_writer = Arc::new(RecordingStopWriter {
        stops: Arc::new(Mutex::new(Vec::new())),
    });
    let reclaim = Reclaim::new(store, killer, stop_writer);
    let stale_source = RestartOnlyStaleSource {
        sessions: vec!["SR-MT019-SWEEP".to_string()],
    };

    let report =
        handshake_core::process_ledger::reconcile_restart_orphans_at_boot(&reclaim, &stale_source)
            .await
            .expect("boot reconcile must not abort on a sweep-internal reclaim error");

    assert_eq!(
        report.sweep_reclaim_errors.len(),
        1,
        "the in-progress sweep's reclaim_error must be surfaced, not dropped: {report:?}"
    );
    assert!(
        report.sweep_reclaim_errors[0]
            .contains("simulated in-progress sweep follow-up reclaim failure"),
        "the surfaced error must be the sweep's own reclaim error: {:?}",
        report.sweep_reclaim_errors
    );
    assert_eq!(report.sessions_reconciled, 1);
}

#[tokio::test]
async fn mt019_boot_reconcile_counts_only_proven_kills_as_reclaimed() {
    let session = "SR-MT019-COUNTERS".to_string();
    let killed = reclaimable(&session, ProcessEngineKind::OfficialCliBridge);
    let unreapable = reclaimable(&session, ProcessEngineKind::OfficialCliBridge);
    let unreapable_uuid = unreapable.process_uuid;
    let fixture = Fixture::new(
        HashMap::from([(session.clone(), vec![killed, unreapable])]),
        HashSet::from([unreapable_uuid]),
    );
    let stale_source = RestartOnlyStaleSource {
        sessions: vec![session],
    };

    let report = handshake_core::process_ledger::reconcile_restart_orphans_at_boot(
        fixture.reclaim.as_ref(),
        &stale_source,
    )
    .await
    .expect("boot reconcile stays fail-open on kill failure (recorded F3 operator decision)");

    assert_eq!(
        report.processes_reclaimed, 1,
        "only a proven Killed/KilledPendingStop may count as reclaimed: {report:?}"
    );
    assert_eq!(
        report.processes_kill_failed, 1,
        "a Failed kill must be counted separately, never as reclaimed: {report:?}"
    );
    // Fail-open, not false evidence: no STOP was written for the unreapable row.
    let stops = fixture.stop_writer.stops();
    assert_eq!(stops.len(), 1);
    assert_ne!(
        stops[0].process_uuid, unreapable_uuid,
        "an unreapable process must never receive a STOP"
    );
}

#[tokio::test]
async fn mt019_restart_reconcile_binds_the_owner_predicate_when_the_source_knows_its_instance() {
    struct OwnerAwareStaleSource {
        instance_id: Uuid,
    }

    #[async_trait]
    impl StaleSessionSource for OwnerAwareStaleSource {
        async fn stale_sessions(&self, _ttl: Duration) -> Result<Vec<String>, ProcessLedgerError> {
            Ok(Vec::new())
        }

        async fn restart_sessions(&self) -> Result<Vec<String>, ProcessLedgerError> {
            Ok(vec!["SR-MT019-OWNER".to_string()])
        }

        fn self_runtime_instance_id(&self) -> Option<Uuid> {
            Some(self.instance_id)
        }
    }

    #[derive(Default)]
    struct OwnerPredicateStore {
        foreign_owner_claims: Mutex<Vec<(String, Uuid)>>,
    }

    #[async_trait]
    impl ReclaimProcessStore for OwnerPredicateStore {
        async fn active_processes_for_session(
            &self,
            _session_id: &str,
        ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
            panic!("a restart pass must never use the owner-blind session claim");
        }

        async fn active_foreign_owner_processes_for_session(
            &self,
            session_id: &str,
            excluded_owner_runtime_instance_id: Uuid,
        ) -> Result<Vec<ReclaimableProcess>, ProcessLedgerError> {
            self.foreign_owner_claims
                .lock()
                .unwrap()
                .push((session_id.to_string(), excluded_owner_runtime_instance_id));
            Ok(Vec::new())
        }

        async fn renew_reclaim_claim(
            &self,
            _process_uuid: Uuid,
            claim: &ReclaimClaim,
        ) -> Result<ReclaimClaim, ProcessLedgerError> {
            Ok(claim.clone())
        }

        async fn mark_reclaim_kill_succeeded(
            &self,
            _stop: &ProcessStop,
            _claim: &ReclaimClaim,
        ) -> Result<(), ProcessLedgerError> {
            Ok(())
        }

        async fn mark_reclaim_kill_started(
            &self,
            _process_uuid: Uuid,
            _claim: &ReclaimClaim,
        ) -> Result<(), ProcessLedgerError> {
            Ok(())
        }

        async fn release_reclaim_claim(
            &self,
            _process_uuid: Uuid,
            _claim: &ReclaimClaim,
        ) -> Result<(), ProcessLedgerError> {
            Ok(())
        }

        async fn resolve_reclaim_kill_operation(
            &self,
            _process_uuid: Uuid,
            _kill_operation_uuid: Uuid,
            _status: ReclaimKillOperationStatus,
        ) -> Result<(), ProcessLedgerError> {
            Ok(())
        }

        async fn in_progress_kill_operations_for_session(
            &self,
            _session_id: &str,
            _limit: usize,
        ) -> Result<Vec<ReclaimKillOperationCandidate>, ProcessLedgerError> {
            Ok(Vec::new())
        }
    }

    let instance_id = Uuid::now_v7();
    let store = Arc::new(OwnerPredicateStore::default());
    let reclaim = Reclaim::new(
        Arc::clone(&store),
        Arc::new(StatusKill::new(HashMap::new())),
        Arc::new(RecordingStopWriter {
            stops: Arc::new(Mutex::new(Vec::new())),
        }),
    );

    handshake_core::process_ledger::reconcile_restart_orphans_at_boot(
        &reclaim,
        &OwnerAwareStaleSource { instance_id },
    )
    .await
    .expect("owner-scoped restart reconcile");

    assert_eq!(
        store.foreign_owner_claims.lock().unwrap().clone(),
        vec![("SR-MT019-OWNER".to_string(), instance_id)],
        "the restart claim must carry an explicit owner_runtime_instance_id exclusion"
    );
}
