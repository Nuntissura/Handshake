use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use chrono::Utc;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

use handshake_core::process_ledger::{
    spawn_staleness_reclaim_task, KillError, ProcessEngineKind, ProcessStop, Reclaim,
    ReclaimProcessStore, ReclaimStopWriter, ReclaimTrigger, ReclaimableProcess, SandboxKill,
    StaleSessionSource, StalenessReclaimConfig, SURREAL_ACTIVE_RECLAIM_CLAIM_QUERY,
};
#[cfg(feature = "surreal-test-support")]
use handshake_core::{
    process_ledger::{LedgerEvent, ProcessLedgerStore, ProcessStart, SurrealProcessLedgerStore},
    storage::surreal::{
        bootstrap_mt137_process_ledger_test_schema, SurrealStorage, SurrealStorageConfig,
    },
};

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
async fn failure_reclaim_fails_loud_and_does_not_terminalize_a_surviving_process() {
    let process_a = reclaimable("SR-FAIL", ProcessEngineKind::SandboxContainer);
    let process_b = reclaimable("SR-FAIL", ProcessEngineKind::HelperSubprocess);
    let fixture = Fixture::new(
        HashMap::from([(
            "SR-FAIL".to_string(),
            vec![process_a.clone(), process_b.clone()],
        )]),
        HashSet::from([process_b.process_uuid]),
    );

    let error = fixture
        .reclaim
        .run("SR-FAIL", ReclaimTrigger::Failure)
        .await
        .expect_err("a surviving owned process must fail reclaim loud");
    assert!(error
        .to_string()
        .contains(&process_b.process_uuid.to_string()));
    assert!(error
        .to_string()
        .contains("external reclaim cleanup failed"));

    let stops = fixture.stop_writer.stops();
    assert_eq!(stops.len(), 1);
    assert!(stops.iter().all(|stop| stop.exit_code == Some(-1)));
    assert_eq!(stops[0].process_uuid, process_a.process_uuid);
    assert_ne!(
        stops[0].process_uuid, process_b.process_uuid,
        "the surviving process must remain non-terminal for retry"
    );
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
    let stale_source = Arc::new(FakeStaleSource::new(vec!["SR-STALE".to_string()]));
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

#[test]
fn surreal_reclaim_query_atomically_claims_open_processes_for_one_session() {
    let query = SURREAL_ACTIVE_RECLAIM_CLAIM_QUERY;
    assert!(query.starts_with("UPDATE kernel_process_lifecycle SET"));
    assert!(query.contains("parent_session_id = $session_id"));
    assert!(query.contains("stopped_at = NONE"));
    assert!(query.contains("ELSE { $claimed_at }"));
    assert!(query.contains("ELSE { $claim_reason }"));
    assert!(query.contains("stop_reason = $killed_reason { stop_reason }"));
    assert!(query.contains("RETURN BEFORE"));
    assert!(!query.to_ascii_lowercase().contains("for update"));
    assert!(!query.to_ascii_lowercase().contains("postgres"));
}

// The claim is one SurrealDB statement: it stamps every newly matched row and
// returns the pre-claim values needed by the cleanup path. A same-owner retry
// may preserve an existing cleanup-completed marker, but the row remains
// non-open and the original sentinel timestamp continues to guard finalization.
#[test]
fn surreal_reclaim_query_claim_and_readback_are_one_statement() {
    let query = SURREAL_ACTIVE_RECLAIM_CLAIM_QUERY.to_ascii_lowercase();
    assert!(
        query.starts_with("update kernel_process_lifecycle set"),
        "claim must begin with the row mutation"
    );
    assert!(
        query.contains("else { $claimed_at }"),
        "a new claim must mark stopped_at so concurrent reclaims see the row as taken"
    );
    assert!(
        query.contains("return before"),
        "the caller must act on exactly the rows claimed by the update"
    );
    assert_eq!(
        query.matches(';').count(),
        1,
        "claim and readback must remain one SurrealDB statement"
    );
}

// Logic-level serialization proof: model the atomic-claim semantics with an
// in-memory store that drains each session's active rows under a lock on the
// first call (mirroring the SurrealDB UPDATE ... RETURN BEFORE claim), and prove that two
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
        stops: Mutex::new(Vec::new()),
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

// Exercises the real SurrealDB atomic claim against the authoritative embedded
// process-ledger schema slice. Two concurrent calls from the same process boot
// must produce one exact owner for every active row. The losing caller must
// surface the outstanding same-boot claim rather than falsely reporting that
// the session has no reclaim work.
#[cfg(feature = "surreal-test-support")]
#[tokio::test]
async fn surreal_concurrent_reclaim_claims_each_row_once() {
    let temp = tempfile::tempdir().expect("SurrealDB reclaim tempdir");
    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(temp.path()).expect("valid SurrealDB test path"),
    )
    .await
    .expect("open embedded SurrealDB reclaim store");
    bootstrap_mt137_process_ledger_test_schema(&storage)
        .await
        .expect("bootstrap authoritative process-ledger schema");
    let store = Arc::new(SurrealProcessLedgerStore::new(storage));

    let session = format!("SR-SURREAL-RACE-{}", Uuid::now_v7());
    let starts: Vec<ProcessStart> = (0..4)
        .map(|offset| {
            ProcessStart::new(
                ProcessEngineKind::HelperSubprocess,
                "mt137-surreal-race-test",
                Some("WP-KERNEL-012".to_owned()),
            )
            .with_parent_session_id(session.clone())
            .with_os_pid(40_000 + offset)
        })
        .collect();
    let expected: HashSet<Uuid> = starts.iter().map(|start| start.process_uuid).collect();
    ProcessLedgerStore::write_batch(
        store.as_ref(),
        starts.into_iter().map(LedgerEvent::Start).collect(),
    )
    .await
    .expect("seed active SurrealDB process rows");

    let a = Arc::clone(&store);
    let b = Arc::clone(&store);
    let sa = session.clone();
    let sb = session.clone();
    let ra = tokio::spawn(async move { a.active_processes_for_session(&sa).await });
    let rb = tokio::spawn(async move { b.active_processes_for_session(&sb).await });
    let mut successful_claims = Vec::new();
    let mut convergence_errors = Vec::new();
    for result in [ra.await.unwrap(), rb.await.unwrap()] {
        match result {
            Ok(claimed) => successful_claims.push(claimed),
            Err(error) => convergence_errors.push(error),
        }
    }
    assert_eq!(
        successful_claims.len(),
        1,
        "exactly one concurrent caller must own the durable claims"
    );
    assert_eq!(
        convergence_errors.len(),
        1,
        "the losing caller must not translate outstanding same-boot claims into zero work"
    );
    assert!(convergence_errors[0]
        .to_string()
        .contains("same-boot reclaim claims have not converged"));
    let claimed = successful_claims.pop().expect("one successful claim");

    let claimed: Vec<Uuid> = claimed.iter().map(|process| process.process_uuid).collect();
    let mut ids = HashSet::new();
    for process_uuid in &claimed {
        assert!(
            ids.insert(*process_uuid),
            "process {} was claimed by both concurrent reclaims (double-claim)",
            process_uuid
        );
    }
    assert_eq!(claimed.len(), expected.len(), "no active row may be missed");
    assert_eq!(ids, expected);
}

fn reclaimable(session_id: &str, engine_kind: ProcessEngineKind) -> ReclaimableProcess {
    ReclaimableProcess {
        process_uuid: Uuid::now_v7(),
        os_pid: None,
        parent_session_id: session_id.to_string(),
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
        sandbox_capabilities_snapshot: serde_json::json!({"adapter_id": "sandbox-adapter-test"}),
        metadata_jsonb: serde_json::json!({}),
        reclaim_claimed_at: Utc::now(),
        reclaim_expected_reason: "reclaim_claimed:integration-fixture".to_owned(),
        reclaim_expected_killed_reason: "reclaim_killed:integration-fixture".to_owned(),
        reclaim_cleanup_completed: false,
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
            stops: Mutex::new(Vec::new()),
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

    async fn mark_cleanup_completed(
        &self,
        _process: &ReclaimableProcess,
    ) -> Result<(), handshake_core::process_ledger::ProcessLedgerError> {
        Ok(())
    }

    async fn abandon(
        &self,
        _processes: &[ReclaimableProcess],
    ) -> Result<(), handshake_core::process_ledger::ProcessLedgerError> {
        Ok(())
    }
}

/// In-memory model of the MT-008 atomic-claim semantics: the FIRST reclaim to
/// reach a session under the lock drains that session's active rows; any
/// concurrent reclaim then observes an empty active set (the rows are already
/// claimed, `stopped_at` no longer `NONE`). This mirrors the SurrealDB
/// `UPDATE ... RETURN BEFORE` claim and lets the serialization decision be
/// proven without a real embedded store in every test configuration.
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

    async fn mark_cleanup_completed(
        &self,
        _process: &ReclaimableProcess,
    ) -> Result<(), handshake_core::process_ledger::ProcessLedgerError> {
        Ok(())
    }

    async fn abandon(
        &self,
        _processes: &[ReclaimableProcess],
    ) -> Result<(), handshake_core::process_ledger::ProcessLedgerError> {
        Ok(())
    }
}

struct RecordingKill {
    killed: Mutex<Vec<Uuid>>,
    failures: HashSet<Uuid>,
}

impl RecordingKill {
    fn killed(&self) -> Vec<Uuid> {
        self.killed.lock().unwrap().clone()
    }
}

impl SandboxKill for RecordingKill {
    fn kill(&self, process_uuid: Uuid) -> Result<(), KillError> {
        self.killed.lock().unwrap().push(process_uuid);
        if self.failures.contains(&process_uuid) {
            return Err(KillError::new("mock kill failure"));
        }
        Ok(())
    }
}

struct RecordingStopWriter {
    stops: Mutex<Vec<ProcessStop>>,
}

impl RecordingStopWriter {
    fn stops(&self) -> Vec<ProcessStop> {
        self.stops.lock().unwrap().clone()
    }
}

#[async_trait]
impl ReclaimStopWriter for RecordingStopWriter {
    async fn append_reclaim_stop(
        &self,
        stop: ProcessStop,
    ) -> Result<(), handshake_core::process_ledger::ProcessLedgerError> {
        self.stops.lock().unwrap().push(stop);
        Ok(())
    }
}

struct FakeStaleSource {
    sessions: Mutex<Vec<String>>,
}

impl FakeStaleSource {
    fn new(sessions: Vec<String>) -> Self {
        Self {
            sessions: Mutex::new(sessions),
        }
    }
}

#[async_trait]
impl StaleSessionSource for FakeStaleSource {
    async fn stale_sessions(
        &self,
        _ttl: Duration,
    ) -> Result<Vec<String>, handshake_core::process_ledger::ProcessLedgerError> {
        Ok(std::mem::take(&mut *self.sessions.lock().unwrap()))
    }
}
