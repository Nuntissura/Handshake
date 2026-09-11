//! Product-side schema for `hsk.surreal_swarm_load_report@1` (MT-142 Lane A1,
//! AC-142-5 / AC-142-6): the machine-readable load report the swarm tests
//! fill and the user manual documents.
//!
//! Research basis:
//! `Handshake_Artifacts/WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1/MT-142/kb01/research/research_basis.json`
//! (`selected_design.retry_policy.observability` for the counters,
//! `numeric_budgets.ci_profile.required_nonzero_classes` for
//! [`REQUIRED_OPERATION_CLASSES`], `validation_plan.remote_seam` for
//! [`RemoteProofStatus`], `numeric_budgets.machine_context` for
//! [`MachineContext`]).
//!
//! Rules encoded by [`SwarmLoadReport::validate`]:
//! * a class with zero samples is [`PercentileReport::NotRun`], never zeros;
//! * every [`Rate`] carries its numerator and a non-zero denominator;
//! * every required operation class is present in `operation_mix` and either
//!   has `attempted > 0` or is explicitly [`OperationRunStatus::NotRun`];
//! * no string carries a user-profile path or a credential-looking token.
//!
//! Visibility: `pub` because the MT-142 `tests/` swarm target builds and
//! emits this report and `user_manual` documents its field vocabulary.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Schema identifier carried in `schema_id`.
pub const SWARM_LOAD_REPORT_SCHEMA_ID: &str = "hsk.surreal_swarm_load_report@1";

/// Operation classes every CI run must exercise (research basis
/// `numeric_budgets.ci_profile.required_nonzero_classes`).
pub const REQUIRED_OPERATION_CLASSES: [OperationClass; 7] = [
    OperationClass::PointRead,
    OperationClass::RangeOrSearchQuery,
    OperationClass::Create,
    OperationClass::IdempotentUpsert,
    OperationClass::OptimisticVersionedUpdate,
    OperationClass::Delete,
    OperationClass::MultiRecordProjectionOrLedgerTransaction,
];

const FORBIDDEN_PATH_FRAGMENTS: [&str; 4] = ["C:\\Users", "C:/Users", "/home/", "/Users/"];
const FORBIDDEN_SECRET_FRAGMENTS: [&str; 2] = ["password", "token="];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationClass {
    PointRead,
    RangeOrSearchQuery,
    Create,
    IdempotentUpsert,
    OptimisticVersionedUpdate,
    Delete,
    MultiRecordProjectionOrLedgerTransaction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    /// Unclassified error: a genuine correctness or infrastructure failure.
    Terminal,
    RetryExhausted,
    Timeout,
    Cancelled,
    LockWaitTimeout,
    /// Expected loser of a typed compare-and-set race, idempotency divergence
    /// or title race (review R2-1-8): contention, not a correctness failure.
    ExpectedStaleOrConflict,
}

/// Anti-serialization metric (review R2-1-3): summed per-operation latency over
/// the run's wall clock. A ratio near 1 means the workers were globally
/// serialized regardless of any call-boundary gauge; a ratio approaching the
/// worker count means the engine overlapped them.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EffectiveParallelism {
    pub sum_operation_latency_ms: f64,
    pub wall_clock_ms: u64,
    pub ratio: f64,
}

impl EffectiveParallelism {
    /// `None` when `wall_clock_ms` is zero or the sum is not finite.
    pub fn new(sum_operation_latency_ms: f64, wall_clock_ms: u64) -> Option<Self> {
        if wall_clock_ms == 0 || !sum_operation_latency_ms.is_finite() {
            return None;
        }
        Some(Self {
            sum_operation_latency_ms,
            wall_clock_ms,
            ratio: sum_operation_latency_ms / wall_clock_ms as f64,
        })
    }
}

/// Provenance label of a recorded budget (research basis labels).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetLabel {
    Assumption,
    Measured,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LatencyBudget {
    pub p99_max_ms: f64,
    pub label: BudgetLabel,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConflictRateBudget {
    pub max_rate: f64,
    pub label: BudgetLabel,
}

/// Recorded (non-gating) numeric budgets (review R2-1-7) so a load-budget
/// regression is readable from the JSON.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoadBudgets {
    pub per_operation_timeout_ms: u64,
    pub per_worker_timeout_ms: u64,
    pub whole_test_timeout_ms: u64,
    pub latency_p99_budget_ms_by_operation: Option<BTreeMap<OperationClass, LatencyBudget>>,
    pub conflict_rate_budget: Option<ConflictRateBudget>,
}

/// Derived by [`SwarmLoadReport::evaluate_budgets`]; independent of
/// `integrity_verdict`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetVerdict {
    Pass,
    Regression,
    NotConfigured,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineMode {
    /// `Surreal::new::<RocksDb>` in-process (the only shipped mode).
    EmbeddedRocksDb,
    /// A configured remote endpoint (never the default).
    Remote,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationRunStatus {
    Run,
    NotRun,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityVerdict {
    Pass,
    LostWrite,
    DuplicateEffect,
    PartialCommit,
    DirtyRead,
    RetryExhausted,
    Timeout,
    Cancelled,
    NotRun,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteProofStatus {
    NotRunUnconfigured,
    Pass,
    Fail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreDriveKind {
    Hdd,
    Ssd,
    Unknown,
}

/// Nearest-rank percentiles over a non-empty sample set.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Percentiles {
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub sample_count: u64,
}

/// Percentiles or an explicit "no samples" marker.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PercentileReport {
    Measured(Percentiles),
    NotRun,
}

/// A ratio that keeps its numerator and denominator.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rate {
    pub numerator: u64,
    pub denominator: u64,
    pub rate: f64,
}

impl Rate {
    /// `None` when `denominator` is zero.
    pub fn new(numerator: u64, denominator: u64) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        Some(Self {
            numerator,
            denominator,
            rate: numerator as f64 / denominator as f64,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetCardinality {
    pub records: u64,
    pub workspaces: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct OperationMixEntry {
    /// Share of the workload assigned to the class.
    pub share: f64,
    pub status: OperationRunStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrityEntry {
    pub row_count: u64,
    pub content_hash: String,
}

/// Measured host facts; never paths or credentials.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineContext {
    pub cpu_model: String,
    pub logical_cpus: u32,
    pub total_memory_bytes: u64,
    pub store_drive_kind: StoreDriveKind,
    pub os: String,
}

/// `hsk.surreal_swarm_load_report@1`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SwarmLoadReport {
    pub schema_id: String,
    pub run_id: String,
    pub source_commit: String,
    pub surrealdb_version: String,
    pub sdk_version: String,
    pub engine_mode: EngineMode,
    pub workload_seed: u64,
    pub worker_count: u32,
    pub operation_count: u64,
    pub dataset_cardinality: DatasetCardinality,
    pub operation_mix: BTreeMap<OperationClass, OperationMixEntry>,
    pub contention_ratio: f64,
    pub attempted_by_operation: BTreeMap<OperationClass, u64>,
    pub succeeded_by_operation: BTreeMap<OperationClass, u64>,
    pub failed_by_operation_and_class: BTreeMap<OperationClass, BTreeMap<FailureClass, u64>>,
    pub conflict_count: u64,
    pub conflict_rate: Rate,
    pub retry_count: u64,
    pub retry_rate: Rate,
    pub retry_exhaustion_count: u64,
    pub lock_wait_ms_p50_p95_p99: PercentileReport,
    pub latency_ms_p50_p95_p99_by_operation: BTreeMap<OperationClass, PercentileReport>,
    pub throughput_operations_per_second: f64,
    pub maximum_concurrent_operations: u32,
    pub timeout_count: u64,
    pub cancellation_count: u64,
    pub shutdown_elapsed_ms: u64,
    /// Keyed by table name.
    pub reopen_integrity_counts_and_hashes: BTreeMap<String, IntegrityEntry>,
    pub integrity_verdict: IntegrityVerdict,
    pub remote_proof_status: RemoteProofStatus,
    pub machine_context: MachineContext,
    pub effective_parallelism: EffectiveParallelism,
    pub budgets: LoadBudgets,
    pub budget_verdict: BudgetVerdict,
}

impl SwarmLoadReport {
    /// Compares measured p99 latencies and the conflict rate against the
    /// recorded budgets. `NotConfigured` when no budget is recorded.
    pub fn evaluate_budgets(&self) -> BudgetVerdict {
        let latency = self.budgets.latency_p99_budget_ms_by_operation.as_ref();
        let conflict = self.budgets.conflict_rate_budget.as_ref();
        if latency.is_none() && conflict.is_none() {
            return BudgetVerdict::NotConfigured;
        }
        let latency_regression = latency.is_some_and(|budgets| {
            budgets.iter().any(|(class, budget)| {
                matches!(
                    self.latency_ms_p50_p95_p99_by_operation.get(class),
                    Some(PercentileReport::Measured(measured)) if measured.p99_ms > budget.p99_max_ms
                )
            })
        });
        let conflict_regression =
            conflict.is_some_and(|budget| self.conflict_rate.rate > budget.max_rate);
        if latency_regression || conflict_regression {
            BudgetVerdict::Regression
        } else {
            BudgetVerdict::Pass
        }
    }

    /// Every problem found, or `Ok` when the report is well-formed.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut problems = Vec::new();
        self.check_parallelism_and_budgets(&mut problems);
        if self.schema_id != SWARM_LOAD_REPORT_SCHEMA_ID {
            problems.push(format!(
                "schema_id must be {SWARM_LOAD_REPORT_SCHEMA_ID}, found {}",
                self.schema_id
            ));
        }
        if self.worker_count == 0 {
            problems.push("worker_count must be > 0".to_string());
        }
        if self.operation_count == 0 {
            problems.push("operation_count must be > 0".to_string());
        }
        if !(0.0..=1.0).contains(&self.contention_ratio) {
            problems.push(format!(
                "contention_ratio must be within [0, 1], found {}",
                self.contention_ratio
            ));
        }
        check_percentiles(
            "lock_wait_ms_p50_p95_p99",
            &self.lock_wait_ms_p50_p95_p99,
            &mut problems,
        );
        for (class, report) in &self.latency_ms_p50_p95_p99_by_operation {
            check_percentiles(
                &format!("latency_ms_p50_p95_p99_by_operation[{class:?}]"),
                report,
                &mut problems,
            );
        }
        check_rate("conflict_rate", &self.conflict_rate, &mut problems);
        check_rate("retry_rate", &self.retry_rate, &mut problems);
        self.check_operation_classes(&mut problems);
        self.check_verdicts(&mut problems);
        self.check_strings(&mut problems);
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems)
        }
    }

    fn check_operation_classes(&self, problems: &mut Vec<String>) {
        for class in REQUIRED_OPERATION_CLASSES {
            match self.operation_mix.get(&class) {
                None => problems.push(format!("operation_mix is missing required class {class:?}")),
                Some(entry) if entry.status == OperationRunStatus::Run => {
                    let attempted = self.attempted_by_operation.get(&class).copied().unwrap_or(0);
                    if attempted == 0 {
                        problems.push(format!(
                            "operation class {class:?} is marked run but attempted 0 operations"
                        ));
                    }
                }
                Some(_) => {}
            }
        }
        for (class, succeeded) in &self.succeeded_by_operation {
            let attempted = self.attempted_by_operation.get(class).copied().unwrap_or(0);
            if *succeeded > attempted {
                problems.push(format!(
                    "operation class {class:?} succeeded {succeeded} > attempted {attempted}"
                ));
            }
        }
    }

    fn check_parallelism_and_budgets(&self, problems: &mut Vec<String>) {
        let parallelism = &self.effective_parallelism;
        if parallelism.wall_clock_ms == 0 {
            problems.push("effective_parallelism wall_clock_ms must be > 0".to_string());
        } else {
            let expected = parallelism.sum_operation_latency_ms / parallelism.wall_clock_ms as f64;
            if !parallelism.sum_operation_latency_ms.is_finite()
                || parallelism.sum_operation_latency_ms < 0.0
                || !parallelism.ratio.is_finite()
                || (parallelism.ratio - expected).abs() > 1e-9
            {
                problems.push(format!(
                    "effective_parallelism ratio {} does not equal sum/wall {expected}",
                    parallelism.ratio
                ));
            }
        }
        for (name, value) in [
            ("per_operation_timeout_ms", self.budgets.per_operation_timeout_ms),
            ("per_worker_timeout_ms", self.budgets.per_worker_timeout_ms),
            ("whole_test_timeout_ms", self.budgets.whole_test_timeout_ms),
        ] {
            if value == 0 {
                problems.push(format!("budgets.{name} must be > 0"));
            }
        }
        let derived = self.evaluate_budgets();
        if self.budget_verdict != derived {
            problems.push(format!(
                "budget_verdict {:?} does not match the recorded budgets ({derived:?})",
                self.budget_verdict
            ));
        }
    }

    fn check_verdicts(&self, problems: &mut Vec<String>) {
        if self.integrity_verdict == IntegrityVerdict::Pass
            && self.reopen_integrity_counts_and_hashes.is_empty()
        {
            problems.push(
                "integrity_verdict pass requires reopen_integrity_counts_and_hashes entries"
                    .to_string(),
            );
        }
        if self.remote_proof_status == RemoteProofStatus::Pass
            && self.engine_mode != EngineMode::Remote
        {
            problems.push("remote_proof_status pass requires engine_mode remote".to_string());
        }
    }

    fn check_strings(&self, problems: &mut Vec<String>) {
        match serde_json::to_value(self) {
            Ok(value) => {
                let mut offending = BTreeSet::new();
                collect_forbidden_strings(&value, "$", &mut offending);
                problems.extend(offending);
            }
            Err(error) => problems.push(format!("report is not serializable: {error}")),
        }
    }
}

fn check_percentiles(field: &str, report: &PercentileReport, problems: &mut Vec<String>) {
    if let PercentileReport::Measured(percentiles) = report {
        if percentiles.sample_count == 0 {
            problems.push(format!("{field} is measured with sample_count 0; use not_run"));
        }
        let finite = [percentiles.p50_ms, percentiles.p95_ms, percentiles.p99_ms]
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0);
        if !finite {
            problems.push(format!("{field} percentiles must be finite and non-negative"));
        }
    }
}

fn check_rate(field: &str, rate: &Rate, problems: &mut Vec<String>) {
    if rate.denominator == 0 {
        problems.push(format!("{field} denominator must be > 0"));
        return;
    }
    let expected = rate.numerator as f64 / rate.denominator as f64;
    if !rate.rate.is_finite() || (rate.rate - expected).abs() > 1e-9 {
        problems.push(format!(
            "{field} rate {} does not equal numerator/denominator {expected}",
            rate.rate
        ));
    }
}

fn collect_forbidden_strings(value: &serde_json::Value, path: &str, offending: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::String(text) => {
            if let Some(fragment) = forbidden_fragment(text) {
                offending.insert(format!("{path} contains forbidden fragment {fragment:?}"));
            }
        }
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_forbidden_strings(item, &format!("{path}[{index}]"), offending);
            }
        }
        serde_json::Value::Object(fields) => {
            for (key, item) in fields {
                if let Some(fragment) = forbidden_fragment(key) {
                    offending.insert(format!("{path}.{key} key contains forbidden fragment {fragment:?}"));
                }
                collect_forbidden_strings(item, &format!("{path}.{key}"), offending);
            }
        }
        _ => {}
    }
}

fn forbidden_fragment(text: &str) -> Option<&'static str> {
    if let Some(fragment) = FORBIDDEN_PATH_FRAGMENTS
        .iter()
        .find(|fragment| text.contains(*fragment))
    {
        return Some(fragment);
    }
    let lowered = text.to_ascii_lowercase();
    FORBIDDEN_SECRET_FRAGMENTS
        .iter()
        .find(|fragment| lowered.contains(*fragment))
        .copied()
}

/// Nearest-rank percentiles (`rank = ceil(p / 100 * n)`, 1-based). Sorts
/// `samples` in place; returns `None` for an empty slice so callers emit
/// [`PercentileReport::NotRun`] instead of zeros. Samples must be finite.
pub fn percentiles_from_samples(samples: &mut [f64]) -> Option<Percentiles> {
    if samples.is_empty() {
        return None;
    }
    samples.sort_by(f64::total_cmp);
    let count = samples.len();
    let pick = |percentile: f64| -> f64 {
        let rank = ((percentile / 100.0) * count as f64).ceil() as usize;
        let index = rank.clamp(1, count) - 1;
        samples.get(index).copied().unwrap_or(0.0)
    };
    Some(Percentiles {
        p50_ms: pick(50.0),
        p95_ms: pick(95.0),
        p99_ms: pick(99.0),
        sample_count: count as u64,
    })
}

/// [`PercentileReport`] from raw samples: `NotRun` when there are none.
pub fn percentile_report_from_samples(samples: &mut [f64]) -> PercentileReport {
    match percentiles_from_samples(samples) {
        Some(percentiles) => PercentileReport::Measured(percentiles),
        None => PercentileReport::NotRun,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 30 required fields named by the MT-142 contract, in declaration order.
    const CONTRACT_FIELDS: [&str; 30] = [
        "schema_id",
        "run_id",
        "source_commit",
        "surrealdb_version",
        "sdk_version",
        "engine_mode",
        "workload_seed",
        "worker_count",
        "operation_count",
        "dataset_cardinality",
        "operation_mix",
        "contention_ratio",
        "attempted_by_operation",
        "succeeded_by_operation",
        "failed_by_operation_and_class",
        "conflict_count",
        "conflict_rate",
        "retry_count",
        "retry_rate",
        "retry_exhaustion_count",
        "lock_wait_ms_p50_p95_p99",
        "latency_ms_p50_p95_p99_by_operation",
        "throughput_operations_per_second",
        "maximum_concurrent_operations",
        "timeout_count",
        "cancellation_count",
        "shutdown_elapsed_ms",
        "reopen_integrity_counts_and_hashes",
        "integrity_verdict",
        "remote_proof_status",
    ];

    /// Fields this schema adds beyond the contract list (research basis
    /// `numeric_budgets.machine_context`; review R2-1-3 `effective_parallelism`;
    /// review R2-1-7 `budgets` + `budget_verdict`).
    const EXTRA_FIELDS: [&str; 4] = [
        "machine_context",
        "effective_parallelism",
        "budgets",
        "budget_verdict",
    ];

    fn measured(p50: f64, p95: f64, p99: f64, samples: u64) -> PercentileReport {
        PercentileReport::Measured(Percentiles {
            p50_ms: p50,
            p95_ms: p95,
            p99_ms: p99,
            sample_count: samples,
        })
    }

    fn well_formed_report() -> SwarmLoadReport {
        let mut operation_mix = BTreeMap::new();
        let mut attempted = BTreeMap::new();
        let mut succeeded = BTreeMap::new();
        let mut latency = BTreeMap::new();
        for (index, class) in REQUIRED_OPERATION_CLASSES.into_iter().enumerate() {
            operation_mix.insert(
                class,
                OperationMixEntry {
                    share: 1.0 / REQUIRED_OPERATION_CLASSES.len() as f64,
                    status: OperationRunStatus::Run,
                },
            );
            let count = 100 + index as u64;
            attempted.insert(class, count);
            succeeded.insert(class, count - 1);
            latency.insert(class, measured(2.0, 8.5, 30.0, count));
        }
        let mut failed = BTreeMap::new();
        failed.insert(
            OperationClass::OptimisticVersionedUpdate,
            BTreeMap::from([(FailureClass::Terminal, 1u64)]),
        );
        SwarmLoadReport {
            schema_id: SWARM_LOAD_REPORT_SCHEMA_ID.to_string(),
            run_id: "mt142-ci-0001".to_string(),
            source_commit: "67ce701c".to_string(),
            surrealdb_version: "3.2.0".to_string(),
            sdk_version: "3.2.0".to_string(),
            engine_mode: EngineMode::EmbeddedRocksDb,
            workload_seed: 42,
            worker_count: 16,
            operation_count: 2000,
            dataset_cardinality: DatasetCardinality {
                records: 2000,
                workspaces: 4,
            },
            operation_mix,
            contention_ratio: 0.25,
            attempted_by_operation: attempted,
            succeeded_by_operation: succeeded,
            failed_by_operation_and_class: failed,
            conflict_count: 40,
            conflict_rate: Rate::new(40, 2000).expect("non-zero denominator"),
            retry_count: 38,
            retry_rate: Rate::new(38, 2000).expect("non-zero denominator"),
            retry_exhaustion_count: 0,
            lock_wait_ms_p50_p95_p99: measured(0.1, 1.5, 4.0, 900),
            latency_ms_p50_p95_p99_by_operation: latency,
            throughput_operations_per_second: 812.5,
            maximum_concurrent_operations: 16,
            timeout_count: 0,
            cancellation_count: 0,
            shutdown_elapsed_ms: 1200,
            reopen_integrity_counts_and_hashes: BTreeMap::from([(
                "knowledge_rich_documents".to_string(),
                IntegrityEntry {
                    row_count: 1999,
                    content_hash: "blake3:abc".to_string(),
                },
            )]),
            integrity_verdict: IntegrityVerdict::Pass,
            remote_proof_status: RemoteProofStatus::NotRunUnconfigured,
            machine_context: MachineContext {
                cpu_model: "AMD Ryzen 9 5950X 16-Core Processor".to_string(),
                logical_cpus: 32,
                total_memory_bytes: 137_347_756_032,
                store_drive_kind: StoreDriveKind::Hdd,
                os: "Microsoft Windows 11 Home 10.0.26200".to_string(),
            },
            effective_parallelism: EffectiveParallelism::new(9_600.0, 2_462)
                .expect("non-zero wall clock"),
            budgets: LoadBudgets {
                per_operation_timeout_ms: 5_000,
                per_worker_timeout_ms: 60_000,
                whole_test_timeout_ms: 180_000,
                latency_p99_budget_ms_by_operation: Some(BTreeMap::from([(
                    OperationClass::PointRead,
                    LatencyBudget {
                        p99_max_ms: 40.0,
                        label: BudgetLabel::Assumption,
                    },
                )])),
                conflict_rate_budget: Some(ConflictRateBudget {
                    max_rate: 0.4,
                    label: BudgetLabel::Assumption,
                }),
            },
            budget_verdict: BudgetVerdict::Pass,
        }
    }

    #[test]
    fn budget_verdict_is_derived_and_checked() {
        let mut report = well_formed_report();
        assert_eq!(report.evaluate_budgets(), BudgetVerdict::Pass);

        report.budgets.conflict_rate_budget = Some(ConflictRateBudget {
            max_rate: 0.01,
            label: BudgetLabel::Assumption,
        });
        assert_eq!(report.evaluate_budgets(), BudgetVerdict::Regression);
        let problems = report.validate().expect_err("stale verdict must fail");
        assert!(problems.iter().any(|p| p.starts_with("budget_verdict Pass")), "{problems:?}");
        report.budget_verdict = BudgetVerdict::Regression;
        assert_eq!(report.validate(), Ok(()));

        report.budgets.conflict_rate_budget = None;
        report.budgets.latency_p99_budget_ms_by_operation = None;
        assert_eq!(report.evaluate_budgets(), BudgetVerdict::NotConfigured);
        report.budget_verdict = BudgetVerdict::NotConfigured;
        assert_eq!(report.validate(), Ok(()));

        report.budgets.per_worker_timeout_ms = 0;
        let problems = report.validate().expect_err("zero budget must fail");
        assert!(problems.iter().any(|p| p == "budgets.per_worker_timeout_ms must be > 0"), "{problems:?}");
    }

    #[test]
    fn effective_parallelism_requires_wall_clock_and_consistent_ratio() {
        assert!(EffectiveParallelism::new(10.0, 0).is_none());
        assert!(EffectiveParallelism::new(f64::NAN, 10).is_none());
        let mut report = well_formed_report();
        report.effective_parallelism.wall_clock_ms = 0;
        let problems = report.validate().expect_err("zero wall clock must fail");
        assert!(problems.iter().any(|p| p == "effective_parallelism wall_clock_ms must be > 0"), "{problems:?}");
        let mut report = well_formed_report();
        report.effective_parallelism.ratio = 1.0;
        let problems = report.validate().expect_err("inconsistent ratio must fail");
        assert!(problems.iter().any(|p| p.starts_with("effective_parallelism ratio 1")), "{problems:?}");
        let value = serde_json::to_value(FailureClass::ExpectedStaleOrConflict).expect("serializes");
        assert_eq!(value, "expected_stale_or_conflict");
    }

    #[test]
    fn well_formed_report_validates() {
        let report = well_formed_report();
        assert_eq!(report.validate(), Ok(()));
    }

    #[test]
    fn validate_rejects_zero_sample_percentiles() {
        let mut report = well_formed_report();
        report.lock_wait_ms_p50_p95_p99 = measured(0.0, 0.0, 0.0, 0);
        report
            .latency_ms_p50_p95_p99_by_operation
            .insert(OperationClass::Delete, measured(1.0, 1.0, 1.0, 0));
        let problems = report.validate().expect_err("zero samples must fail");
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems.iter().all(|p| p.contains("sample_count 0")), "{problems:?}");

        report.lock_wait_ms_p50_p95_p99 = PercentileReport::NotRun;
        report
            .latency_ms_p50_p95_p99_by_operation
            .insert(OperationClass::Delete, PercentileReport::NotRun);
        assert_eq!(report.validate(), Ok(()));
    }

    #[test]
    fn validate_rejects_zero_denominators_and_inconsistent_rates() {
        let mut report = well_formed_report();
        report.conflict_rate = Rate {
            numerator: 0,
            denominator: 0,
            rate: 0.0,
        };
        report.retry_rate = Rate {
            numerator: 1,
            denominator: 4,
            rate: 0.5,
        };
        let problems = report.validate().expect_err("bad rates must fail");
        assert!(problems.iter().any(|p| p == "conflict_rate denominator must be > 0"), "{problems:?}");
        assert!(problems.iter().any(|p| p.starts_with("retry_rate rate 0.5")), "{problems:?}");
        assert!(Rate::new(3, 0).is_none());
    }

    #[test]
    fn validate_requires_every_required_operation_class() {
        let mut report = well_formed_report();
        report.operation_mix.remove(&OperationClass::Delete);
        report.attempted_by_operation.insert(OperationClass::Create, 0);
        report.succeeded_by_operation.insert(OperationClass::Create, 0);
        let problems = report.validate().expect_err("missing class must fail");
        assert!(problems.iter().any(|p| p.contains("missing required class Delete")), "{problems:?}");
        assert!(problems.iter().any(|p| p.contains("Create is marked run but attempted 0")), "{problems:?}");

        let mut report = well_formed_report();
        report.operation_mix.insert(
            OperationClass::Create,
            OperationMixEntry {
                share: 0.0,
                status: OperationRunStatus::NotRun,
            },
        );
        report.attempted_by_operation.insert(OperationClass::Create, 0);
        report.succeeded_by_operation.insert(OperationClass::Create, 0);
        assert_eq!(report.validate(), Ok(()));
    }

    #[test]
    fn validate_rejects_user_profile_paths_and_secrets() {
        let cases = [
            "C:\\Users\\someone\\store",
            "C:/Users/someone/store",
            "/home/someone/store",
            "/Users/someone/store",
            "Password=hunter2",
            "?token=abc",
        ];
        for text in cases {
            let mut report = well_formed_report();
            report.run_id = text.to_string();
            let problems = report.validate().expect_err(text);
            assert!(problems.iter().any(|p| p.starts_with("$.run_id contains forbidden fragment")), "{problems:?}");
        }
        let mut report = well_formed_report();
        report
            .reopen_integrity_counts_and_hashes
            .insert("/home/x".to_string(), IntegrityEntry { row_count: 1, content_hash: "h".to_string() });
        let problems = report.validate().expect_err("forbidden map key must fail");
        assert!(problems.iter().any(|p| p.contains("key contains forbidden fragment")), "{problems:?}");
    }

    #[test]
    fn validate_rejects_inconsistent_verdicts() {
        let mut report = well_formed_report();
        report.reopen_integrity_counts_and_hashes.clear();
        report.remote_proof_status = RemoteProofStatus::Pass;
        let problems = report.validate().expect_err("inconsistent verdicts must fail");
        assert_eq!(problems.len(), 2, "{problems:?}");
    }

    #[test]
    fn serde_round_trip_preserves_exact_field_names() {
        let report = well_formed_report();
        let value = serde_json::to_value(&report).expect("serializes");
        let object = value.as_object().expect("report is an object");
        let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
        for field in CONTRACT_FIELDS {
            assert!(actual.contains(field), "contract field {field} missing from {actual:?}");
        }
        let allowed: BTreeSet<&str> = CONTRACT_FIELDS
            .into_iter()
            .chain(EXTRA_FIELDS)
            .collect();
        let unexpected: Vec<&str> = actual.difference(&allowed).copied().collect();
        assert!(unexpected.is_empty(), "unexpected serialised keys: {unexpected:?}");
        assert_eq!(object.len(), CONTRACT_FIELDS.len() + EXTRA_FIELDS.len());

        assert_eq!(value["engine_mode"], "embedded_rocks_db");
        assert_eq!(value["remote_proof_status"], "not_run_unconfigured");
        assert_eq!(value["integrity_verdict"], "pass");
        assert_eq!(value["lock_wait_ms_p50_p95_p99"]["status"], "measured");
        assert_eq!(value["lock_wait_ms_p50_p95_p99"]["sample_count"], 900);
        assert_eq!(value["operation_mix"]["point_read"]["status"], "run");
        assert_eq!(value["machine_context"]["store_drive_kind"], "hdd");

        let text = serde_json::to_string(&report).expect("serializes");
        let decoded: SwarmLoadReport = serde_json::from_str(&text).expect("deserializes");
        assert_eq!(decoded, report);

        let not_run = serde_json::to_value(PercentileReport::NotRun).expect("serializes");
        assert_eq!(not_run, serde_json::json!({ "status": "not_run" }));
    }

    #[test]
    fn nearest_rank_percentiles() {
        assert!(percentiles_from_samples(&mut []).is_none());
        assert_eq!(percentile_report_from_samples(&mut []), PercentileReport::NotRun);

        let mut samples: Vec<f64> = (1..=100).map(f64::from).rev().collect();
        let percentiles = percentiles_from_samples(&mut samples).expect("samples");
        assert_eq!(percentiles.sample_count, 100);
        assert_eq!(percentiles.p50_ms, 50.0);
        assert_eq!(percentiles.p95_ms, 95.0);
        assert_eq!(percentiles.p99_ms, 99.0);

        let mut single = [7.5];
        let percentiles = percentiles_from_samples(&mut single).expect("samples");
        assert_eq!((percentiles.p50_ms, percentiles.p95_ms, percentiles.p99_ms), (7.5, 7.5, 7.5));
        assert_eq!(percentiles.sample_count, 1);

        let mut three = [30.0, 10.0, 20.0];
        let percentiles = percentiles_from_samples(&mut three).expect("samples");
        assert_eq!(percentiles.p50_ms, 20.0);
        assert_eq!(percentiles.p95_ms, 30.0);
        assert_eq!(percentiles.p99_ms, 30.0);
    }
}
