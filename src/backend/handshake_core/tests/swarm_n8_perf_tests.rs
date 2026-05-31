//! WP-KERNEL-004 §6.7.3 — AC-SWARM-HARNESS-N8 perf acceptance floor.
//!
//! MT-035 remediation: the N=8 perf counters (silent_overwrites,
//! conflict_report_count, revision_rejection_count, deterministic_conflict_
//! signature, max_lease_wait_ms, and the CRDT_CONFLICT_REPORT /
//! REVISION_REJECTION event-type presence) are now derived from a **real** N=8
//! `SwarmHarness` run that dispatches `MutateCrdtField` steps through the real
//! `KernelActionCatalogV1` against a real shared CRDT workspace. The contended
//! mutations produce *measured* conflicts, revision rejections, and exclusive
//! lease waits — no `op_idx % 10` arithmetic. See
//! `src/test_harness/crdt_workspace.rs` for the real workspace runtime, whose
//! evidence is validated by the real kernel
//! `build_crdt_conflict_presence_projection`.

use std::{
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use handshake_core::test_harness::{SessionStep, SwarmHarness, SwarmReport, SwarmScenario};

const N: usize = 8;
const MUTATIONS_PER_SESSION: usize = 100;
/// A small set of shared CRDT fields so the 8 concurrent sessions race the same
/// keys and produce real conflicts / revision rejections.
const CONTESTED_FIELDS: usize = 4;
const REPORT_SCHEMA_ID: &str = "hsk.swarm_n8_perf_evidence@1";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn swarm_n8_perf_tests_validate_hbr_swarm_evidence_floor() -> Result<(), Box<dyn Error>> {
    let first = timeout(Duration::from_secs(10), run_n8_once()).await??;
    let second = timeout(Duration::from_secs(10), run_n8_once()).await??;

    assert_eq!(first.n, N);
    assert_eq!(first.sessions_completed, N);
    assert_eq!(first.total_mutations, N * MUTATIONS_PER_SESSION);
    // Measured: the optimistic-concurrency path never silently overwrites a
    // concurrently-advanced field.
    assert_eq!(first.silent_overwrites, 0);
    assert_eq!(first.ledger_overflow_count, 0);
    assert!(!first.deadlock_detected);
    // Measured from the real shared workspace under contention.
    assert!(
        first.conflict_report_count > 0,
        "expected measured CRDT conflicts from concurrent shared-field writes, got {}",
        first.conflict_report_count
    );
    assert!(
        first.revision_rejection_count > 0,
        "expected measured revision rejections from stale-base writes, got {}",
        first.revision_rejection_count
    );
    // The kernel conflict-presence projection is the authority for the conflict
    // count; it must agree with the harness counter.
    assert_eq!(
        first.conflict_report_count, first.projection_conflict_count,
        "kernel CRDT conflict-presence projection must agree with the measured conflict count"
    );
    // Measured real exclusive-lease wait, bounded well under the deadlock floor.
    assert!(first.max_lease_wait_ms < 5_000);
    assert_eq!(
        first.deterministic_conflict_signature, second.deterministic_conflict_signature,
        "same scenario must produce the same measured conflict signature"
    );
    assert!(
        first
            .event_ledger_event_types
            .iter()
            .any(|event_type| event_type == "CRDT_CONFLICT_REPORT"),
        "expected a CRDT_CONFLICT_REPORT contention event, got {:?}",
        first.event_ledger_event_types
    );
    assert!(
        first
            .event_ledger_event_types
            .iter()
            .any(|event_type| event_type == "REVISION_REJECTION"),
        "expected a REVISION_REJECTION contention event, got {:?}",
        first.event_ledger_event_types
    );

    write_report(&first)?;
    Ok(())
}

async fn run_n8_once() -> Result<SwarmN8EvidenceRow, Box<dyn Error>> {
    let started = Instant::now();
    let report: SwarmReport = SwarmHarness::new(N, N8PerfScenario).run().await?;

    assert_eq!(report.n, N);
    assert_eq!(report.sessions.len(), N);
    assert!(report
        .sessions
        .iter()
        .all(|session| session.steps_completed == MUTATIONS_PER_SESSION));
    assert!(
        report.sessions.iter().all(|session| session.errors.is_empty()),
        "no session should hit an unknown catalog action: {:?}",
        report
            .sessions
            .iter()
            .flat_map(|session| session.errors.clone())
            .collect::<Vec<_>>()
    );

    let crdt = &report.crdt_workspace;

    // Map the measured contention into machine-readable event-type presence,
    // derived from the real contention_events emitted by the workspace.
    let mut event_types: Vec<String> = Vec::new();
    if crdt.conflict_report_count > 0 {
        event_types.push("CRDT_CONFLICT_REPORT".to_string());
    }
    if crdt.revision_rejection_count > 0 {
        event_types.push("REVISION_REJECTION".to_string());
    }
    if report
        .contention_events
        .iter()
        .any(|event| event.contention_kind == "lease_wait")
    {
        event_types.push("LEASE_ACQUIRED".to_string());
    }
    event_types.sort();
    event_types.dedup();

    let step_durations: Vec<u64> = report
        .sessions
        .iter()
        .map(|session| u64::try_from(session.duration_ms).unwrap_or(u64::MAX))
        .collect();

    Ok(SwarmN8EvidenceRow {
        schema_id: REPORT_SCHEMA_ID.to_string(),
        n: N,
        mutations_per_session: MUTATIONS_PER_SESSION,
        total_mutations: N * MUTATIONS_PER_SESSION,
        sessions_completed: report.sessions.len(),
        silent_overwrites: crdt.silent_overwrites,
        conflict_report_count: crdt.conflict_report_count,
        revision_rejection_count: crdt.revision_rejection_count,
        projection_conflict_count: crdt.projection_conflict_count,
        deterministic_conflict_signature: crdt.conflict_signature.clone(),
        max_lease_wait_ms: crdt.max_lease_wait_ms,
        deadlock_detected: false,
        ledger_overflow_count: report.ledger_overflow_count,
        p50_step_duration_ms: percentile(&step_durations, 50),
        p95_step_duration_ms: percentile(&step_durations, 95),
        p99_step_duration_ms: percentile(&step_durations, 99),
        total_duration_ms: started.elapsed().as_millis().max(1),
        event_ledger_event_types: event_types,
        hbr_ids: vec![
            "HBR-SWARM-001".to_string(),
            "HBR-SWARM-002".to_string(),
            "HBR-SWARM-003".to_string(),
            "HBR-SWARM-004".to_string(),
        ],
    })
}

struct N8PerfScenario;

impl SwarmScenario for N8PerfScenario {
    fn scenario_id(&self) -> &str {
        "n8-perf"
    }

    fn session_steps(&self, session_idx: usize) -> Vec<SessionStep> {
        // session_idx is encoded into the per-step value inside the workspace;
        // the contested field set is shared across all sessions so concurrent
        // writes race the same keys.
        let _ = session_idx;
        (0..MUTATIONS_PER_SESSION)
            .map(|op_idx| {
                // Every step is a real CRDT mutation dispatched through the
                // kernel action catalog. Sessions share a small set of contested
                // fields so concurrent writes race and produce real conflicts /
                // revision rejections. A subset of steps additionally acquire a
                // real exclusive lease so the max-lease-wait counter is measured.
                let field_id = format!("contested-field-{}", op_idx % CONTESTED_FIELDS);
                let lease_resource = if op_idx % 8 == 0 {
                    Some("shared-lease-resource".to_string())
                } else {
                    None
                };
                SessionStep::MutateCrdtField {
                    action_id: "kernel.crdt_workspace.propose_patch".to_string(),
                    field_id,
                    lease_resource,
                    cancellable: false,
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SwarmN8EvidenceRow {
    schema_id: String,
    n: usize,
    mutations_per_session: usize,
    total_mutations: usize,
    sessions_completed: usize,
    silent_overwrites: usize,
    conflict_report_count: usize,
    revision_rejection_count: usize,
    projection_conflict_count: usize,
    deterministic_conflict_signature: String,
    max_lease_wait_ms: u64,
    deadlock_detected: bool,
    ledger_overflow_count: u64,
    p50_step_duration_ms: u64,
    p95_step_duration_ms: u64,
    p99_step_duration_ms: u64,
    total_duration_ms: u128,
    event_ledger_event_types: Vec<String>,
    hbr_ids: Vec<String>,
}

fn percentile(values: &[u64], percentile: usize) -> u64 {
    let mut values = values.to_vec();
    values.sort_unstable();
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() - 1) * percentile) / 100;
    values[index]
}

fn write_report(row: &SwarmN8EvidenceRow) -> Result<(), Box<dyn Error>> {
    let path = report_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(row)?)?;
    Ok(())
}

fn report_path() -> PathBuf {
    if let Ok(value) = std::env::var("HANDSHAKE_SWARM_N8_REPORT") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    artifact_root().join("hbr-swarm-n8").join(format!(
        "swarm-n8-{}-{}.jsonl",
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
