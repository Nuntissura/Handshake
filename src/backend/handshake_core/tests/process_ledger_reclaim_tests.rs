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
    spawn_staleness_reclaim_task, KillError, KillOutcome, ProcessEngineKind, ProcessStop, Reclaim,
    ReclaimProcessStore, ReclaimStopWriter, ReclaimTrigger, ReclaimableProcess, SandboxKill,
    StaleSessionSource, StalenessReclaimConfig, POSTGRES_ACTIVE_RECLAIM_QUERY_SQL,
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
async fn failure_reclaim_kills_pending_processes_and_writes_stop_even_when_kill_fails() {
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
    assert_eq!(stops.len(), 2);
    assert!(stops.iter().all(|stop| stop.exit_code == Some(-1)));
    assert!(stops
        .iter()
        .any(|stop| stop.process_uuid == process_a.process_uuid));
    assert!(stops
        .iter()
        .any(|stop| stop.process_uuid == process_b.process_uuid));
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
fn postgres_reclaim_query_uses_row_lock_and_open_process_filter() {
    let sql = POSTGRES_ACTIVE_RECLAIM_QUERY_SQL;
    assert!(sql.contains("FROM kernel_process_lifecycle"));
    assert!(sql.contains("parent_session_id = $1"));
    assert!(sql.contains("stopped_at IS NULL"));
    assert!(sql.contains("FOR UPDATE"));
    assert!(!sql.to_ascii_lowercase().contains("sqlite"));
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

impl ReclaimStopWriter for RecordingStopWriter {
    fn append_reclaim_stop(
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
