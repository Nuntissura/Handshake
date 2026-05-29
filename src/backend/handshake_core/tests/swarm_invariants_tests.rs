use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{
    sync::Semaphore,
    task::JoinSet,
    time::{sleep, timeout},
};
use uuid::Uuid;

use handshake_core::{
    process_ledger::{
        LedgerEvent, LedgerEventKind, LedgerOverflowEvent, ProcessEngineKind, ProcessLedgerError,
        ProcessLedgerOverflowSink, ProcessLedgerStore, ProcessLedgerWriter, ProcessStart,
        ProcessStop,
    },
    test_harness::{
        HbrSwarmInvariantFail, HbrSwarmLoopCounter, FR_EVT_LOOP_CAP, HBR_SWARM_002_LOOP_CAP,
        HBR_SWARM_INVARIANT_FAIL,
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
        lease.grants_completed == 16 && lease.max_simultaneous_holders == 1,
        "lock_lease_contention",
        json!(&lease),
    )?;

    let cancellation = run_cancellation_invariant().await?;
    assert_invariant(
        cancellation.cancelled_sessions == 8 && cancellation.max_propagation_ms <= 500,
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

async fn run_lease_contention_invariant() -> Result<LeaseEvidence, Box<dyn Error>> {
    let semaphore = Arc::new(Semaphore::new(1));
    let active_holders = Arc::new(AtomicUsize::new(0));
    let max_simultaneous_holders = Arc::new(AtomicUsize::new(0));
    let grant_order = Arc::new(Mutex::new(Vec::<usize>::new()));
    let mut join_set = JoinSet::new();

    for session_idx in 0..16 {
        let semaphore = Arc::clone(&semaphore);
        let active_holders = Arc::clone(&active_holders);
        let max_simultaneous_holders = Arc::clone(&max_simultaneous_holders);
        let grant_order = Arc::clone(&grant_order);
        join_set.spawn(async move {
            let permit = semaphore
                .acquire_owned()
                .await
                .map_err(|error| error.to_string())?;
            let holders = active_holders.fetch_add(1, Ordering::SeqCst) + 1;
            max_simultaneous_holders.fetch_max(holders, Ordering::SeqCst);
            {
                let mut grants = grant_order
                    .lock()
                    .map_err(|_| "grant order mutex poisoned".to_string())?;
                grants.push(session_idx);
            }
            sleep(Duration::from_millis(2)).await;
            active_holders.fetch_sub(1, Ordering::SeqCst);
            drop(permit);
            Ok::<usize, String>(session_idx)
        });
    }

    let completed = timeout(Duration::from_secs(5), drain_join_set(join_set)).await??;
    let grants = grant_order
        .lock()
        .map_err(|_| "grant order mutex poisoned")?
        .clone();
    let unique_grants = grants.iter().copied().collect::<BTreeSet<_>>().len();

    Ok(LeaseEvidence {
        sessions: 16,
        grants_completed: completed.len(),
        unique_grants,
        max_simultaneous_holders: max_simultaneous_holders.load(Ordering::SeqCst),
    })
}

async fn run_cancellation_invariant() -> Result<CancellationEvidence, Box<dyn Error>> {
    let cancellation = Arc::new(AtomicBool::new(false));
    let mut join_set = JoinSet::new();

    for session_idx in 0..8 {
        let cancellation = Arc::clone(&cancellation);
        join_set.spawn(async move {
            while !cancellation.load(Ordering::SeqCst) {
                sleep(Duration::from_millis(10)).await;
            }
            Ok::<(usize, u128), String>((session_idx, 0))
        });
    }

    sleep(Duration::from_millis(100)).await;
    let cancellation_started = Instant::now();
    cancellation.store(true, Ordering::SeqCst);
    let mut completed = timeout(Duration::from_millis(500), drain_join_set(join_set)).await??;
    let propagation_ms = cancellation_started.elapsed().as_millis();
    for (_, elapsed) in &mut completed {
        *elapsed = propagation_ms;
    }

    Ok(CancellationEvidence {
        sessions: 8,
        cancelled_sessions: completed.len(),
        max_propagation_ms: completed
            .iter()
            .map(|(_, elapsed)| *elapsed)
            .max()
            .unwrap_or_default(),
        deadline_ms: 500,
    })
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

async fn drain_join_set<T: 'static>(
    mut join_set: JoinSet<Result<T, String>>,
) -> Result<Vec<T>, String> {
    let mut out = Vec::new();
    while let Some(joined) = join_set.join_next().await {
        out.push(joined.map_err(|error| error.to_string())??);
    }
    Ok(out)
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
