use std::{
    collections::BTreeMap,
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{task::JoinSet, time::timeout};
use uuid::Uuid;

use handshake_core::{
    process_ledger::{
        LedgerEvent, LedgerEventKind, LedgerOverflowEvent, ProcessEngineKind, ProcessLedgerError,
        ProcessLedgerOverflowSink, ProcessLedgerStore, ProcessLedgerWriter, ProcessStart,
        ProcessStop,
    },
    test_harness::{
        HbrSwarmInvariantFail, HbrSwarmLoopCounter, SessionStep, SwarmHarness, SwarmScenario,
        FR_EVT_LOOP_CAP, HBR_SWARM_002_LOOP_CAP, HBR_SWARM_INVARIANT_FAIL, SHARED_LEASE_RESOURCE,
    },
};

const REPORT_SCHEMA_ID: &str = "hsk.swarm_invariants_evidence@1";
const WP_ID: &str =
    "WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn swarm_invariants_tests_validate_all_four_ac_swarm_harness_invariants(
) -> Result<(), Box<dyn Error>> {
    let lease = run_lease_contention_invariant().await?;
    assert_invariant(
        lease.grants_completed == LEASE_SESSIONS * LEASE_GRANTS_PER_SESSION
            && lease.max_simultaneous_holders == 1
            && lease.unique_grants == LEASE_SESSIONS,
        "lock_lease_contention",
        json!(&lease),
    )?;

    let cancellation = run_cancellation_invariant().await?;
    assert_invariant(
        cancellation.cancelled_sessions == CANCEL_SESSIONS
            && cancellation.max_propagation_ms <= 2_000,
        "cancellation_propagation",
        json!(&cancellation),
    )?;

    let loop_counter = run_loop_counter_invariant();
    assert_invariant(
        loop_counter.receipt_emitted
            && loop_counter.terminated
            && loop_counter.iterations == HBR_SWARM_002_LOOP_CAP,
        "loop_counter_cap",
        json!(&loop_counter),
    )?;

    let process_ledger = run_process_ledger_consistency_invariant().await?;
    assert_invariant(
        process_ledger.sessions == 8
            && process_ledger.processes_per_session == 10
            && process_ledger.start_rows == 80
            && process_ledger.stop_rows == 80
            && process_ledger.duplicate_process_uuid_count == 0
            && process_ledger.missing_stop_count == 0
            && process_ledger.wrong_session_correlation_count == 0
            && process_ledger.ledger_overflow_count == 0,
        "process_ledger_consistency",
        json!(&process_ledger),
    )?;

    write_report(&SwarmInvariantEvidenceRow {
        schema_id: REPORT_SCHEMA_ID.to_string(),
        wp_id: WP_ID.to_string(),
        lock_lease: lease,
        cancellation,
        loop_counter,
        process_ledger,
        failure_receipt_kind: HBR_SWARM_INVARIANT_FAIL.to_string(),
        hbr_ids: vec![
            "HBR-SWARM-001".to_string(),
            "HBR-SWARM-002".to_string(),
            "HBR-SWARM-003".to_string(),
            "HBR-SWARM-004".to_string(),
            "HBR-QUIET-003".to_string(),
        ],
    })?;

    Ok(())
}

const LEASE_SESSIONS: usize = 16;
const LEASE_GRANTS_PER_SESSION: usize = 1;
const CANCEL_SESSIONS: usize = 8;
const CANCEL_STEPS_PER_SESSION: usize = 40;
const CANCEL_AFTER_MS: u64 = 25;

/// MT-037 lock/lease invariant — REAL platform primitive.
///
/// Replaces the former in-test `tokio::sync::Semaphore` with the harness's real
/// `SharedLeaseRegistry` (backed by `tokio::sync::Mutex`). `LEASE_SESSIONS`
/// sessions concurrently dispatch a real `MutateCrdtField` step that first
/// acquires the shared exclusive lease through the real kernel action-catalog
/// dispatch path, then commits to the CRDT workspace. Exclusivity is *measured*:
/// the workspace increments a live holder count on grant and decrements it on
/// guard drop, and the high-water mark must never exceed 1.
async fn run_lease_contention_invariant() -> Result<LeaseEvidence, Box<dyn Error>> {
    let report = timeout(
        Duration::from_secs(10),
        SwarmHarness::new(LEASE_SESSIONS, LeaseContentionScenario).run(),
    )
    .await??;

    let crdt = &report.crdt_workspace;
    Ok(LeaseEvidence {
        sessions: LEASE_SESSIONS,
        grants_completed: crdt.lease_grants_completed,
        // Each session grants the lease exactly once, so distinct grants equals
        // the session count.
        unique_grants: report
            .sessions
            .iter()
            .filter(|session| session.steps_completed == LEASE_GRANTS_PER_SESSION)
            .count(),
        max_simultaneous_holders: crdt.max_simultaneous_lease_holders,
    })
}

/// MT-037 cancellation invariant — REAL platform primitive.
///
/// Replaces the former in-test `AtomicBool` poll with the platform
/// `kernel::sandbox::CancellationToken`, flipped mid-run by the harness watcher
/// (`cancel_after_ms`). Each session issues real `MutateCrdtField` steps marked
/// `cancellable`; in-flight steps observe the real token and abort, recording a
/// real cancellation on the shared workspace. The number of cancelled sessions
/// and the worst-case propagation are *measured* from the run.
async fn run_cancellation_invariant() -> Result<CancellationEvidence, Box<dyn Error>> {
    let started = std::time::Instant::now();
    let report = timeout(
        Duration::from_secs(10),
        SwarmHarness::new(CANCEL_SESSIONS, CancelMidMutationScenario).run(),
    )
    .await??;
    let elapsed_ms = started.elapsed().as_millis();

    // A session is "cancelled" when at least one of its cancellable steps
    // observed the real token mid-flight; the workspace reports the count of
    // distinct sessions that recorded a real cancellation.
    let cancelled_sessions = report.crdt_workspace.distinct_cancelled_sessions;

    Ok(CancellationEvidence {
        sessions: CANCEL_SESSIONS,
        cancelled_sessions,
        // Real worst-case wall time from token flip to run completion.
        max_propagation_ms: elapsed_ms,
        deadline_ms: 2_000,
    })
}

/// Real lease-contention scenario: each session takes the shared exclusive lease
/// once, committing through the real CRDT dispatch path.
struct LeaseContentionScenario;

impl SwarmScenario for LeaseContentionScenario {
    fn scenario_id(&self) -> &str {
        "lease-contention-invariant"
    }

    fn session_steps(&self, session_idx: usize) -> Vec<SessionStep> {
        vec![SessionStep::MutateCrdtField {
            action_id: "kernel.crdt_workspace.propose_patch".to_string(),
            field_id: format!("lease-field-{session_idx}"),
            lease_resource: Some(SHARED_LEASE_RESOURCE.to_string()),
            cancellable: false,
        }]
    }
}

/// Real cancel-mid-mutation scenario: each session issues many cancellable CRDT
/// mutations; the harness flips the real cancellation token mid-run.
struct CancelMidMutationScenario;

impl SwarmScenario for CancelMidMutationScenario {
    fn scenario_id(&self) -> &str {
        "cancel-mid-mutation-invariant"
    }

    fn session_steps(&self, session_idx: usize) -> Vec<SessionStep> {
        (0..CANCEL_STEPS_PER_SESSION)
            .map(|op_idx| SessionStep::MutateCrdtField {
                action_id: "kernel.crdt_workspace.propose_patch".to_string(),
                field_id: format!("cancel-field-{session_idx}-{}", op_idx % 4),
                lease_resource: None,
                cancellable: true,
            })
            .collect()
    }

    fn cancel_after_ms(&self) -> Option<u64> {
        Some(CANCEL_AFTER_MS)
    }
}

fn run_loop_counter_invariant() -> LoopCounterEvidence {
    let mut counter = HbrSwarmLoopCounter::new("stuck_precondition", HBR_SWARM_002_LOOP_CAP);
    let mut receipt = None;
    for _ in 0..=HBR_SWARM_002_LOOP_CAP {
        if let Some(emitted) = counter.tick("event_never_fired") {
            receipt = Some(emitted);
            break;
        }
    }
    let receipt = receipt.expect("loop cap receipt");

    LoopCounterEvidence {
        cap: HBR_SWARM_002_LOOP_CAP,
        iterations: receipt.iterations,
        event_type: receipt.event_type,
        receipt_emitted: true,
        terminated: counter.is_terminated(),
    }
}

async fn run_process_ledger_consistency_invariant() -> Result<ProcessLedgerEvidence, Box<dyn Error>>
{
    let store = Arc::new(InMemoryProcessLedgerStore::default());
    let overflow_sink = Arc::new(InMemoryOverflowSink::default());
    let (writer, drain) = ProcessLedgerWriter::new_manual(256, overflow_sink.clone())?;
    let writer = Arc::new(writer);
    let mut join_set = JoinSet::new();

    for session_idx in 0..8 {
        let writer = Arc::clone(&writer);
        join_set.spawn(async move {
            let session_id = format!("swarm-invariant-session-{session_idx}");
            for process_idx in 0..10 {
                let start = ProcessStart::new(
                    ProcessEngineKind::MechanicalJob,
                    "KERNEL_BUILDER",
                    Some(WP_ID.to_string()),
                )
                .with_parent_session_id(session_id.clone())
                .with_sandbox_adapter_id("swarm-invariants")
                .with_work_profile_id(format!("swarm-invariants-{session_idx}-{process_idx}"));
                writer.append_start(start.clone())?;
                tokio::task::yield_now().await;
                writer.append_stop(ProcessStop::from_start(&start, Some(0)))?;
            }
            Ok::<(), ProcessLedgerError>(())
        });
    }

    drain_process_tasks(join_set).await?;
    drain.drain_available_to(store.clone()).await?;
    let events = store.events()?;
    Ok(process_ledger_evidence(
        &events,
        overflow_sink.overflow_count()?,
    ))
}

async fn drain_process_tasks(
    mut join_set: JoinSet<Result<(), ProcessLedgerError>>,
) -> Result<(), Box<dyn Error>> {
    while let Some(joined) = join_set.join_next().await {
        joined??;
    }
    Ok(())
}

fn process_ledger_evidence(events: &[LedgerEvent], overflow_count: u64) -> ProcessLedgerEvidence {
    let mut starts_by_uuid = BTreeMap::<Uuid, Vec<&LedgerEvent>>::new();
    let mut stops_by_uuid = BTreeMap::<Uuid, Vec<&LedgerEvent>>::new();
    let mut session_process_counts = BTreeMap::<String, usize>::new();
    for event in events {
        match event.kind() {
            LedgerEventKind::Start => {
                starts_by_uuid
                    .entry(event.process_uuid())
                    .or_default()
                    .push(event);
                *session_process_counts
                    .entry(event.parent_session_id().unwrap_or_default().to_string())
                    .or_default() += 1;
            }
            LedgerEventKind::Stop => {
                stops_by_uuid
                    .entry(event.process_uuid())
                    .or_default()
                    .push(event);
            }
        }
    }

    let duplicate_process_uuid_count = starts_by_uuid
        .values()
        .filter(|rows| rows.len() != 1)
        .count()
        + stops_by_uuid
            .values()
            .filter(|rows| rows.len() != 1)
            .count();
    let missing_stop_count = starts_by_uuid
        .keys()
        .filter(|process_uuid| !stops_by_uuid.contains_key(process_uuid))
        .count();
    let wrong_session_correlation_count = events
        .iter()
        .filter(|event| {
            event
                .parent_session_id()
                .map(|session| !session.starts_with("swarm-invariant-session-"))
                .unwrap_or(true)
        })
        .count();

    ProcessLedgerEvidence {
        sessions: session_process_counts.len(),
        processes_per_session: session_process_counts.values().copied().min().unwrap_or(0),
        start_rows: starts_by_uuid.values().map(Vec::len).sum(),
        stop_rows: stops_by_uuid.values().map(Vec::len).sum(),
        duplicate_process_uuid_count,
        missing_stop_count,
        wrong_session_correlation_count,
        ledger_overflow_count: overflow_count,
    }
}

fn assert_invariant(
    condition: bool,
    invariant_id: &str,
    details: serde_json::Value,
) -> Result<(), Box<dyn Error>> {
    if condition {
        return Ok(());
    }
    let receipt = HbrSwarmInvariantFail::new(invariant_id, WP_ID, details);
    write_failure_receipt(&receipt)?;
    panic!(
        "{} invariant failed: {:?}",
        HBR_SWARM_INVARIANT_FAIL, receipt
    );
}

fn write_report(row: &SwarmInvariantEvidenceRow) -> Result<(), Box<dyn Error>> {
    append_jsonl(report_path(), row)
}

fn write_failure_receipt(receipt: &HbrSwarmInvariantFail) -> Result<(), Box<dyn Error>> {
    append_jsonl(report_path(), receipt)
}

fn append_jsonl<T>(path: PathBuf, row: &T) -> Result<(), Box<dyn Error>>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(row)?)?;
    Ok(())
}

fn report_path() -> PathBuf {
    if let Ok(value) = std::env::var("HANDSHAKE_SWARM_INVARIANTS_REPORT") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    artifact_root().join("hbr-swarm-invariants").join(format!(
        "swarm-invariants-{}-{}.jsonl",
        std::process::id(),
        epoch_millis()
    ))
}

fn artifact_root() -> PathBuf {
    if let Ok(value) = std::env::var("HANDSHAKE_ARTIFACT_ROOT") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    repo_root()
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("."))
        .join("Handshake_Artifacts")
}

fn repo_root() -> PathBuf {
    let mut current = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if current.join(".GOV").exists() {
            return current;
        }
        assert!(current.pop(), "repo root with .GOV not found");
    }
}

fn epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SwarmInvariantEvidenceRow {
    schema_id: String,
    wp_id: String,
    lock_lease: LeaseEvidence,
    cancellation: CancellationEvidence,
    loop_counter: LoopCounterEvidence,
    process_ledger: ProcessLedgerEvidence,
    failure_receipt_kind: String,
    hbr_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LeaseEvidence {
    sessions: usize,
    grants_completed: usize,
    unique_grants: usize,
    max_simultaneous_holders: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CancellationEvidence {
    sessions: usize,
    cancelled_sessions: usize,
    max_propagation_ms: u128,
    deadline_ms: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LoopCounterEvidence {
    cap: usize,
    iterations: usize,
    event_type: String,
    receipt_emitted: bool,
    terminated: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProcessLedgerEvidence {
    sessions: usize,
    processes_per_session: usize,
    start_rows: usize,
    stop_rows: usize,
    duplicate_process_uuid_count: usize,
    missing_stop_count: usize,
    wrong_session_correlation_count: usize,
    ledger_overflow_count: u64,
}

#[derive(Default)]
struct InMemoryProcessLedgerStore {
    events: Mutex<Vec<LedgerEvent>>,
}

impl InMemoryProcessLedgerStore {
    fn events(&self) -> Result<Vec<LedgerEvent>, ProcessLedgerError> {
        self.events
            .lock()
            .map(|events| events.clone())
            .map_err(|_| ProcessLedgerError::Store("process ledger store poisoned".to_string()))
    }
}

#[async_trait]
impl ProcessLedgerStore for InMemoryProcessLedgerStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        self.events
            .lock()
            .map_err(|_| ProcessLedgerError::Store("process ledger store poisoned".to_string()))?
            .extend(events);
        Ok(())
    }
}

#[derive(Default)]
struct InMemoryOverflowSink {
    overflow_count: AtomicUsize,
}

impl InMemoryOverflowSink {
    fn overflow_count(&self) -> Result<u64, ProcessLedgerError> {
        Ok(self.overflow_count.load(Ordering::SeqCst) as u64)
    }
}

impl ProcessLedgerOverflowSink for InMemoryOverflowSink {
    fn emit_overflow(&self, _event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        self.overflow_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn swarm_invariant_failure_receipt_shape_is_machine_readable() {
    let receipt =
        HbrSwarmInvariantFail::new("fixture_invariant", WP_ID, json!({ "field": "value" }));
    let encoded = serde_json::to_value(receipt).expect("receipt serializes");

    assert_eq!(encoded["receipt_kind"], HBR_SWARM_INVARIANT_FAIL);
    assert_eq!(encoded["wp_id"], WP_ID);
    assert_eq!(encoded["invariant_id"], "fixture_invariant");
    assert!(encoded["receipt_uuid"].as_str().is_some());
    assert_eq!(FR_EVT_LOOP_CAP, "FR-EVT-LOOP-CAP");
}
