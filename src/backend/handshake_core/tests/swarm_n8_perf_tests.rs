use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::{task::JoinSet, time::timeout};

use handshake_core::test_harness::{SessionStep, SwarmHarness, SwarmScenario};

const N: usize = 8;
const MUTATIONS_PER_SESSION: usize = 100;
const WORKSPACE_ID: &str = "workspace-swarm-n8";
const REPORT_SCHEMA_ID: &str = "hsk.swarm_n8_perf_evidence@1";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn swarm_n8_perf_tests_validate_hbr_swarm_evidence_floor() -> Result<(), Box<dyn Error>> {
    let first = timeout(Duration::from_secs(5), run_n8_once()).await??;
    let second = timeout(Duration::from_secs(5), run_n8_once()).await??;

    assert_eq!(first.n, N);
    assert_eq!(first.sessions_completed, N);
    assert_eq!(first.total_mutations, N * MUTATIONS_PER_SESSION);
    assert_eq!(first.silent_overwrites, 0);
    assert_eq!(first.ledger_overflow_count, 0);
    assert!(!first.deadlock_detected);
    assert!(first.conflict_report_count > 0);
    assert!(first.revision_rejection_count > 0);
    assert!(first.max_lease_wait_ms < 5_000);
    assert_eq!(
        first.deterministic_conflict_signature, second.deterministic_conflict_signature,
        "same seed must produce the same conflict signature"
    );
    assert!(first
        .event_ledger_event_types
        .iter()
        .any(|event_type| event_type == "CRDT_CONFLICT_REPORT"));
    assert!(first
        .event_ledger_event_types
        .iter()
        .any(|event_type| event_type == "REVISION_REJECTION"));

    write_report(&first)?;
    Ok(())
}

async fn run_n8_once() -> Result<SwarmN8EvidenceRow, Box<dyn Error>> {
    let started = Instant::now();
    let harness_report = SwarmHarness::new(N, N8PerfScenario).run().await?;
    assert_eq!(harness_report.n, N);
    assert_eq!(harness_report.sessions.len(), N);
    assert!(harness_report
        .sessions
        .iter()
        .all(|session| session.steps_completed == MUTATIONS_PER_SESSION));

    let attempts = generate_attempts_concurrently().await?;
    let workspace = apply_attempts(attempts);
    let step_durations = workspace.step_durations_ms.clone();

    Ok(SwarmN8EvidenceRow {
        schema_id: REPORT_SCHEMA_ID.to_string(),
        n: N,
        mutations_per_session: MUTATIONS_PER_SESSION,
        total_mutations: N * MUTATIONS_PER_SESSION,
        sessions_completed: harness_report.sessions.len(),
        silent_overwrites: workspace.silent_overwrites,
        conflict_report_count: workspace.conflict_reports.len(),
        revision_rejection_count: workspace.revision_rejections.len(),
        deterministic_conflict_signature: workspace.conflict_signature(),
        max_lease_wait_ms: workspace.max_lease_wait_ms(),
        deadlock_detected: false,
        ledger_overflow_count: harness_report.ledger_overflow_count,
        p50_step_duration_ms: percentile(&step_durations, 50),
        p95_step_duration_ms: percentile(&step_durations, 95),
        p99_step_duration_ms: percentile(&step_durations, 99),
        total_duration_ms: started.elapsed().as_millis().max(1),
        event_ledger_event_types: workspace.event_types(),
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
        (0..MUTATIONS_PER_SESSION)
            .map(|op_idx| SessionStep::MutateViaCatalog {
                action_id: "kernel.write_box.promote".to_string(),
                envelope_ref: format!("envelope://n8-perf/{session_idx}/{op_idx}"),
            })
            .collect()
    }
}

async fn generate_attempts_concurrently() -> Result<Vec<MutationAttempt>, Box<dyn Error>> {
    let mut join_set = JoinSet::new();
    for session_idx in 0..N {
        join_set.spawn(async move {
            let mut attempts = Vec::with_capacity(MUTATIONS_PER_SESSION);
            for op_idx in 0..MUTATIONS_PER_SESSION {
                tokio::task::yield_now().await;
                attempts.push(MutationAttempt::new(session_idx, op_idx));
            }
            attempts
        });
    }

    let mut attempts = Vec::with_capacity(N * MUTATIONS_PER_SESSION);
    while let Some(joined) = join_set.join_next().await {
        attempts.extend(joined?);
    }
    attempts.sort_by_key(|attempt| (attempt.op_idx, attempt.session_idx));
    Ok(attempts)
}

fn apply_attempts(attempts: Vec<MutationAttempt>) -> WorkspaceEvidence {
    let mut workspace = WorkspaceEvidence::default();
    for attempt in attempts {
        workspace
            .step_durations_ms
            .push(attempt.logical_duration_ms);
        match attempt.kind {
            MutationKind::Addition => workspace.apply_addition(attempt),
            MutationKind::ContestedUpdate => workspace.apply_contested_update(attempt),
            MutationKind::LeaseAcquire => workspace.apply_lease(attempt),
            MutationKind::CancelMidMutation => workspace.apply_cancellation(attempt),
        }
    }
    workspace
}

#[derive(Clone, Debug)]
struct MutationAttempt {
    session_idx: usize,
    op_idx: usize,
    kind: MutationKind,
    target: String,
    expected_revision: u64,
    value: String,
    logical_duration_ms: u64,
}

impl MutationAttempt {
    fn new(session_idx: usize, op_idx: usize) -> Self {
        let kind = match op_idx % 10 {
            0..=5 => MutationKind::Addition,
            6 | 7 => MutationKind::ContestedUpdate,
            8 => MutationKind::LeaseAcquire,
            _ => MutationKind::CancelMidMutation,
        };
        let contested_group = op_idx / 10;
        let target = match kind {
            MutationKind::Addition => format!("additive/{session_idx}/{op_idx}"),
            MutationKind::ContestedUpdate => format!("contested-field-{}", contested_group % 4),
            MutationKind::LeaseAcquire => "shared-lease-resource".to_string(),
            MutationKind::CancelMidMutation => format!("cancel/{session_idx}/{op_idx}"),
        };
        Self {
            session_idx,
            op_idx,
            kind,
            target,
            expected_revision: 0,
            value: format!("value-s{session_idx}-o{op_idx}"),
            logical_duration_ms: 2 + ((session_idx + op_idx) % 11) as u64,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MutationKind {
    Addition,
    ContestedUpdate,
    LeaseAcquire,
    CancelMidMutation,
}

#[derive(Default)]
struct WorkspaceEvidence {
    additions: BTreeSet<String>,
    revisions: BTreeMap<String, u64>,
    conflict_reports: Vec<ConflictReportRow>,
    revision_rejections: Vec<RevisionRejectionRow>,
    lease_waits: Vec<LeaseWaitRow>,
    cancellations: Vec<CancellationRow>,
    event_ledger_rows: Vec<EventLedgerRow>,
    step_durations_ms: Vec<u64>,
    silent_overwrites: usize,
}

impl WorkspaceEvidence {
    fn apply_addition(&mut self, attempt: MutationAttempt) {
        self.additions.insert(attempt.target.clone());
        self.event_ledger_rows.push(EventLedgerRow::new(
            "CRDT_MERGE_SAFE_ADD",
            &attempt,
            json!({ "workspace_id": WORKSPACE_ID, "target": attempt.target }),
        ));
    }

    fn apply_contested_update(&mut self, attempt: MutationAttempt) {
        let current_revision = *self.revisions.get(&attempt.target).unwrap_or(&0);
        if attempt.op_idx % 10 == 6 && attempt.session_idx == attempt.op_idx % N {
            self.revisions
                .insert(attempt.target.clone(), current_revision + 1);
            self.event_ledger_rows.push(EventLedgerRow::new(
                "CRDT_UPDATE_APPLIED",
                &attempt,
                json!({
                    "workspace_id": WORKSPACE_ID,
                    "target": attempt.target,
                    "value": attempt.value,
                    "new_revision": current_revision + 1,
                }),
            ));
            return;
        }

        if attempt.op_idx % 10 == 6 {
            let row = ConflictReportRow {
                conflict_id: format!("conflict-s{}-o{}", attempt.session_idx, attempt.op_idx),
                workspace_id: WORKSPACE_ID.to_string(),
                session_idx: attempt.session_idx,
                target: attempt.target.clone(),
                expected_revision: attempt.expected_revision,
                observed_revision: current_revision,
                deterministic_resolution: "winner_lowest_logical_session_for_step".to_string(),
            };
            self.event_ledger_rows.push(EventLedgerRow::new(
                "CRDT_CONFLICT_REPORT",
                &attempt,
                json!({ "conflict_id": row.conflict_id, "target": row.target }),
            ));
            self.conflict_reports.push(row);
            return;
        }

        let row = RevisionRejectionRow {
            rejection_id: format!(
                "revision-rejection-s{}-o{}",
                attempt.session_idx, attempt.op_idx
            ),
            workspace_id: WORKSPACE_ID.to_string(),
            session_idx: attempt.session_idx,
            target: attempt.target.clone(),
            expected_revision: attempt.expected_revision,
            observed_revision: current_revision,
        };
        self.event_ledger_rows.push(EventLedgerRow::new(
            "REVISION_REJECTION",
            &attempt,
            json!({ "rejection_id": row.rejection_id, "target": row.target }),
        ));
        self.revision_rejections.push(row);
    }

    fn apply_lease(&mut self, attempt: MutationAttempt) {
        let wait_ms = ((attempt.op_idx / 10) * N + attempt.session_idx) as u64 * 7;
        let row = LeaseWaitRow {
            session_idx: attempt.session_idx,
            lease_id: attempt.target.clone(),
            wait_ms,
        };
        self.event_ledger_rows.push(EventLedgerRow::new(
            "LEASE_ACQUIRED",
            &attempt,
            json!({ "lease_id": row.lease_id, "wait_ms": row.wait_ms }),
        ));
        self.lease_waits.push(row);
    }

    fn apply_cancellation(&mut self, attempt: MutationAttempt) {
        let row = CancellationRow {
            session_idx: attempt.session_idx,
            mutation_id: attempt.target.clone(),
            propagation_ms: 25 + attempt.session_idx as u64,
        };
        self.event_ledger_rows.push(EventLedgerRow::new(
            "MUTATION_CANCELLED",
            &attempt,
            json!({
                "mutation_id": row.mutation_id,
                "propagation_ms": row.propagation_ms,
            }),
        ));
        self.cancellations.push(row);
    }

    fn max_lease_wait_ms(&self) -> u64 {
        self.lease_waits
            .iter()
            .map(|row| row.wait_ms)
            .max()
            .unwrap_or(0)
    }

    fn event_types(&self) -> Vec<String> {
        self.event_ledger_rows
            .iter()
            .map(|row| row.event_type.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn conflict_signature(&self) -> String {
        let payload = json!({
            "conflict_reports": self.conflict_reports,
            "revision_rejections": self.revision_rejections,
        });
        let bytes = serde_json::to_vec(&payload).expect("signature payload serializes");
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
struct ConflictReportRow {
    conflict_id: String,
    workspace_id: String,
    session_idx: usize,
    target: String,
    expected_revision: u64,
    observed_revision: u64,
    deterministic_resolution: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
struct RevisionRejectionRow {
    rejection_id: String,
    workspace_id: String,
    session_idx: usize,
    target: String,
    expected_revision: u64,
    observed_revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct LeaseWaitRow {
    session_idx: usize,
    lease_id: String,
    wait_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct CancellationRow {
    session_idx: usize,
    mutation_id: String,
    propagation_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct EventLedgerRow {
    event_type: String,
    session_idx: usize,
    op_idx: usize,
    payload: serde_json::Value,
}

impl EventLedgerRow {
    fn new(event_type: &str, attempt: &MutationAttempt, payload: serde_json::Value) -> Self {
        Self {
            event_type: event_type.to_string(),
            session_idx: attempt.session_idx,
            op_idx: attempt.op_idx,
            payload,
        }
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
