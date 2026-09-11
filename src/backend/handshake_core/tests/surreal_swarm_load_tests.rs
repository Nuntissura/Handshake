#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! MT-142 "Harden SurrealDB swarm concurrency and load" - deterministic
//! mixed-workload engine against the real embedded RocksDB store, with an
//! in-memory oracle, exact canonical-state reconciliation after a real
//! shutdown + reopen, and the `hsk.surreal_swarm_load_report@1` emitter.
//!
//! Proves AC-142-3, AC-142-5 (end-to-end retry accounting), AC-142-6
//! (PT-142-3, PT-142-7):
//! * `ci_profile_16_workers_2000_operations_is_correct_and_bounded`
//! * `extended_profile_64_workers_50000_operations` (runs only with
//!   `HANDSHAKE_SWARM_EXTENDED=1`; otherwise prints
//!   `SWARM_EXTENDED=NOT_RUN_UNCONFIGURED` and returns - never `#[ignore]`)
//!
//! Seed: `HANDSHAKE_SWARM_SEED` (printed as `SWARM_SEED=`). Report path is
//! printed as `SWARM_LOAD_REPORT=<path>`; the report directory is
//! `HANDSHAKE_SWARM_LOAD_REPORT_DIR` else
//! `HANDSHAKE_ARTIFACTS_ROOT/handshake-test/swarm-load/`.

#[path = "swarm_support/mod.rs"]
mod swarm_support;

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use handshake_core::storage::knowledge::{
    KnowledgeEntityKind, KnowledgeRichDocument, KnowledgeStore, NewKnowledgeEntity,
};
use handshake_core::storage::surreal::swarm_load_report::{
    percentile_report_from_samples, BudgetLabel, BudgetVerdict, ConflictRateBudget,
    DatasetCardinality, EffectiveParallelism, EngineMode, IntegrityVerdict, LatencyBudget,
    LoadBudgets, OperationClass, OperationMixEntry, OperationRunStatus, PercentileReport, Rate,
    RemoteProofStatus, SwarmLoadReport, REQUIRED_OPERATION_CLASSES,
    SWARM_LOAD_REPORT_SCHEMA_ID,
};
use handshake_core::storage::surreal::{
    RowFilter, SurrealDatabase, SurrealTestInspector, TableSelector,
};
use handshake_core::storage::{Database, LoomSearchFilters};
use serde_json::json;
use swarm_support::*;
use tokio::time::timeout;

/// Default workload shares per class (every required class is non-zero). The
/// read classes are `PointRead` + `RangeOrSearchQuery`; `HANDSHAKE_SWARM_READ_WRITE_MIX`
/// rescales reads against writes while keeping the ratios inside each group.
const DEFAULT_OPERATION_MIX: [(OperationClass, f64); 7] = [
    (OperationClass::PointRead, 0.30),
    (OperationClass::RangeOrSearchQuery, 0.10),
    (OperationClass::Create, 0.10),
    (OperationClass::IdempotentUpsert, 0.15),
    (OperationClass::OptimisticVersionedUpdate, 0.20),
    (OperationClass::Delete, 0.05),
    (OperationClass::MultiRecordProjectionOrLedgerTransaction, 0.10),
];

/// Contract minimums for the extended profile (`load_profiles.extended_local`).
const EXTENDED_MINIMUM_WORKERS: u32 = 64;
const EXTENDED_MINIMUM_OPERATIONS: u64 = 50_000;
const EXTENDED_MINIMUM_DATASET: u64 = 5_000;

/// Review R2-1-3: summed operation latency over wall clock must show real
/// engine overlap, not the call-boundary gauge's worker count.
const MINIMUM_EFFECTIVE_PARALLELISM: f64 = 2.0;

/// Reads an env var as `T`, panicking on a malformed value (never silently
/// falling back, which would hide a mistyped operator override).
fn env_parsed<T: std::str::FromStr>(name: &str) -> Option<T> {
    let raw = std::env::var(name).ok()?;
    let raw = raw.trim().to_owned();
    if raw.is_empty() {
        return None;
    }
    Some(
        raw.parse::<T>()
            .unwrap_or_else(|_| panic!("{name} must parse as {}, got {raw:?}", std::any::type_name::<T>())),
    )
}

#[derive(Clone, Copy, Debug)]
struct WorkloadConfig {
    profile: &'static str,
    seed: u64,
    workers: u32,
    operations: u64,
    /// Documents seeded before the workload (dataset cardinality floor).
    seed_documents: u64,
    workspaces: u32,
    /// Share of the seeded documents forming the hot set (key skew).
    hot_set_fraction: f64,
    /// Probability that a keyed write targets the hot set.
    contention_ratio: f64,
    /// Fraction of operations drawn from the read classes.
    read_fraction: f64,
    operation_mix: [(OperationClass, f64); 7],
    per_operation_timeout: Duration,
    per_worker_timeout: Duration,
    whole_test_timeout: Duration,
    /// Dedicated documents raced through shared idempotency keys.
    shared_idempotency_keys: u32,
    shared_entity_keys: u32,
    shared_titles: u32,
    /// True when any value came from the environment (recorded in the report).
    overridden: bool,
}

/// Rescales `DEFAULT_OPERATION_MIX` so the read classes sum to `read_fraction`
/// and the write classes share the remainder in their default proportions.
fn scaled_mix(read_fraction: f64) -> [(OperationClass, f64); 7] {
    let is_read = |class: OperationClass| {
        matches!(
            class,
            OperationClass::PointRead | OperationClass::RangeOrSearchQuery
        )
    };
    let default_read: f64 = DEFAULT_OPERATION_MIX
        .iter()
        .filter(|(class, _)| is_read(*class))
        .map(|(_, share)| share)
        .sum();
    let default_write = 1.0 - default_read;
    let mut mix = DEFAULT_OPERATION_MIX;
    for (class, share) in mix.iter_mut() {
        *share = if is_read(*class) {
            *share / default_read * read_fraction
        } else {
            *share / default_write * (1.0 - read_fraction)
        };
    }
    mix
}

impl WorkloadConfig {
    fn ci(seed: u64) -> Self {
        Self {
            profile: "ci",
            seed,
            workers: 16,
            operations: 2_000,
            seed_documents: 240,
            workspaces: 4,
            hot_set_fraction: 0.02,
            contention_ratio: 0.25,
            read_fraction: 0.40,
            operation_mix: DEFAULT_OPERATION_MIX,
            per_operation_timeout: PER_OPERATION_TIMEOUT,
            per_worker_timeout: PER_WORKER_TIMEOUT,
            whole_test_timeout: WHOLE_TEST_TIMEOUT,
            shared_idempotency_keys: 24,
            shared_entity_keys: 32,
            shared_titles: 16,
            overridden: false,
        }
    }

    /// Extended profile with the contract minimums as defaults; every knob is
    /// overridable from the environment (review R2-1-6) and every override is
    /// clamped to the contract minimum rather than silently accepted below it.
    fn extended(seed: u64) -> Self {
        let workers = env_parsed::<u32>("HANDSHAKE_SWARM_WORKERS")
            .unwrap_or(EXTENDED_MINIMUM_WORKERS)
            .max(EXTENDED_MINIMUM_WORKERS);
        let operations = env_parsed::<u64>("HANDSHAKE_SWARM_OPERATIONS")
            .unwrap_or(EXTENDED_MINIMUM_OPERATIONS)
            .max(EXTENDED_MINIMUM_OPERATIONS);
        let seed_documents = env_parsed::<u64>("HANDSHAKE_SWARM_DATASET")
            .unwrap_or(EXTENDED_MINIMUM_DATASET)
            .max(EXTENDED_MINIMUM_DATASET);
        let read_fraction = env_parsed::<f64>("HANDSHAKE_SWARM_READ_WRITE_MIX").unwrap_or(0.40);
        assert!(
            (0.05..=0.95).contains(&read_fraction),
            "HANDSHAKE_SWARM_READ_WRITE_MIX is the READ fraction and must stay within [0.05, 0.95] so every write class keeps a non-zero share, got {read_fraction}"
        );
        let hot_set_fraction = env_parsed::<f64>("HANDSHAKE_SWARM_KEY_SKEW").unwrap_or(0.01);
        assert!(
            (0.0001..=1.0).contains(&hot_set_fraction),
            "HANDSHAKE_SWARM_KEY_SKEW is the hot-set fraction and must stay within (0, 1], got {hot_set_fraction}"
        );
        let contention_ratio = env_parsed::<f64>("HANDSHAKE_SWARM_CONTENTION").unwrap_or(0.25);
        assert!(
            (0.0..=1.0).contains(&contention_ratio),
            "HANDSHAKE_SWARM_CONTENTION must stay within [0, 1], got {contention_ratio}"
        );
        let overridden = [
            "HANDSHAKE_SWARM_WORKERS",
            "HANDSHAKE_SWARM_OPERATIONS",
            "HANDSHAKE_SWARM_DATASET",
            "HANDSHAKE_SWARM_READ_WRITE_MIX",
            "HANDSHAKE_SWARM_KEY_SKEW",
            "HANDSHAKE_SWARM_CONTENTION",
            "HANDSHAKE_SWARM_SEED",
        ]
        .iter()
        .any(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()));
        Self {
            profile: "extended",
            seed,
            workers,
            operations,
            seed_documents,
            workspaces: 8,
            hot_set_fraction,
            contention_ratio,
            read_fraction,
            operation_mix: scaled_mix(read_fraction),
            per_operation_timeout: PER_OPERATION_TIMEOUT,
            per_worker_timeout: Duration::from_millis(600_000),
            whole_test_timeout: Duration::from_millis(1_800_000),
            shared_idempotency_keys: 256,
            shared_entity_keys: 512,
            shared_titles: 128,
            overridden,
        }
    }

    fn hot_len(&self) -> usize {
        ((self.seed_documents as f64 * self.hot_set_fraction).round() as usize).max(1)
    }

    /// Every effective knob, for the report (review R2-1-6).
    fn effective_configuration(&self) -> serde_json::Value {
        json!({
            "profile": self.profile,
            "seed": self.seed,
            "workers": self.workers,
            "operations": self.operations,
            "dataset_records_seeded": self.seed_documents,
            "workspaces": self.workspaces,
            "read_fraction": self.read_fraction,
            "key_skew_hot_set_fraction": self.hot_set_fraction,
            "contention_ratio": self.contention_ratio,
            "per_operation_timeout_ms": self.per_operation_timeout.as_millis() as u64,
            "per_worker_timeout_ms": self.per_worker_timeout.as_millis() as u64,
            "whole_test_timeout_ms": self.whole_test_timeout.as_millis() as u64,
            "shared_idempotency_keys": self.shared_idempotency_keys,
            "shared_entity_keys": self.shared_entity_keys,
            "shared_titles": self.shared_titles,
            "env_overridden": self.overridden,
            "env_vars": [
                "HANDSHAKE_SWARM_SEED", "HANDSHAKE_SWARM_WORKERS", "HANDSHAKE_SWARM_OPERATIONS",
                "HANDSHAKE_SWARM_DATASET", "HANDSHAKE_SWARM_READ_WRITE_MIX",
                "HANDSHAKE_SWARM_KEY_SKEW", "HANDSHAKE_SWARM_CONTENTION",
            ],
            "contract_minimums_enforced": {
                "workers": EXTENDED_MINIMUM_WORKERS,
                "operations": EXTENDED_MINIMUM_OPERATIONS,
                "dataset_records": EXTENDED_MINIMUM_DATASET,
            },
        })
    }
}

/// The research basis's PRE-MEASUREMENT p99 expectations
/// (`numeric_budgets.ci_profile.expected_p99_latency_ms_range`, every value
/// labelled ASSUMPTION there). These are never overwritten: the divergence
/// between what the research basis predicted and what this HDD actually
/// delivers is itself evidence, so the report carries them under
/// `research_basis_assumption` even once measured budgets are in force.
const RESEARCH_BASIS_P99_ASSUMPTION_MS: [(OperationClass, f64); 7] = [
    (OperationClass::PointRead, 20.0),
    (OperationClass::RangeOrSearchQuery, 300.0),
    (OperationClass::Create, 150.0),
    (OperationClass::IdempotentUpsert, 150.0),
    (OperationClass::OptimisticVersionedUpdate, 150.0),
    (OperationClass::Delete, 400.0),
    (
        OperationClass::MultiRecordProjectionOrLedgerTransaction,
        400.0,
    ),
];

/// Headroom applied when a budget is derived from a measured p99, so the
/// budget detects a genuine regression instead of tracking run-to-run noise.
const MEASURED_BUDGET_HEADROOM: f64 = 2.0;

/// Provenance of a measured latency budget: the clean run it was derived
/// from, so a reader can audit how the number was set. Constructed the moment
/// [`MEASURED_P99_BUDGETS`] is filled in from the first clean run; until then
/// the type exists so the derivation contract is reviewable in code.
#[allow(dead_code)]
struct MeasuredBudgetSource {
    /// `run_id` of the clean run the p99s came from.
    run_id: &'static str,
    /// Store-open duration of that run, proving it was measured after the
    /// bootstrap regression was fixed.
    store_open_ms: u64,
    /// `(class, measured p99 ms, budget ms)`; budget = p99 x
    /// [`MEASURED_BUDGET_HEADROOM`], rounded to a sensible figure.
    per_class: [(OperationClass, f64, f64); 7],
}

/// Measured per-class budgets, derived from the first CLEAN run on this
/// machine once A2's bootstrap fix landed. `None` until that run exists, in
/// which case the research-basis assumptions above stay in force and the
/// report says so under `budget_provenance.mode`.
const MEASURED_P99_BUDGETS: Option<MeasuredBudgetSource> = Some(MeasuredBudgetSource {
    // First clean CI run after the bootstrap cost was explained (not a
    // regression): 16 workers, 2,041 operations, integrity pass, zero failed
    // predicates. Budgets are that run's p99 x MEASURED_BUDGET_HEADROOM,
    // rounded up to a round figure, so a genuine future regression trips them
    // while run-to-run noise does not.
    run_id: "mt142-ci-01a08ec242ec7d2395ca32c7986b1589",
    store_open_ms: 209_729,
    per_class: [
        (OperationClass::PointRead, 45.0, 100.0),
        (OperationClass::RangeOrSearchQuery, 54.7, 120.0),
        (OperationClass::Create, 150.4, 320.0),
        (OperationClass::IdempotentUpsert, 392.9, 800.0),
        (OperationClass::OptimisticVersionedUpdate, 468.7, 950.0),
        (OperationClass::Delete, 653.7, 1_350.0),
        (
            OperationClass::MultiRecordProjectionOrLedgerTransaction,
            437.6,
            900.0,
        ),
    ],
});

/// Recorded, non-gating budgets (review R2-1-7): the timeouts actually applied
/// plus the per-class latency budget currently in force. A `budget_verdict` of
/// `regression` means "measured outside the recorded budget", never a
/// correctness failure - `integrity_verdict` owns correctness.
fn load_budgets(config: &WorkloadConfig) -> LoadBudgets {
    let latency = match &MEASURED_P99_BUDGETS {
        Some(source) => source
            .per_class
            .iter()
            .map(|(class, _measured, budget)| {
                (
                    *class,
                    LatencyBudget {
                        p99_max_ms: *budget,
                        label: BudgetLabel::Measured,
                    },
                )
            })
            .collect(),
        None => RESEARCH_BASIS_P99_ASSUMPTION_MS
            .iter()
            .map(|(class, p99_max_ms)| {
                (
                    *class,
                    LatencyBudget {
                        p99_max_ms: *p99_max_ms,
                        label: BudgetLabel::Assumption,
                    },
                )
            })
            .collect(),
    };
    LoadBudgets {
        per_operation_timeout_ms: config.per_operation_timeout.as_millis() as u64,
        per_worker_timeout_ms: config.per_worker_timeout.as_millis() as u64,
        whole_test_timeout_ms: config.whole_test_timeout.as_millis() as u64,
        latency_p99_budget_ms_by_operation: Some(latency),
        // Kept as the ASSUMPTION it is: 0.043 measured against a 0.40 ceiling
        // is comfortably inside, so it remains a meaningful bound.
        conflict_rate_budget: Some(ConflictRateBudget {
            max_rate: 0.40,
            label: BudgetLabel::Assumption,
        }),
    }
}

/// Auditable provenance of whatever budget is in force, plus the untouched
/// research-basis assumptions.
fn budget_provenance() -> serde_json::Value {
    let assumptions: BTreeMap<String, f64> = RESEARCH_BASIS_P99_ASSUMPTION_MS
        .iter()
        .map(|(class, ms)| (format!("{class:?}"), *ms))
        .collect();
    let mut provenance = json!({
        "research_basis_assumption": {
            "p99_ms_by_operation": assumptions,
            "label": "assumption",
            "source": "research_basis.json numeric_budgets.ci_profile.expected_p99_latency_ms_range (pre-measurement estimate; kept even when measured budgets are in force so the divergence stays visible)",
        },
        "conflict_rate_budget": {
            "max_rate": 0.40,
            "label": "assumption",
            "source": "research_basis.json numeric_budgets.ci_profile.expected_conflict_rate_range",
        },
    });
    provenance["mode"] = match &MEASURED_P99_BUDGETS {
        None => json!("research_basis_assumption"),
        Some(_) => json!("measured_with_headroom"),
    };
    if let Some(source) = &MEASURED_P99_BUDGETS {
        let derived: BTreeMap<String, serde_json::Value> = source
            .per_class
            .iter()
            .map(|(class, measured, budget)| {
                (
                    format!("{class:?}"),
                    json!({
                        "measured_p99_ms": measured,
                        "budget_p99_ms": budget,
                        "headroom_factor": budget / measured,
                    }),
                )
            })
            .collect();
        provenance["measured"] = json!({
            "source_run_id": source.run_id,
            "source_store_open_ms": source.store_open_ms,
            "headroom_factor": MEASURED_BUDGET_HEADROOM,
            "label": "measured",
            "per_class": derived,
            "derivation": "budget = measured p99 x headroom, rounded; derived from the first clean run after the bootstrap regression was fixed, on the machine_context recorded in this report",
        });
    }
    provenance
}

/// Process RSS before/after with the store's on-disk size, or a typed NotRun
/// with the reason when the platform has no cheap probe (review R2-1-6).
fn memory_fragment(store_path: &std::path::Path, rss_before: Option<u64>) -> serde_json::Value {
    let rss_after = process_rss_bytes();
    let store_bytes = directory_size_bytes(store_path);
    match (rss_before, rss_after) {
        (Some(before), Some(after)) => json!({
            "status": "measured",
            "rss_bytes_before": before,
            "rss_bytes_after": after,
            "store_bytes": store_bytes,
            "rss_over_store_ratio": store_bytes
                .filter(|bytes| *bytes > 0)
                .map(|bytes| after as f64 / bytes as f64),
            "finding_rule": "research basis numeric_budgets.extended_profile.memory_watch: RSS > 4x the on-disk store size is a reportable finding, never a pass gate",
        }),
        _ => json!({
            "status": "not_run",
            "reason": "no cheap resident-set probe on this platform (Windows-only Get-Process WorkingSet64 helper)",
            "store_bytes": store_bytes,
        }),
    }
}

fn pick_class(rng: &mut SwarmRng, mix: &[(OperationClass, f64); 7]) -> OperationClass {
    let roll = rng.next_f64();
    let mut cumulative = 0.0;
    for (class, share) in mix {
        cumulative += share;
        if roll < cumulative {
            return *class;
        }
    }
    mix[mix.len() - 1].0
}

#[derive(Clone, Debug)]
struct DocRef {
    rich_document_id: String,
}

/// Live document pool: `hot` never shrinks (contention never dies), `cold`
/// grows with creates and shrinks with deletes.
struct DocPool {
    hot: Vec<DocRef>,
    cold: Mutex<Vec<DocRef>>,
}

impl DocPool {
    fn pick_hot(&self, rng: &mut SwarmRng) -> DocRef {
        self.hot[rng.below(self.hot.len())].clone()
    }

    fn pick_cold(&self, rng: &mut SwarmRng) -> Option<DocRef> {
        let cold = self.cold.lock().expect("cold pool");
        if cold.is_empty() {
            None
        } else {
            Some(cold[rng.below(cold.len())].clone())
        }
    }

    fn pick(&self, rng: &mut SwarmRng, contention_ratio: f64) -> DocRef {
        if rng.chance(contention_ratio) {
            self.pick_hot(rng)
        } else {
            self.pick_cold(rng).unwrap_or_else(|| self.pick_hot(rng))
        }
    }

    fn push_cold(&self, doc: DocRef) {
        self.cold.lock().expect("cold pool").push(doc);
    }

    /// Removes and returns a random cold document (reserved for deletion).
    fn take_cold(&self, rng: &mut SwarmRng) -> Option<DocRef> {
        let mut cold = self.cold.lock().expect("cold pool");
        if cold.is_empty() {
            return None;
        }
        let index = rng.below(cold.len());
        Some(cold.swap_remove(index))
    }

    fn remove(&self, rich_document_id: &str) {
        let mut cold = self.cold.lock().expect("cold pool");
        cold.retain(|doc| doc.rich_document_id != rich_document_id);
    }
}

struct Selectors {
    documents: TableSelector,
    loom_blocks: TableSelector,
    search_index: TableSelector,
}

struct Shared {
    config: WorkloadConfig,
    run_id: String,
    workspaces: Vec<String>,
    pool: DocPool,
    /// Dedicated documents for the shared idempotency keys (never in a pool).
    idempotency_targets: Vec<DocRef>,
    oracle: Mutex<Oracle>,
    gauge: InFlightGauge,
    db: SurrealDatabase,
    doc_api: DocApi,
    inspector: SurrealTestInspector,
    selectors: Selectors,
}

type Cache = HashMap<String, (i64, String)>;

struct Worker {
    shared: Arc<Shared>,
    id: u32,
    rng: SwarmRng,
    cache: Cache,
    metrics: SwarmMetrics,
    sub: u64,
}

impl Worker {
    fn oracle(&self) -> std::sync::MutexGuard<'_, Oracle> {
        self.shared.oracle.lock().expect("oracle")
    }

    fn per_op(&self) -> Duration {
        self.shared.config.per_operation_timeout
    }

    async fn head_version(&mut self, doc: &DocRef) -> Result<Option<(i64, String)>, OpOutcome> {
        if let Some(cached) = self.cache.get(&doc.rich_document_id) {
            return Ok(Some(cached.clone()));
        }
        let read = timeout(
            self.per_op(),
            self.shared
                .db
                .get_knowledge_rich_document(&doc.rich_document_id),
        )
        .await;
        match read {
            Err(_) => Err(OpOutcome::Timeout),
            Ok(Err(error)) => Err(OpOutcome::from_error(&error)),
            Ok(Ok(None)) => {
                self.oracle().note_missing_read(&doc.rich_document_id);
                self.shared.pool.remove(&doc.rich_document_id);
                Ok(None)
            }
            Ok(Ok(Some(document))) => {
                let head = (document.doc_version, document.content_sha256.clone());
                self.cache
                    .insert(doc.rich_document_id.clone(), head.clone());
                Ok(Some(head))
            }
        }
    }

    fn note_saved(&mut self, document: &KnowledgeRichDocument) {
        self.cache.insert(
            document.rich_document_id.clone(),
            (document.doc_version, document.content_sha256.clone()),
        );
        self.oracle().ack_save(document);
    }

    async fn point_read(&mut self) -> OpOutcome {
        let doc = self
            .shared
            .pool
            .pick(&mut self.rng, self.shared.config.contention_ratio);
        let read = timeout(
            self.per_op(),
            self.shared
                .db
                .get_knowledge_rich_document(&doc.rich_document_id),
        )
        .await;
        let document = match read {
            Err(_) => return OpOutcome::Timeout,
            Ok(Err(error)) => return OpOutcome::from_error(&error),
            Ok(Ok(None)) => {
                self.oracle().note_missing_read(&doc.rich_document_id);
                self.cache.remove(&doc.rich_document_id);
                self.shared.pool.remove(&doc.rich_document_id);
                return OpOutcome::Ok;
            }
            Ok(Ok(Some(document))) => document,
        };
        // Dirty-read proof: the observed head must have a committed version row.
        let row = timeout(
            self.per_op(),
            self.shared
                .db
                .get_knowledge_rich_document_version(&document.rich_document_id, document.doc_version),
        )
        .await;
        match row {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => OpOutcome::from_error(&error),
            Ok(Ok(row)) => {
                let matches = row.is_some_and(|row| row.content_sha256 == document.content_sha256);
                self.oracle().note_read(&document, matches);
                self.cache.insert(
                    document.rich_document_id.clone(),
                    (document.doc_version, document.content_sha256.clone()),
                );
                OpOutcome::Ok
            }
        }
    }

    async fn range_or_search(&mut self) -> OpOutcome {
        let workspace = self.shared.workspaces[self.rng.below(self.shared.workspaces.len())].clone();
        self.sub += 1;
        if self.sub % 2 == 0 {
            let listed = timeout(
                self.per_op(),
                self.shared
                    .db
                    .list_knowledge_rich_documents(&workspace, None, None),
            )
            .await;
            match listed {
                Err(_) => OpOutcome::Timeout,
                Ok(Err(error)) => OpOutcome::from_error(&error),
                Ok(Ok(documents)) => {
                    for document in &documents {
                        assert!(
                            document.doc_version >= 1,
                            "range query returned an unversioned document {}",
                            document.rich_document_id
                        );
                    }
                    // Dirty-read sample: the first listed head must be committed.
                    if let Some(document) = documents.first() {
                        let row = timeout(
                            self.per_op(),
                            self.shared.db.get_knowledge_rich_document_version(
                                &document.rich_document_id,
                                document.doc_version,
                            ),
                        )
                        .await;
                        match row {
                            Err(_) => return OpOutcome::Timeout,
                            Ok(Err(error)) => return OpOutcome::from_error(&error),
                            Ok(Ok(row)) => {
                                let matches = row
                                    .is_some_and(|row| row.content_sha256 == document.content_sha256);
                                self.oracle().note_read(document, matches);
                            }
                        }
                    }
                    OpOutcome::Ok
                }
            }
        } else {
            let searched = timeout(
                self.per_op(),
                self.shared.db.search_loom_blocks(
                    &workspace,
                    "swarm",
                    LoomSearchFilters::default(),
                    25,
                    0,
                ),
            )
            .await;
            match searched {
                Err(_) => OpOutcome::Timeout,
                Ok(Err(error)) => OpOutcome::from_error(&error),
                Ok(Ok(_results)) => OpOutcome::Ok,
            }
        }
    }

    async fn create(&mut self, operation: u64) -> OpOutcome {
        let workspace = self.shared.workspaces[self.rng.below(self.shared.workspaces.len())].clone();
        let title = format!("swarm create w{} op{operation} {}", self.id, self.shared.run_id);
        let created = timeout(
            self.per_op(),
            self.shared.db.create_knowledge_rich_document(new_document(
                &workspace,
                &title,
                &format!("created by worker {} op {operation}", self.id),
            )),
        )
        .await;
        match created {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => OpOutcome::from_error(&error),
            Ok(Ok(document)) => {
                self.oracle().ack_create(&document);
                self.cache.insert(
                    document.rich_document_id.clone(),
                    (document.doc_version, document.content_sha256.clone()),
                );
                self.shared.pool.push_cold(DocRef {
                    rich_document_id: document.rich_document_id,
                });
                OpOutcome::Ok
            }
        }
    }

    /// Three idempotent sub-kinds in rotation: keyed save (private key with an
    /// immediate replay, or a shared key raced across workers), natural-key
    /// entity upsert, natural-key title create.
    async fn idempotent_upsert(&mut self, operation: u64) -> OpOutcome {
        self.sub += 1;
        match self.sub % 3 {
            0 => self.idempotent_save(operation).await,
            1 => self.entity_upsert().await,
            _ => self.title_create().await,
        }
    }

    async fn idempotent_save(&mut self, operation: u64) -> OpOutcome {
        let shared_key = self.rng.chance(0.5);
        let (key, doc, expected_version, payload) = if shared_key {
            let index = self.rng.below(self.shared.idempotency_targets.len());
            let doc = self.shared.idempotency_targets[index].clone();
            (
                format!("{}-shared-{index}", self.shared.run_id),
                doc,
                1i64,
                document_content(&format!("shared idempotent effect {index}")),
            )
        } else {
            let doc = self
                .shared
                .pool
                .pick(&mut self.rng, self.shared.config.contention_ratio);
            let Some((version, _)) = (match self.head_version(&doc).await {
                Ok(head) => head,
                Err(outcome) => return outcome,
            }) else {
                return OpOutcome::NotFound("document vanished before idempotent save".to_owned());
            };
            (
                format!("{}-w{}-op{operation}", self.shared.run_id, self.id),
                doc,
                version,
                document_content(&format!("private idempotent effect w{} op{operation}", self.id)),
            )
        };
        let first = self.idempotent_call(&key, &doc, expected_version, payload.clone()).await;
        if first != OpOutcome::Ok || shared_key {
            return first;
        }
        // Immediate replay of a private key: must converge to the same effect.
        let started = Instant::now();
        let replay = self.idempotent_call(&key, &doc, expected_version, payload).await;
        let latency = started.elapsed();
        self.metrics.record(
            OperationClass::IdempotentUpsert,
            &replay,
            latency,
            self.id,
            operation,
        );
        if replay != OpOutcome::Ok {
            self.oracle().violations.push(Violation {
                class: IntegrityVerdict::DuplicateEffect,
                detail: format!("immediate replay of idempotency key {key} did not converge: {replay:?}"),
            });
        }
        first
    }

    async fn idempotent_call(
        &mut self,
        key: &str,
        doc: &DocRef,
        expected_version: i64,
        payload: serde_json::Value,
    ) -> OpOutcome {
        let saved = timeout(
            self.per_op(),
            self.shared.db.save_knowledge_rich_document_version_idempotent(
                key,
                &doc.rich_document_id,
                expected_version,
                payload,
                None,
                None,
                None,
            ),
        )
        .await;
        match saved {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => {
                let outcome = OpOutcome::from_error(&error);
                if matches!(outcome, OpOutcome::TypedConflict(_)) {
                    self.cache.remove(&doc.rich_document_id);
                }
                if matches!(outcome, OpOutcome::NotFound(_)) {
                    self.oracle().note_missing_read(&doc.rich_document_id);
                    self.shared.pool.remove(&doc.rich_document_id);
                }
                outcome
            }
            Ok(Ok(write)) => {
                self.oracle()
                    .ack_idempotent(key, &write.value, write.replayed);
                self.cache.insert(
                    write.value.rich_document_id.clone(),
                    (write.value.doc_version, write.value.content_sha256.clone()),
                );
                OpOutcome::Ok
            }
        }
    }

    async fn entity_upsert(&mut self) -> OpOutcome {
        let workspace = self.shared.workspaces[self.rng.below(self.shared.workspaces.len())].clone();
        let key = format!(
            "swarm-entity-{}",
            self.rng.below(self.shared.config.shared_entity_keys as usize)
        );
        let upserted = timeout(
            self.per_op(),
            self.shared.db.upsert_knowledge_entity(NewKnowledgeEntity {
                workspace_id: workspace.clone(),
                entity_kind: KnowledgeEntityKind::Concept,
                entity_key: key.clone(),
                display_name: format!("Swarm entity {key}"),
                detection_provenance: json!({ "source": "mt142-swarm", "worker": self.id }),
                primary_source_id: None,
                detected_in_run: None,
                evidence_span_ids: Vec::new(),
            }),
        )
        .await;
        match upserted {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => OpOutcome::from_error(&error),
            Ok(Ok(entity)) => {
                self.oracle().note_natural_key(
                    NaturalKeyKind::Entity,
                    &workspace,
                    &key,
                    &entity.entity_id,
                    false,
                );
                OpOutcome::Ok
            }
        }
    }

    async fn title_create(&mut self) -> OpOutcome {
        let workspace = self.shared.workspaces[self.rng.below(self.shared.workspaces.len())].clone();
        let title = format!(
            "Swarm Shared Title {}",
            self.rng.below(self.shared.config.shared_titles as usize)
        );
        let created = timeout(
            self.per_op(),
            self.shared
                .db
                .create_knowledge_rich_document_if_title_absent(new_document(
                    &workspace,
                    &title,
                    &format!("title natural key {title}"),
                )),
        )
        .await;
        match created {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => OpOutcome::from_error(&error),
            Ok(Ok((document, created))) => {
                let mut oracle = self.oracle();
                if created {
                    oracle.ack_create(&document);
                }
                oracle.note_natural_key(
                    NaturalKeyKind::Title,
                    &workspace,
                    &title,
                    &document.rich_document_id,
                    created,
                );
                OpOutcome::Ok
            }
        }
    }

    /// Compare-and-set save with the expected version taken from the worker's
    /// cache, so contention produces real typed stale outcomes.
    async fn optimistic_update(&mut self, operation: u64) -> OpOutcome {
        let doc = self
            .shared
            .pool
            .pick(&mut self.rng, self.shared.config.contention_ratio);
        let Some((expected_version, _)) = (match self.head_version(&doc).await {
            Ok(head) => head,
            Err(outcome) => return outcome,
        }) else {
            return OpOutcome::NotFound("document vanished before optimistic update".to_owned());
        };
        self.save(&doc, expected_version, &format!("optimistic w{} op{operation}", self.id))
            .await
    }

    async fn save(&mut self, doc: &DocRef, expected_version: i64, text: &str) -> OpOutcome {
        let saved = timeout(
            self.per_op(),
            self.shared.db.save_knowledge_rich_document_version(
                &doc.rich_document_id,
                expected_version,
                document_content(text),
                None,
                None,
                None,
            ),
        )
        .await;
        match saved {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => {
                let outcome = OpOutcome::from_error(&error);
                match &outcome {
                    OpOutcome::TypedConflict(_) | OpOutcome::UntypedEngineConflict(_) => {
                        self.cache.remove(&doc.rich_document_id);
                    }
                    OpOutcome::NotFound(_) => {
                        self.oracle().note_missing_read(&doc.rich_document_id);
                        self.cache.remove(&doc.rich_document_id);
                        self.shared.pool.remove(&doc.rich_document_id);
                    }
                    _ => {}
                }
                outcome
            }
            Ok(Ok(document)) => {
                self.note_saved(&document);
                OpOutcome::Ok
            }
        }
    }

    async fn delete(&mut self, operation: u64) -> OpOutcome {
        let Some(doc) = self.shared.pool.take_cold(&mut self.rng) else {
            // The hot set is never deleted; with no cold document left this
            // delete attempt fails typed (counted under Delete, no violation).
            return OpOutcome::NotFound("no cold document is available to delete".to_owned());
        };
        self.oracle().mark_delete_pending(&doc.rich_document_id);
        self.cache.remove(&doc.rich_document_id);
        let deleted = timeout(
            self.per_op(),
            self.shared
                .doc_api
                .delete_document(&doc.rich_document_id, &format!("w{}-op{operation}", self.id)),
        )
        .await;
        match deleted {
            Err(_) => OpOutcome::Timeout,
            Ok(Ok(ack)) => {
                self.oracle()
                    .ack_delete(&doc.rich_document_id, &ack.receipt_event_id);
                OpOutcome::Ok
            }
            Ok(Err(DeleteFailure::Conflict(text))) => OpOutcome::TypedConflict(text),
            Ok(Err(DeleteFailure::NotFound(text))) => OpOutcome::NotFound(text),
            Ok(Err(DeleteFailure::Other(status, text))) => {
                OpOutcome::Terminal(format!("delete HTTP {status}: {text}"))
            }
            Ok(Err(DeleteFailure::Transport(text))) => {
                OpOutcome::Terminal(format!("delete transport: {text}"))
            }
        }
    }

    /// A create or save whose storage transaction touches the document, its
    /// version row, its loom block and its search-index row; atomicity is
    /// verified by reading every touched table afterwards.
    async fn multi_record_transaction(&mut self, operation: u64) -> OpOutcome {
        self.sub += 1;
        let (outcome, target): (OpOutcome, Option<(String, i64, String)>) = if self.sub % 2 == 0 {
            let workspace = self.shared.workspaces[self.rng.below(self.shared.workspaces.len())].clone();
            let title = format!("swarm multi w{} op{operation} {}", self.id, self.shared.run_id);
            let created = timeout(
                self.per_op(),
                self.shared.db.create_knowledge_rich_document(new_document(
                    &workspace,
                    &title,
                    &format!("multi-record create w{} op{operation}", self.id),
                )),
            )
            .await;
            match created {
                Err(_) => (OpOutcome::Timeout, None),
                Ok(Err(error)) => (OpOutcome::from_error(&error), None),
                Ok(Ok(document)) => {
                    self.oracle().ack_create(&document);
                    self.cache.insert(
                        document.rich_document_id.clone(),
                        (document.doc_version, document.content_sha256.clone()),
                    );
                    self.shared.pool.push_cold(DocRef {
                        rich_document_id: document.rich_document_id.clone(),
                    });
                    (
                        OpOutcome::Ok,
                        Some((
                            document.rich_document_id,
                            document.doc_version,
                            document.content_sha256,
                        )),
                    )
                }
            }
        } else {
            let doc = self
                .shared
                .pool
                .pick(&mut self.rng, self.shared.config.contention_ratio);
            let Some((expected_version, _)) = (match self.head_version(&doc).await {
                Ok(head) => head,
                Err(outcome) => return outcome,
            }) else {
                return OpOutcome::NotFound("document vanished before multi-record save".to_owned());
            };
            let outcome = self
                .save(&doc, expected_version, &format!("multi-record save w{} op{operation}", self.id))
                .await;
            let target = (outcome == OpOutcome::Ok)
                .then(|| self.cache.get(&doc.rich_document_id).cloned())
                .flatten()
                .map(|(version, sha)| (doc.rich_document_id.clone(), version, sha));
            (outcome, target)
        };
        let Some((rich_document_id, doc_version, content_sha256)) = target else {
            return outcome;
        };
        // Atomicity proof: every projected row of the committed transaction.
        let checks = timeout(self.per_op(), async {
            let inspector = &self.shared.inspector;
            let selectors = &self.shared.selectors;
            let doc_row = inspector
                .exists(&selectors.documents, RowFilter::IdEquals(rich_document_id.clone()))
                .await
                .map_err(|error| error.to_string())?;
            let loom_row = inspector
                .exists(&selectors.loom_blocks, RowFilter::IdEquals(rich_document_id.clone()))
                .await
                .map_err(|error| error.to_string())?;
            let search_row = inspector
                .exists(&selectors.search_index, RowFilter::IdEquals(rich_document_id.clone()))
                .await
                .map_err(|error| error.to_string())?;
            let version_row = self
                .shared
                .db
                .get_knowledge_rich_document_version(&rich_document_id, doc_version)
                .await
                .map_err(|error| error.to_string())?
                .is_some_and(|row| row.content_sha256 == content_sha256);
            Ok::<_, String>((doc_row, loom_row, search_row, version_row))
        })
        .await;
        match checks {
            Err(_) => OpOutcome::Timeout,
            Ok(Err(error)) => OpOutcome::Terminal(format!("multi-record verification failed: {error}")),
            Ok(Ok((doc_row, loom_row, search_row, version_row))) => {
                if !(doc_row && loom_row && search_row && version_row) {
                    self.oracle().violations.push(Violation {
                        class: IntegrityVerdict::PartialCommit,
                        detail: format!(
                            "multi-record transaction on {rich_document_id} v{doc_version} left a partial projection: document={doc_row} loom_block={loom_row} search_index={search_row} version_row={version_row}"
                        ),
                    });
                }
                OpOutcome::Ok
            }
        }
    }

    async fn run(mut self, operations: u64) -> SwarmMetrics {
        for operation in 0..operations {
            let class = pick_class(&mut self.rng, &self.shared.config.operation_mix);
            let shared = Arc::clone(&self.shared);
            let _in_flight = shared.gauge.enter();
            let started = Instant::now();
            let outcome = match class {
                OperationClass::PointRead => self.point_read().await,
                OperationClass::RangeOrSearchQuery => self.range_or_search().await,
                OperationClass::Create => self.create(operation).await,
                OperationClass::IdempotentUpsert => self.idempotent_upsert(operation).await,
                OperationClass::OptimisticVersionedUpdate => self.optimistic_update(operation).await,
                OperationClass::Delete => self.delete(operation).await,
                OperationClass::MultiRecordProjectionOrLedgerTransaction => {
                    self.multi_record_transaction(operation).await
                }
            };
            self.metrics
                .record(class, &outcome, started.elapsed(), self.id, operation);
            if outcome == OpOutcome::Timeout {
                // A timeout is a failure; stop this worker so the report names
                // the class instead of cascading further timeouts.
                break;
            }
        }
        self.metrics
    }
}

struct ProfileOutcome {
    report: SwarmLoadReport,
    report_path: std::path::PathBuf,
    shutdown_elapsed: Duration,
    seeded_documents: u64,
    /// Every profile predicate that failed; empty is the only pass. The
    /// per-class metrics, worker timeouts and reconciliation violations behind
    /// these strings are already in `report` and its diagnostics block.
    failures: Vec<String>,
}

async fn seed_documents(
    db: &SurrealDatabase,
    workspaces: &[String],
    count: u64,
    parallelism: u32,
    prefix: &str,
    per_operation_timeout: Duration,
) -> Vec<(KnowledgeRichDocument, String)> {
    let mut tasks = Vec::new();
    for lane in 0..parallelism as u64 {
        let db = db.clone();
        let workspaces = workspaces.to_vec();
        let prefix = prefix.to_owned();
        tasks.push(tokio::spawn(async move {
            let mut created = Vec::new();
            let mut index = lane;
            while index < count {
                let workspace = workspaces[(index % workspaces.len() as u64) as usize].clone();
                let document = timeout(
                    per_operation_timeout,
                    db.create_knowledge_rich_document(new_document(
                        &workspace,
                        &format!("{prefix} {index}"),
                        &format!("{prefix} base {index}"),
                    )),
                )
                .await
                .unwrap_or_else(|_| panic!("seeding document {index} exceeded its per-operation bound"))
                .unwrap_or_else(|error| panic!("seeding document {index} failed: {error}"));
                created.push((document, workspace));
                index += parallelism as u64;
            }
            created
        }));
    }
    let mut documents = Vec::new();
    for task in tasks {
        documents.extend(task.await.expect("seed lane joined"));
    }
    documents.sort_by(|a, b| a.0.title.cmp(&b.0.title));
    documents
}

async fn run_profile(config: WorkloadConfig) -> ProfileOutcome {
    let diagnostics_owned = install_retry_diagnostics();
    clear_captured_product_events();
    let retry_before = retry_diagnostics_snapshot();
    let rss_before = process_rss_bytes();
    let run_id = new_run_id(&format!("mt142-{}", config.profile));
    println!(
        "SWARM_SEED={} SWARM_RUN_ID={run_id} SWARM_PROFILE={} workers={} operations={} retry_diagnostics_owned={diagnostics_owned}",
        config.seed, config.profile, config.workers, config.operations
    );

    // Bounded and measured: the store open is reported next to the budgets so
    // a whole-test budget can be re-derived against evidence.
    let open_started = Instant::now();
    let store = open_store_measured().await;
    let store_open_ms = open_started.elapsed().as_millis() as u64;
    let mut workspaces = Vec::with_capacity(config.workspaces as usize);
    for _ in 0..config.workspaces {
        workspaces.push(store.create_workspace().await);
    }
    let inspector = store.storage.test_inspector();
    let baseline = table_counts(&inspector).await;
    let selectors = Selectors {
        documents: inspector
            .table_selector("knowledge_rich_documents")
            .await
            .expect("documents selector"),
        loom_blocks: inspector
            .table_selector("loom_blocks")
            .await
            .expect("loom_blocks selector"),
        search_index: inspector
            .table_selector("loom_block_search_index")
            .await
            .expect("search index selector"),
    };
    let doc_api = DocApi::boot(&store).await;

    // Dataset seeding (parallel lanes, deterministic titles).
    let seeding_started = Instant::now();
    let seeded = seed_documents(
        &store.db,
        &workspaces,
        config.seed_documents,
        config.workers.min(16),
        "swarm seed",
        config.per_operation_timeout,
    )
    .await;
    let idempotency_seeds = seed_documents(
        &store.db,
        &workspaces,
        u64::from(config.shared_idempotency_keys),
        config.workers.min(16),
        "swarm idem",
        config.per_operation_timeout,
    )
    .await;
    let seeding_elapsed_ms = seeding_started.elapsed().as_millis();
    println!(
        "SWARM_SEEDED documents={} idempotency_targets={} elapsed_ms={}",
        seeded.len(),
        idempotency_seeds.len(),
        seeding_started.elapsed().as_millis()
    );
    let mut oracle = Oracle::default();
    for (document, _) in seeded.iter().chain(idempotency_seeds.iter()) {
        oracle.ack_create(document);
    }
    // Seeding waits are not part of the measured workload.
    let seeding_lock_wait_samples = store.db.lock_registry().take_lock_wait_samples().len();
    let lock_mode = store.db.lock_registry().mode();
    let hot_len = config.hot_len().min(seeded.len());
    let refs: Vec<DocRef> = seeded
        .iter()
        .map(|(document, _)| DocRef {
            rich_document_id: document.rich_document_id.clone(),
        })
        .collect();
    let pool = DocPool {
        hot: refs[..hot_len].to_vec(),
        cold: Mutex::new(refs[hot_len..].to_vec()),
    };
    let idempotency_targets: Vec<DocRef> = idempotency_seeds
        .iter()
        .map(|(document, _)| DocRef {
            rich_document_id: document.rich_document_id.clone(),
        })
        .collect();
    let seeded_documents = (seeded.len() + idempotency_seeds.len()) as u64;

    let shared = Arc::new(Shared {
        config,
        run_id: run_id.clone(),
        workspaces: workspaces.clone(),
        pool,
        idempotency_targets,
        oracle: Mutex::new(oracle),
        gauge: InFlightGauge::default(),
        db: store.db.clone(),
        doc_api,
        inspector: inspector.clone(),
        selectors,
    });

    // Workload.
    let per_worker = config.operations / u64::from(config.workers);
    let remainder = config.operations % u64::from(config.workers);
    // Review R2-1-3: the engine-window gauge is incremented inside
    // `SurrealStorage::with_lease`, so a hidden global mutex cannot inflate it.
    store.storage.reset_lease_high_water();
    let workload_started = Instant::now();
    let mut tasks = Vec::with_capacity(config.workers as usize);
    for worker_id in 0..config.workers {
        let operations = per_worker + u64::from(u64::from(worker_id) < remainder);
        let worker = Worker {
            shared: Arc::clone(&shared),
            id: worker_id,
            rng: SwarmRng::derive(config.seed, u64::from(worker_id)),
            cache: HashMap::new(),
            metrics: SwarmMetrics::default(),
            sub: u64::from(worker_id),
        };
        let bound = config.per_worker_timeout;
        tasks.push(tokio::spawn(async move {
            match timeout(bound, worker.run(operations)).await {
                Ok(metrics) => Ok(metrics),
                Err(_) => Err(format!(
                    "worker {worker_id} exceeded its per-worker bound of {} ms",
                    bound.as_millis()
                )),
            }
        }));
    }
    let mut metrics = SwarmMetrics::default();
    let mut worker_timeouts = Vec::new();
    for task in tasks {
        match task.await.expect("worker task joined") {
            Ok(worker_metrics) => metrics.merge(worker_metrics),
            Err(timeout_text) => worker_timeouts.push(timeout_text),
        }
    }
    let workload_elapsed = workload_started.elapsed();
    let lease_high_water = store.storage.lease_high_water();
    let retry_after = retry_diagnostics_snapshot().delta_since(&retry_before);
    // Keyed-lock waits recorded by the product registry during the workload
    // (every clone of the wrapper, including the delete route's, shares it).
    let lock_wait_sample_count = store.db.lock_registry().lock_wait_sample_count();
    let lock_wait_samples_dropped = store.db.lock_registry().lock_wait_samples_dropped();
    let mut lock_wait_ms: Vec<f64> = store
        .db
        .lock_registry()
        .take_lock_wait_samples()
        .into_iter()
        .map(|wait| wait.as_secs_f64() * 1000.0)
        .collect();
    if lock_mode == handshake_core::storage::surreal::keyed_lock::LockMode::Keyed {
        assert!(
            lock_wait_sample_count > 0,
            "a keyed registry must record lock-wait samples during the workload (seeding recorded {seeding_lock_wait_samples})"
        );
    }
    let lock_wait_report = percentile_report_from_samples(&mut lock_wait_ms);
    println!(
        "SWARM_LOCK_WAIT mode={lock_mode:?} samples={lock_wait_sample_count} report={lock_wait_report:?}"
    );
    println!(
        "SWARM_WORKLOAD attempted={} succeeded={} conflicts={} untyped_conflicts={} retries={} retry_exhaustions={} timeouts={} cancellations={} lease_high_water={lease_high_water} call_boundary_high_water={} elapsed_ms={}",
        metrics.attempted_total(),
        metrics.succeeded_total(),
        metrics.conflicts,
        metrics.untyped_conflicts.len(),
        retry_after.scheduled,
        retry_after.exhausted,
        metrics.timeouts,
        metrics.cancellations,
        shared.gauge.high_water(),
        workload_elapsed.as_millis()
    );

    // Graceful shutdown of the one owning engine, then a real reopen.
    let shared = Arc::try_unwrap(shared).unwrap_or_else(|_| panic!("every worker released the shared state"));
    shared.doc_api.shutdown().await;
    let oracle = shared.oracle.into_inner().expect("oracle");
    let max_in_flight = shared.gauge.high_water();
    let shutdown_started = Instant::now();
    let shutdown_wait = store.storage.config().shutdown_wait();
    let shutdown_report = store
        .storage
        .shutdown_with_report()
        .await
        .expect("graceful shutdown of the embedded engine must succeed");
    let shutdown_elapsed = shutdown_started.elapsed();
    println!(
        "SWARM_SHUTDOWN elapsed_ms={} report={shutdown_report:?} bound_ms={}",
        shutdown_elapsed.as_millis(),
        shutdown_wait.as_millis()
    );
    assert!(
        !store.storage.is_accepting_operations(),
        "shutdown must stop admission"
    );
    let reopened = store
        .reopen_database()
        .await
        .expect("reopen the same data_dir after shutdown");
    let reopened_inspector = reopened.storage().test_inspector();
    let reconcile_started = Instant::now();
    let integrity = reconcile(&reopened, &reopened_inspector, &oracle, &baseline).await;
    let reconcile_elapsed_ms = reconcile_started.elapsed().as_millis();
    println!(
        "SWARM_INTEGRITY verdict={:?} violations={} documents_checked={} versions_checked={} elapsed_ms={}",
        integrity.verdict,
        integrity.violations.len(),
        integrity.documents_checked,
        integrity.versions_checked,
        reconcile_started.elapsed().as_millis()
    );
    let store_path = reopened.storage().config().path().to_path_buf();
    reopened
        .storage()
        .shutdown()
        .await
        .expect("shutdown of the reopened engine");
    drop(reopened_inspector);
    drop(reopened);
    store.close_and_remove().await.expect("close and remove the store");

    // Report.
    let attempted_total = metrics.attempted_total().max(1);
    let mut operation_mix = BTreeMap::new();
    let mut attempted_by_operation = BTreeMap::new();
    let mut succeeded_by_operation = BTreeMap::new();
    let mut failed_by_operation_and_class = BTreeMap::new();
    let mut latency_by_operation = BTreeMap::new();
    let mut sum_operation_latency_ms = 0.0f64;
    for (class, share) in config.operation_mix {
        let class_metrics = metrics.by_class.get(&class);
        let attempted = class_metrics.map(|m| m.attempted).unwrap_or(0);
        operation_mix.insert(
            class,
            OperationMixEntry {
                share,
                status: if attempted > 0 {
                    OperationRunStatus::Run
                } else {
                    OperationRunStatus::NotRun
                },
            },
        );
        attempted_by_operation.insert(class, attempted);
        succeeded_by_operation.insert(class, class_metrics.map(|m| m.succeeded).unwrap_or(0));
        if let Some(m) = class_metrics {
            if !m.failed.is_empty() {
                failed_by_operation_and_class.insert(class, m.failed.clone());
            }
        }
        let mut samples: Vec<f64> = class_metrics.map(|m| m.latency_ms.clone()).unwrap_or_default();
        sum_operation_latency_ms += samples.iter().sum::<f64>();
        latency_by_operation.insert(class, percentile_report_from_samples(&mut samples));
    }

    // Review R2-1-2: every predicate the test asserts on is evaluated BEFORE
    // the artifact is written and folded into the verdict, so no report on
    // disk can read as a pass for a run that failed.
    let mut failures: Vec<String> = Vec::new();
    if !worker_timeouts.is_empty() {
        failures.push(format!("worker timeouts: {worker_timeouts:?}"));
    }
    if metrics.timeouts > 0 {
        failures.push(format!(
            "{} operation timeouts ({:?})",
            metrics.timeouts, metrics.timed_out_classes
        ));
    }
    if !metrics.lock_wait_timeouts.is_empty() {
        failures.push(format!("lock-wait timeouts: {:?}", metrics.lock_wait_timeouts));
    }
    if !metrics.retry_exhausted_errors.is_empty() {
        failures.push(format!(
            "retry exhaustion: {:?}",
            metrics.retry_exhausted_errors
        ));
    }
    if !metrics.untyped_conflicts.is_empty() {
        failures.push(format!(
            "raw engine conflicts leaked untyped: {:?}",
            metrics.untyped_conflicts
        ));
    }
    if !metrics.unexpected_terminal.is_empty() {
        failures.push(format!(
            "unexpected terminal errors: {:?}",
            metrics.unexpected_terminal
        ));
    }
    if metrics.cancellations > 0 {
        failures.push(format!(
            "{} operations were cancelled during the workload",
            metrics.cancellations
        ));
    }
    for class in REQUIRED_OPERATION_CLASSES {
        let attempted = attempted_by_operation.get(&class).copied().unwrap_or(0);
        let succeeded = succeeded_by_operation.get(&class).copied().unwrap_or(0);
        if attempted == 0 {
            failures.push(format!("required operation class {class:?} was never attempted"));
        } else if succeeded == 0 {
            failures.push(format!(
                "required operation class {class:?} never succeeded ({attempted} attempted)"
            ));
        }
    }
    if integrity.verdict != IntegrityVerdict::Pass {
        failures.push(format!(
            "integrity reconciliation verdict {:?}: {}",
            integrity.verdict,
            integrity.rendered_violations(5)
        ));
    }
    let effective_parallelism =
        EffectiveParallelism::new(sum_operation_latency_ms, workload_elapsed.as_millis() as u64)
            .expect("workload wall clock is non-zero");
    println!(
        "SWARM_PARALLELISM sum_latency_ms={:.1} wall_ms={} ratio={:.2}",
        effective_parallelism.sum_operation_latency_ms,
        effective_parallelism.wall_clock_ms,
        effective_parallelism.ratio
    );
    if effective_parallelism.ratio < MINIMUM_EFFECTIVE_PARALLELISM {
        failures.push(format!(
            "effective_parallelism ratio {:.2} is below {MINIMUM_EFFECTIVE_PARALLELISM} (summed operation latency over wall clock): the workers were globally serialized",
            effective_parallelism.ratio
        ));
    }
    // Verdict precedence: a timeout, a cancellation and a retry exhaustion each
    // name their own class; a missing operation class is NotRun; any other
    // failed predicate leaves the integrity claim unproven and is reported as
    // PartialCommit (the operation's commit outcome was never typed). Only an
    // empty failure list may carry the reconciliation's own Pass.
    let integrity_verdict = if !worker_timeouts.is_empty()
        || metrics.timeouts > 0
        || !metrics.lock_wait_timeouts.is_empty()
    {
        IntegrityVerdict::Timeout
    } else if !metrics.retry_exhausted_errors.is_empty() {
        IntegrityVerdict::RetryExhausted
    } else if metrics.cancellations > 0 {
        IntegrityVerdict::Cancelled
    } else if REQUIRED_OPERATION_CLASSES.iter().any(|class| {
        attempted_by_operation.get(class).copied().unwrap_or(0) == 0
            || succeeded_by_operation.get(class).copied().unwrap_or(0) == 0
    }) {
        IntegrityVerdict::NotRun
    } else if integrity.verdict != IntegrityVerdict::Pass {
        integrity.verdict
    } else if failures.is_empty() {
        IntegrityVerdict::Pass
    } else {
        IntegrityVerdict::PartialCommit
    };
    let budgets = load_budgets(&config);
    let report = SwarmLoadReport {
        schema_id: SWARM_LOAD_REPORT_SCHEMA_ID.to_owned(),
        run_id: run_id.clone(),
        source_commit: source_commit(),
        surrealdb_version: SURREALDB_VERSION.to_owned(),
        sdk_version: SURREALDB_VERSION.to_owned(),
        engine_mode: EngineMode::EmbeddedRocksDb,
        workload_seed: config.seed,
        worker_count: config.workers,
        operation_count: metrics.attempted_total(),
        dataset_cardinality: DatasetCardinality {
            records: oracle.docs.len() as u64,
            workspaces: config.workspaces,
        },
        operation_mix,
        contention_ratio: config.contention_ratio,
        attempted_by_operation,
        succeeded_by_operation,
        failed_by_operation_and_class,
        conflict_count: metrics.conflicts,
        conflict_rate: Rate::new(metrics.conflicts, attempted_total).expect("non-zero denominator"),
        retry_count: retry_after.scheduled,
        retry_rate: Rate::new(retry_after.scheduled, attempted_total).expect("non-zero denominator"),
        retry_exhaustion_count: retry_after.exhausted,
        // Drained from the product registry after the workload; an empty
        // sample set (registry disabled) is NOT_RUN, never a zero-latency PASS.
        lock_wait_ms_p50_p95_p99: lock_wait_report,
        latency_ms_p50_p95_p99_by_operation: latency_by_operation,
        throughput_operations_per_second: metrics.attempted_total() as f64
            / workload_elapsed.as_secs_f64().max(f64::EPSILON),
        // Engine-window measure (review R2-1-3), not the call-boundary gauge.
        maximum_concurrent_operations: lease_high_water as u32,
        timeout_count: metrics.timeouts + worker_timeouts.len() as u64,
        cancellation_count: metrics.cancellations,
        shutdown_elapsed_ms: shutdown_elapsed.as_millis() as u64,
        reopen_integrity_counts_and_hashes: integrity.counts_and_hashes.clone(),
        integrity_verdict,
        remote_proof_status: RemoteProofStatus::NotRunUnconfigured,
        machine_context: machine_context(&store_path),
        effective_parallelism,
        budgets,
        budget_verdict: BudgetVerdict::NotConfigured,
    };
    let report = SwarmLoadReport {
        budget_verdict: report.evaluate_budgets(),
        ..report
    };
    let validation = report.validate();
    let mut value = serde_json::to_value(&report).expect("report serializes");
    value["validation"] = match &validation {
        Ok(()) => json!("ok"),
        Err(problems) => json!(problems),
    };
    value["diagnostics"] = json!({
        "retry_diagnostics_owned_by_this_process": diagnostics_owned,
        "untyped_engine_conflicts": metrics.untyped_conflicts.clone(),
        "retry_exhausted_errors": metrics.retry_exhausted_errors.clone(),
        "unexpected_terminal_errors": metrics.unexpected_terminal.iter().take(50).collect::<Vec<_>>(),
        "timed_out_operations": metrics.timed_out_classes.clone(),
        "worker_timeouts": worker_timeouts.clone(),
        "lock_wait_timeouts": metrics.lock_wait_timeouts.clone(),
        "product_warn_error_events": captured_product_events(),
        "shutdown_report": {
            "drained": shutdown_report.drained,
            "cancelled": shutdown_report.cancelled,
            "engine_elapsed_ms": shutdown_report.elapsed.as_millis() as u64,
            "shutdown_wait_bound_ms": shutdown_wait.as_millis() as u64,
        },
        // Review R2-1-9: the sampled registry is the one this test's
        // SurrealDatabase owns. Every HTTP route builds its own wrapper per
        // request (api/knowledge_documents.rs db_for -> SurrealDatabase::new),
        // so the Delete class, which runs through the loopback route, never
        // contributes a sample to these percentiles.
        "lock_wait_samples": {
            "registry_mode": format!("{lock_mode:?}"),
            "workload_samples": lock_wait_sample_count,
            "samples_dropped_by_cap": lock_wait_samples_dropped,
            "seeding_samples_discarded": seeding_lock_wait_samples,
            "covered_operation_classes": [
                "point_read", "range_or_search_query", "create", "idempotent_upsert",
                "optimistic_versioned_update", "multi_record_projection_or_ledger_transaction",
            ],
            "excluded_operation_classes": ["delete"],
            "exclusion_reason": "delete runs through the loopback HTTP route, whose handler builds a fresh SurrealDatabase (and therefore a fresh KeyedLockRegistry) per request; its lock waits are never sampled here",
        },
        "concurrency_measures": {
            "engine_window_lease_high_water": lease_high_water,
            "call_boundary_high_water": max_in_flight,
            "note": "maximum_concurrent_operations reports the engine-window lease high-water mark (SurrealStorage::lease_high_water, incremented inside with_lease); call_boundary_high_water is the weaker gauge that equals the worker count even under total serialization (review R2-1-3)",
        },
        "run_verdict": if failures.is_empty() { "pass" } else { "fail" },
        "failed_predicates": failures.clone(),
        "verdict_precedence": "timeout|lock_wait_timeout -> timeout; retry exhaustion -> retry_exhausted; cancellation -> cancelled; missing class coverage -> not_run; reconciliation violation -> its own class; any other failed predicate -> partial_commit; only an empty failed_predicates list may carry pass",
        "integrity_violations": integrity.violations.iter().take(100).map(Violation::render).collect::<Vec<_>>(),
        "seeded_documents": seeded_documents,
        "dirty_read_checks": oracle.dirty_read_checks,
        "acknowledged_writes": oracle.acknowledged_writes,
        "workload_elapsed_ms": workload_elapsed.as_millis() as u64,
        "effective_configuration": config.effective_configuration(),
        "budget_provenance": budget_provenance(),
        "whole_test_derivation": json!({
            "setup_bound_ms": STORE_OPEN_BOUND.as_millis() as u64,
            "workload_bound_ms": config.whole_test_timeout.as_millis() as u64,
            "measured_setup_store_open_ms": store_open_ms,
            "measured_workload_ms": workload_elapsed.as_millis() as u64,
            "measured_seed_and_reconcile_ms": (reconcile_elapsed_ms + seeding_elapsed_ms) as u64,
            "rule": "setup (cold schema apply, ~4,467 fsynced DDL statements on a 7200-rpm disk) is a fixed cost bounded separately by setup_bound_ms; workload_bound_ms budgets only seeding + workload + shutdown + reopen + reconciliation, so a workload regression is not masked by setup latency",
        }),
        "store_open_ms": store_open_ms,
        "memory": memory_fragment(&store_path, rss_before),
    });
    let report_path = write_report_json(&format!("swarm-load-{}-{run_id}.json", config.profile), &value);
    println!("SWARM_LOAD_REPORT={}", report_path.display());
    println!(
        "SWARM_VERDICT integrity={:?} budget={:?} failed_predicates={}",
        report.integrity_verdict,
        report.budget_verdict,
        failures.len()
    );
    assert_eq!(
        validation,
        Ok(()),
        "hsk.surreal_swarm_load_report@1 must validate"
    );
    // R2-1-2: the artifact can never claim a pass the run did not earn.
    assert_eq!(
        report.integrity_verdict == IntegrityVerdict::Pass,
        failures.is_empty(),
        "a report with integrity_verdict pass must imply an empty failed-predicate list; verdict {:?}, failures {failures:?}",
        report.integrity_verdict
    );

    ProfileOutcome {
        report,
        report_path,
        shutdown_elapsed,
        seeded_documents,
        failures,
    }
}

fn assert_profile(outcome: &ProfileOutcome, config: &WorkloadConfig) {
    let report = &outcome.report;
    // Every predicate below was already evaluated inside run_profile and folded
    // into the artifact's verdict (review R2-1-2); this is the same list,
    // asserted with the full diagnostic text.
    assert!(
        outcome.failures.is_empty(),
        "the {} profile failed {} predicate(s):\n  {}\nproduct warn/error events: {:?}\nreport: {}",
        config.profile,
        outcome.failures.len(),
        outcome.failures.join("\n  "),
        captured_product_events(),
        outcome.report_path.display()
    );
    assert_eq!(
        report.integrity_verdict,
        IntegrityVerdict::Pass,
        "report integrity verdict must be pass once no predicate failed"
    );
    assert!(
        report.operation_count >= config.operations,
        "the profile must attempt at least {} operations, attempted {}",
        config.operations,
        report.operation_count
    );
    assert!(
        report.worker_count >= config.workers,
        "the profile must run at least {} workers",
        config.workers
    );
    // Review R2-1-3: the call-boundary gauge stays as the contract field, but
    // the anti-serialization proof is the effective-parallelism ratio.
    assert!(
        report.maximum_concurrent_operations >= 2,
        "workers must overlap at the call boundary (maximum_concurrent_operations >= 2)"
    );
    assert!(
        report.effective_parallelism.ratio >= MINIMUM_EFFECTIVE_PARALLELISM,
        "effective_parallelism ratio {:.2} is below {MINIMUM_EFFECTIVE_PARALLELISM}: summed operation latency {:.1} ms over {} ms wall clock means the workers were globally serialized",
        report.effective_parallelism.ratio,
        report.effective_parallelism.sum_operation_latency_ms,
        report.effective_parallelism.wall_clock_ms
    );
    assert!(
        report.conflict_count > 0,
        "the contended profile must observe at least one typed conflict (zero contention would be a meaningless green)"
    );
    assert_eq!(report.remote_proof_status, RemoteProofStatus::NotRunUnconfigured);
    assert!(
        matches!(report.lock_wait_ms_p50_p95_p99, PercentileReport::Measured(p) if p.sample_count > 0),
        "the keyed profile must report measured lock-wait percentiles, got {:?}",
        report.lock_wait_ms_p50_p95_p99
    );
    // Budgets are recorded, never gating (research basis: the ranges are
    // ASSUMPTIONs until measured); only their bookkeeping is asserted.
    assert_ne!(
        report.budget_verdict,
        BudgetVerdict::NotConfigured,
        "the profile must record numeric budgets so a load-budget regression is readable from the JSON"
    );
    assert_eq!(
        report.budgets.per_operation_timeout_ms,
        config.per_operation_timeout.as_millis() as u64,
        "recorded per-operation budget must be the bound actually applied"
    );
    assert!(
        outcome.shutdown_elapsed <= handshake_core::storage::surreal::DEFAULT_SHUTDOWN_WAIT,
        "shutdown must complete inside its explicit bound (shutdown_wait)"
    );
    assert!(outcome.report_path.exists(), "report file must exist");
    assert!(outcome.seeded_documents > 0, "dataset must be seeded");
}

/// True when the extended profile is configured to run in this process.
fn extended_profile_enabled() -> bool {
    std::env::var("HANDSHAKE_SWARM_EXTENDED")
        .map(|value| value.trim() == "1")
        .unwrap_or(false)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn ci_profile_16_workers_2000_operations_is_correct_and_bounded() {
    // Review R2-1-17: both profiles share process-global retry and product
    // event counters and one HDD, so they never run in the same process.
    if extended_profile_enabled() {
        println!(
            "SWARM_CI=NOT_RUN_EXTENDED_CONFIGURED (HANDSHAKE_SWARM_EXTENDED=1 reserves this process for the extended profile; run the CI profile without that variable)"
        );
        return;
    }
    let config = WorkloadConfig::ci(workload_seed());
    // Setup and workload are budgeted SEPARATELY: a cold schema apply is a
    // fixed ~360 s cost on this disk and must not be conflated with the
    // workload budget (see `budget_provenance.whole_test_derivation`).
    let outer = STORE_OPEN_BOUND + config.whole_test_timeout;
    let outcome = timeout(outer, run_profile(config))
        .await
        .unwrap_or_else(|_| {
            panic!(
                "CI profile exceeded setup bound {} ms + workload bound {} ms",
                STORE_OPEN_BOUND.as_millis(),
                config.whole_test_timeout.as_millis()
            )
        });
    assert_profile(&outcome, &config);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn extended_profile_64_workers_50000_operations() {
    if !extended_profile_enabled() {
        // Review R2-1-12: the NOT_RUN state is a machine-readable artifact,
        // not console prose, so a validator can tell "never attempted" from
        // "attempted without configuration".
        println!("SWARM_EXTENDED=NOT_RUN_UNCONFIGURED (set HANDSHAKE_SWARM_EXTENDED=1 to run the >=64 worker / >=50000 operation profile)");
        let run_id = new_run_id("mt142-extended-not-run");
        let fragment = json!({
            "schema_id": REPORT_FRAGMENT_SCHEMA_ID,
            "fragment": "extended_profile",
            "run_id": run_id,
            "status": "not_run_unconfigured",
            "gate_env_var": "HANDSHAKE_SWARM_EXTENDED",
            "gate_expected_value": "1",
            "source_commit": source_commit(),
            "configurable_env_vars": [
                "HANDSHAKE_SWARM_SEED", "HANDSHAKE_SWARM_WORKERS", "HANDSHAKE_SWARM_OPERATIONS",
                "HANDSHAKE_SWARM_DATASET", "HANDSHAKE_SWARM_READ_WRITE_MIX",
                "HANDSHAKE_SWARM_KEY_SKEW", "HANDSHAKE_SWARM_CONTENTION",
            ],
            "contract_minimums": {
                "workers": EXTENDED_MINIMUM_WORKERS,
                "operations": EXTENDED_MINIMUM_OPERATIONS,
                "dataset_records": EXTENDED_MINIMUM_DATASET,
            },
        });
        let path = write_report_json(&format!("swarm-load-extended-not-run-{run_id}.json"), &fragment);
        println!("SWARM_EXTENDED_NOT_RUN_REPORT={}", path.display());
        return;
    }
    let config = WorkloadConfig::extended(workload_seed());
    println!(
        "SWARM_EXTENDED_CONFIG workers={} operations={} dataset={} read_fraction={} key_skew={} contention={} seed={}",
        config.workers,
        config.operations,
        config.seed_documents,
        config.read_fraction,
        config.hot_set_fraction,
        config.contention_ratio,
        config.seed
    );
    let outer = STORE_OPEN_BOUND + config.whole_test_timeout;
    let outcome = timeout(outer, run_profile(config))
        .await
        .unwrap_or_else(|_| {
            panic!(
                "extended profile exceeded setup bound {} ms + workload bound {} ms",
                STORE_OPEN_BOUND.as_millis(),
                config.whole_test_timeout.as_millis()
            )
        });
    assert!(
        outcome.report.worker_count >= EXTENDED_MINIMUM_WORKERS,
        "extended profile needs >= {EXTENDED_MINIMUM_WORKERS} workers"
    );
    assert!(
        outcome.report.operation_count >= EXTENDED_MINIMUM_OPERATIONS,
        "extended profile needs >= {EXTENDED_MINIMUM_OPERATIONS} operations"
    );
    assert!(
        outcome.report.dataset_cardinality.records >= EXTENDED_MINIMUM_DATASET,
        "extended profile needs dataset cardinality >= {EXTENDED_MINIMUM_DATASET}"
    );
    assert_profile(&outcome, &config);
}
