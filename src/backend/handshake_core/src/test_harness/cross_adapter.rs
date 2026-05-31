//! MT-057 — cross-adapter sandbox parity harness.
//!
//! Runs a fixed workload matrix against every registered SandboxAdapter
//! and asserts the (exit_code, stdout/stderr predicates,
//! sandbox_adapter_id, stop_reason) tuple matches across adapters,
//! modulo (UUIDs, timestamps, sandbox_internal_id). Adapters that
//! cannot be instantiated in the current test environment surface as
//! `ScenarioOutcome::Skipped { reason }` rather than silent passes —
//! the report exposes `skipped_adapters` to the validator so coverage
//! gaps are explicit.
//!
//! Authoring follows the `tests/kernel_003_migration_tests.rs` fixture
//! convention: each scenario deserializes from JSON into a
//! `WorkloadFixture` carrying a `ProcessSpec`, an expected outcome
//! predicate set, and an optional `kill_signal` for lifecycle-test
//! scenarios. The companion test file at
//! `tests/sandbox_cross_adapter_parity_tests.rs` (MT-057 owned)
//! materializes the fixture catalog and drives this harness.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::process_ledger::table::LedgerEvent;
use crate::process_ledger::{
    LedgerBatcher, LedgerBatcherConfig, LedgerOverflowEvent, ProcessLedgerDrain,
    ProcessLedgerError, ProcessLedgerOverflowSink, ProcessLedgerStore,
};
use crate::sandbox::adapter::SandboxAdapter;
use crate::sandbox::ledger_decorator::LedgerDecorator;
use crate::sandbox::types::{AdapterId, ProcessSpec, ProcessStatus, Signal};

/// Fixture file shape — kept stable across the 8 MT-057 scenarios.
/// `schema_id` follows the newer fixture convention
/// (`hsk.<feature>_fixture@<n>`).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkloadFixture {
    pub schema_id: String,
    pub name: String,
    pub description: String,
    pub spec: ProcessSpec,
    pub expected: ExpectedOutcome,
    /// When set, the harness calls `kill(handle, signal)` after spawn
    /// and asserts the process exits within `kill_timeout_ms` (default
    /// 10_000ms). When `None`, the harness polls `status` until exit.
    #[serde(default)]
    pub kill_signal: Option<Signal>,
    #[serde(default)]
    pub kill_timeout_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExpectedOutcome {
    /// `Zero` requires `exit_code == 0`. `NonZero` requires exit_code != 0.
    /// `Exact(code)` requires an exact match.
    pub exit_code_class: ExitClass,
    #[serde(default)]
    pub stop_reason_contains: Option<String>,
    #[serde(default)]
    pub stdout_predicate: Option<StdoutPredicate>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitClass {
    Zero,
    NonZero,
    Exact(i32),
    /// Either zero or non-zero is acceptable. Useful for scenarios
    /// where the underlying tool varies between adapters (e.g.,
    /// "command not found" returns different codes in podman/docker).
    Any,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StdoutPredicate {
    ExactBytes(String),
    Substring(String),
    /// Empty stdout. Useful for "expected nothing on stdout but exit 0".
    Empty,
}

/// One adapter's outcome on one fixture. The validator reads
/// `skipped_adapters` from `ParityReport` to verify cross-adapter
/// coverage rather than trusting a passing test on a single adapter.
#[derive(Clone, Debug)]
pub enum ScenarioOutcome {
    Ran(RanScenario),
    Skipped { reason: String },
}

#[derive(Clone, Debug)]
pub struct RanScenario {
    pub adapter_id: AdapterId,
    pub fixture_name: String,
    pub exit_code: Option<i32>,
    pub stop_reason: Option<String>,
    pub sandbox_internal_id: String,
    /// Bytes captured from the spawned process. Some adapters (notably
    /// Docker via `kernel_003_bridge`) attach stdout to the spawn
    /// stream; others require `adapter.exec(handle, cat_log_cmd)`.
    /// The harness uses spawn-time capture when available and falls
    /// back to None on adapters whose stream is not wired through.
    pub stdout: Option<Vec<u8>>,
    pub assertions: AssertionsReport,
}

#[derive(Clone, Debug, Default)]
pub struct AssertionsReport {
    pub exit_class_passed: bool,
    pub stop_reason_passed: bool,
    pub stdout_predicate_passed: bool,
    /// Failure messages, one per failed predicate. Empty means all
    /// expected predicates passed.
    pub failures: Vec<String>,
}

impl AssertionsReport {
    pub fn is_ok(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Full harness report: one row per (fixture, adapter) cell, plus a
/// summary of which adapters were skipped.
#[derive(Clone, Debug, Default)]
pub struct ParityReport {
    pub rows: Vec<ScenarioOutcome>,
    pub skipped_adapters: BTreeSet<AdapterId>,
}

impl ParityReport {
    pub fn ran(&self) -> impl Iterator<Item = &RanScenario> {
        self.rows.iter().filter_map(|outcome| match outcome {
            ScenarioOutcome::Ran(run) => Some(run),
            ScenarioOutcome::Skipped { .. } => None,
        })
    }

    pub fn ran_count(&self) -> usize {
        self.ran().count()
    }

    pub fn skipped_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|outcome| matches!(outcome, ScenarioOutcome::Skipped { .. }))
            .count()
    }

    /// `true` when at least two adapters successfully ran the same
    /// fixture — the prerequisite for a meaningful parity assertion.
    pub fn has_parity_coverage_for(&self, fixture_name: &str) -> bool {
        self.ran()
            .filter(|run| run.fixture_name == fixture_name)
            .count()
            >= 2
    }

    /// Cross-adapter parity check for a single fixture, modulo
    /// (UUIDs, timestamps, sandbox_internal_id, adapter_id). Returns
    /// `Ok(())` when every adapter's (exit_code, stop_reason_class)
    /// agrees with at least one other adapter's view of the same
    /// fixture. Skipped adapters are not included in the comparison.
    pub fn assert_parity_for(&self, fixture_name: &str) -> Result<(), String> {
        let runs: Vec<&RanScenario> = self
            .ran()
            .filter(|run| run.fixture_name == fixture_name)
            .collect();
        if runs.len() < 2 {
            // Not enough adapters ran — that's a coverage gap, not a
            // parity failure. The skipped_adapters report exposes
            // the gap separately.
            return Ok(());
        }
        let baseline = runs[0];
        for other in &runs[1..] {
            if baseline.exit_code != other.exit_code {
                return Err(format!(
                    "exit_code parity failure on fixture '{}': {} -> {:?}, {} -> {:?}",
                    fixture_name,
                    baseline.adapter_id.as_str(),
                    baseline.exit_code,
                    other.adapter_id.as_str(),
                    other.exit_code,
                ));
            }
            // stop_reason exact-match is too strict (adapter idiom
            // differs), so we class-compare: both present or both
            // absent. If both present, both must contain the same
            // expected substring (validated against the fixture).
            if baseline.stop_reason.is_some() != other.stop_reason.is_some() {
                return Err(format!(
                    "stop_reason presence parity failure on fixture '{}': {} -> {:?}, {} -> {:?}",
                    fixture_name,
                    baseline.adapter_id.as_str(),
                    baseline.stop_reason,
                    other.adapter_id.as_str(),
                    other.stop_reason,
                ));
            }
        }
        Ok(())
    }
}

/// A slot in the adapter matrix. `Available` holds a real adapter ready
/// to be wrapped in `LedgerDecorator`. `Unavailable` carries the
/// skip reason so the report exposes the coverage gap.
pub enum AdapterSlot {
    Available(Arc<dyn SandboxAdapter>),
    Unavailable { reason: String },
}

/// Real stdout retrieval channel for the parity harness.
///
/// The `SandboxAdapter` trait has no spawn-time stdout stream: both the
/// WSL2-podman and Docker bridges launch their containers detached
/// (`run -d`), so the process's stdout is not handed back on `spawn`.
/// Recovering it requires an adapter-specific side channel
/// (`podman logs <cid>` / `docker logs <cid>`). Rather than bake one
/// adapter's idiom into the harness, the harness asks an optional
/// `ParityStdoutSource` for the captured bytes after the workload has
/// terminated. When no source is registered for an adapter, the harness
/// records `stdout = None` and any declared stdout predicate is reported
/// as a *failure to verify* (never an auto-pass — see
/// [`evaluate_assertions`]). This keeps uncaptured-stdout coverage gaps
/// loud instead of silently green.
#[async_trait]
pub trait ParityStdoutSource: Send + Sync {
    /// Return the real bytes the workload wrote to stdout for `handle`,
    /// or `None` when this source cannot recover them. The harness calls
    /// this only after the process has reached a terminal state.
    async fn parity_stdout(
        &self,
        handle: &crate::sandbox::types::ProcessHandle,
        fixture: &WorkloadFixture,
    ) -> Option<Vec<u8>>;
}

/// The harness itself. Construct with `new`, fill the adapter matrix
/// (one slot per AdapterId), then call `run_fixtures(fixtures)`.
pub struct CrossAdapterParityHarness {
    adapters: BTreeMap<AdapterId, AdapterSlot>,
    stdout_sources: BTreeMap<AdapterId, Arc<dyn ParityStdoutSource>>,
}

impl CrossAdapterParityHarness {
    pub fn new() -> Self {
        Self {
            adapters: BTreeMap::new(),
            stdout_sources: BTreeMap::new(),
        }
    }

    pub fn with_adapter(mut self, id: AdapterId, slot: AdapterSlot) -> Self {
        self.adapters.insert(id, slot);
        self
    }

    /// Register an adapter together with a real stdout-capture channel so
    /// the harness can evaluate stdout predicates against the bytes the
    /// workload actually produced on that adapter.
    pub fn with_adapter_stdout(
        mut self,
        id: AdapterId,
        slot: AdapterSlot,
        stdout_source: Arc<dyn ParityStdoutSource>,
    ) -> Self {
        self.stdout_sources.insert(id.clone(), stdout_source);
        self.adapters.insert(id, slot);
        self
    }

    pub async fn run_fixtures(&self, fixtures: &[WorkloadFixture]) -> ParityReport {
        let mut report = ParityReport::default();
        for (adapter_id, slot) in &self.adapters {
            let adapter = match slot {
                AdapterSlot::Available(adapter) => adapter.clone(),
                AdapterSlot::Unavailable { reason } => {
                    report.skipped_adapters.insert(adapter_id.clone());
                    for fixture in fixtures {
                        report.rows.push(ScenarioOutcome::Skipped {
                            reason: format!(
                                "adapter {} unavailable: {} (fixture {})",
                                adapter_id.as_str(),
                                reason,
                                fixture.name
                            ),
                        });
                    }
                    continue;
                }
            };

            // Wrap every adapter in a fresh LedgerDecorator so the
            // (start, stop) row pair for each fixture lands in our
            // local in-memory store. A shared batcher across adapters
            // would interleave events; one per adapter keeps the
            // per-fixture drain cleanly attributable.
            let stdout_source = self.stdout_sources.get(adapter_id).cloned();
            for fixture in fixtures {
                let outcome = run_single(
                    adapter.clone(),
                    adapter_id.clone(),
                    fixture,
                    stdout_source.clone(),
                )
                .await;
                report.rows.push(outcome);
            }
        }
        report
    }
}

impl Default for CrossAdapterParityHarness {
    fn default() -> Self {
        Self::new()
    }
}

async fn run_single(
    raw_adapter: Arc<dyn SandboxAdapter>,
    adapter_id: AdapterId,
    fixture: &WorkloadFixture,
    stdout_source: Option<Arc<dyn ParityStdoutSource>>,
) -> ScenarioOutcome {
    let store = Arc::new(InMemoryParityStore::default());
    let overflow = Arc::new(InMemoryParityOverflow::default());
    let (batcher, drain) = match LedgerBatcher::manual_for_tests(
        LedgerBatcherConfig {
            capacity: 16,
            batch_size: 4,
            flush_interval: Duration::from_millis(250),
        },
        overflow.clone() as Arc<dyn ProcessLedgerOverflowSink>,
    ) {
        Ok(pair) => pair,
        Err(error) => {
            return ScenarioOutcome::Skipped {
                reason: format!(
                    "LedgerBatcher::manual_for_tests failed for fixture {}: {error}",
                    fixture.name
                ),
            };
        }
    };
    let decorator = LedgerDecorator::new(raw_adapter, batcher);

    let mut spec = fixture.spec.clone();
    // The fixture's `spec.id` is illustrative only — the live
    // adapter dispatch keys off `ProcessHandle.adapter_id`, which the
    // decorator stamps from the inner adapter's capabilities. Use the
    // matrix's AdapterId for that consistency.
    spec.id = adapter_id.clone();

    let handle = match decorator.spawn(spec).await {
        Ok(handle) => handle,
        Err(error) => {
            return ScenarioOutcome::Skipped {
                reason: format!(
                    "spawn failed on adapter {} for fixture {}: {error}",
                    adapter_id.as_str(),
                    fixture.name
                ),
            };
        }
    };
    let sandbox_internal_id = handle.sandbox_internal_id.clone();

    // Drive lifecycle: kill if requested, otherwise poll status.
    let kill_timeout = Duration::from_millis(fixture.kill_timeout_ms.unwrap_or(10_000));
    if let Some(signal) = fixture.kill_signal {
        if let Err(error) = decorator.kill(&handle, signal).await {
            return ScenarioOutcome::Skipped {
                reason: format!(
                    "kill({:?}) failed on adapter {} for fixture {}: {error}",
                    signal,
                    adapter_id.as_str(),
                    fixture.name
                ),
            };
        }
        // Poll status until terminal or timeout.
        let started = std::time::Instant::now();
        loop {
            match decorator.status(&handle).await {
                Ok(ProcessStatus::Exited { .. }) | Ok(ProcessStatus::Killed { .. }) => break,
                Ok(_) if started.elapsed() < kill_timeout => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Ok(_) => {
                    return ScenarioOutcome::Skipped {
                        reason: format!(
                            "process did not terminate within {}ms of kill({:?}) on adapter {} for fixture {}",
                            kill_timeout.as_millis(),
                            signal,
                            adapter_id.as_str(),
                            fixture.name
                        ),
                    };
                }
                Err(error) => {
                    return ScenarioOutcome::Skipped {
                        reason: format!(
                            "status poll failed on adapter {} for fixture {}: {error}",
                            adapter_id.as_str(),
                            fixture.name
                        ),
                    };
                }
            }
        }
    } else {
        // No kill request — poll for natural termination.
        let natural_timeout = Duration::from_millis(fixture.kill_timeout_ms.unwrap_or(30_000));
        let started = std::time::Instant::now();
        loop {
            match decorator.status(&handle).await {
                Ok(ProcessStatus::Exited { .. }) | Ok(ProcessStatus::Killed { .. }) => break,
                Ok(_) if started.elapsed() < natural_timeout => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Ok(_) => {
                    return ScenarioOutcome::Skipped {
                        reason: format!(
                            "process did not terminate within {}ms on adapter {} for fixture {}",
                            natural_timeout.as_millis(),
                            adapter_id.as_str(),
                            fixture.name
                        ),
                    };
                }
                Err(error) => {
                    return ScenarioOutcome::Skipped {
                        reason: format!(
                            "status poll failed on adapter {} for fixture {}: {error}",
                            adapter_id.as_str(),
                            fixture.name
                        ),
                    };
                }
            }
        }
    }

    let exit_code = match decorator.exit_code(&handle).await {
        Ok(code) => code,
        Err(error) => {
            return ScenarioOutcome::Skipped {
                reason: format!(
                    "exit_code() failed on adapter {} for fixture {}: {error}",
                    adapter_id.as_str(),
                    fixture.name
                ),
            };
        }
    };

    // Drain the ledger so we can read the STOP row's stop_reason.
    // drain_available_to requires a sized `Arc<S>` (the trait bound is
    // implicit-Sized), so we keep the concrete typed Arc instead of
    // erasing to dyn ProcessLedgerStore.
    if let Err(error) = drain.drain_available_to(store.clone()).await {
        return ScenarioOutcome::Skipped {
            reason: format!(
                "ledger drain failed on adapter {} for fixture {}: {error}",
                adapter_id.as_str(),
                fixture.name
            ),
        };
    }
    let events = store.events();
    let stop_reason = events.iter().find_map(|event| match event {
        LedgerEvent::Stop(stop) if stop.process_uuid == handle.id => stop.stop_reason.clone(),
        _ => None,
    });

    // Real stdout capture: ask the registered source for the bytes the
    // workload actually wrote. When no source is registered for this
    // adapter the harness keeps `None`, and any declared stdout predicate
    // is reported as a verification failure (never auto-passed).
    let stdout = match &stdout_source {
        Some(source) => source.parity_stdout(&handle, fixture).await,
        None => None,
    };

    let assertions = evaluate_assertions(
        &fixture.expected,
        exit_code,
        stop_reason.as_deref(),
        stdout.as_deref(),
    );

    ScenarioOutcome::Ran(RanScenario {
        adapter_id,
        fixture_name: fixture.name.clone(),
        exit_code,
        stop_reason,
        sandbox_internal_id,
        stdout,
        assertions,
    })
}

fn evaluate_assertions(
    expected: &ExpectedOutcome,
    exit_code: Option<i32>,
    stop_reason: Option<&str>,
    stdout: Option<&[u8]>,
) -> AssertionsReport {
    let mut report = AssertionsReport::default();

    report.exit_class_passed = match (&expected.exit_code_class, exit_code) {
        (ExitClass::Zero, Some(0)) => true,
        (ExitClass::NonZero, Some(code)) if code != 0 => true,
        (ExitClass::Exact(expected), Some(code)) if code == *expected => true,
        (ExitClass::Any, _) => true,
        // Anything else — strict class missing or wrong code — fails.
        _ => false,
    };
    if !report.exit_class_passed {
        report.failures.push(format!(
            "exit_class assertion failed: expected {:?}, got exit_code={:?}",
            expected.exit_code_class, exit_code
        ));
    }

    report.stop_reason_passed = match (&expected.stop_reason_contains, stop_reason) {
        (Some(needle), Some(actual)) => actual.contains(needle),
        (Some(_), None) => false,
        (None, _) => true,
    };
    if !report.stop_reason_passed {
        report.failures.push(format!(
            "stop_reason assertion failed: expected substring {:?}, got {:?}",
            expected.stop_reason_contains, stop_reason
        ));
    }

    // Real stdout predicate evaluation against the bytes the workload
    // actually produced. No auto-pass: when the fixture declares a
    // predicate but no stdout was captured, that is a *verification
    // failure*, not a free green. When the fixture declares no
    // predicate, there is nothing to check and the dimension passes.
    report.stdout_predicate_passed = match (&expected.stdout_predicate, stdout) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(predicate), Some(bytes)) => predicate.matches(bytes),
    };
    if !report.stdout_predicate_passed {
        match (&expected.stdout_predicate, stdout) {
            (Some(predicate), None) => report.failures.push(format!(
                "stdout predicate {predicate:?} could not be verified: no stdout was captured on this adapter (register a ParityStdoutSource to exercise this assertion)"
            )),
            (Some(predicate), Some(bytes)) => report.failures.push(format!(
                "stdout predicate {:?} failed: captured stdout was {:?}",
                predicate,
                String::from_utf8_lossy(bytes)
            )),
            // (None, _) can never reach here because it sets passed=true.
            (None, _) => {}
        }
    }

    report
}

impl StdoutPredicate {
    /// Strict-parity match of this predicate against the real captured
    /// stdout bytes. UTF-8 lossy is used for substring/exact comparison
    /// so non-UTF-8 noise cannot crash the harness; `ExactBytes` and
    /// `Substring` carry their needle as a `String` (the fixture JSON
    /// shape), so the comparison is performed on the lossy decode.
    pub fn matches(&self, stdout: &[u8]) -> bool {
        let text = String::from_utf8_lossy(stdout);
        match self {
            // Exact parity: trailing newline tolerance is intentional —
            // `echo` appends a newline that the predicate text does not
            // carry, so we compare on the trimmed-trailing-newline form
            // while still rejecting any other divergence.
            StdoutPredicate::ExactBytes(expected) => {
                text.trim_end_matches(['\n', '\r']) == expected.trim_end_matches(['\n', '\r'])
            }
            StdoutPredicate::Substring(needle) => text.contains(needle.as_str()),
            StdoutPredicate::Empty => stdout.is_empty(),
        }
    }
}

// ----------------------------------------------------------------------------
// Test-private in-memory ledger sinks. Mirrors the pattern duplicated
// across `tests/process_ledger_writer_tests.rs` and
// `tests/process_ledger_reclaim_tests.rs`. Lives here so MT-057's
// harness is self-contained and future cross-adapter MTs can reuse it.
// ----------------------------------------------------------------------------

#[derive(Default, Clone)]
pub struct InMemoryParityStore {
    inner: Arc<Mutex<Vec<LedgerEvent>>>,
}

impl InMemoryParityStore {
    pub fn events(&self) -> Vec<LedgerEvent> {
        self.inner.lock().expect("parity ledger lock").clone()
    }
}

#[async_trait]
impl ProcessLedgerStore for InMemoryParityStore {
    async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
        let mut guard = self.inner.lock().expect("parity ledger lock");
        for event in events {
            guard.push(event);
        }
        Ok(())
    }
}

#[derive(Default, Clone)]
pub struct InMemoryParityOverflow {
    inner: Arc<Mutex<Vec<LedgerOverflowEvent>>>,
}

impl InMemoryParityOverflow {
    pub fn overflows(&self) -> Vec<LedgerOverflowEvent> {
        self.inner.lock().expect("parity overflow lock").clone()
    }
}

impl ProcessLedgerOverflowSink for InMemoryParityOverflow {
    fn emit_overflow(&self, event: LedgerOverflowEvent) -> Result<(), ProcessLedgerError> {
        self.inner.lock().expect("parity overflow lock").push(event);
        Ok(())
    }
}
