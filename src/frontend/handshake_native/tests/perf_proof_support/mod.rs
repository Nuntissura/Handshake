//! WP-KERNEL-012 MT-045 — shared perf-proof harness for the three `test_perf_large_*.rs` suites
//! (cluster E8, Large-Document & Large-Codebase Performance Proof). Lives in a `tests/` SUBDIRECTORY so
//! Cargo does not compile it as a standalone test binary (only top-level `tests/*.rs` are test targets);
//! each perf suite pulls it in with `mod perf_proof_support;`.
//!
//! ## What it owns
//!
//! - [`Budget`] — resolves a scenario's latency/memory budget from a `PERF_BUDGET_*` env var (RISK-1 /
//!   CTRL-1: a slow host widens the ceiling without a code change) and records the MEASURED value, not
//!   just PASS, into both the contract-authoritative manifest and an external machine-readable receipt.
//! - [`record`] — atomically updates the matching `perf_manifest.json` row on every attempt and writes
//!   additional immutable/current receipts under `Handshake_Artifacts/wp-kernel-012/mt-045/measurements/`.
//! - [`measure_rss_delta_median`] — measures the process RSS delta (after a workload minus before) as
//!   the MEDIAN of 3 runs via the `sysinfo` crate (RISK-5 / CTRL-5: RSS is noisy — allocator page
//!   pre-reservation varies run to run, so a single sample near the budget edge is unreliable).
//! - [`assert_no_local_artifact_dir`] — fails the suite if a repo-local `test_output/` or
//!   `tests/screenshots/` directory exists (CX-212E artifact hygiene). The perf suites write NO image
//!   artifacts (they emit only the external manifest record), but the guard is called so a future
//!   regression that adds a repo-local artifact dir is caught.
//! - [`skip_all`] — the explicit whole-suite operator skip. PostgreSQL-binding scenarios run by default
//!   through the shared managed product-backend fixture (no mocks and no operator-preseeded rows).
//!
//! This module is only ever compiled into the test binaries (it lives under `tests/`), so it never
//! reaches the product binary.

#![allow(dead_code)] // each suite uses a subset of the helpers; the others are not dead in aggregate.

use std::cell::RefCell;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const EXPECTED_SCENARIO_IDS: [&str; 20] = [
    "LC-01", "LC-02", "LC-03", "LC-04", "LC-05", "LC-06", "LC-07", "LC-08", "LR-01", "LR-02",
    "LR-03", "LR-04", "LR-05", "LR-06", "LR-07", "LK-01", "LK-02", "LK-03", "LK-04", "LK-05",
];

// ── Budget resolution + measured-value recording ─────────────────────────────────────────────────

/// A resolved budget for one scenario: the effective ceiling (ms or MB) plus provenance. The ceiling is
/// read from `env_var` and otherwise remains the exact contract default in every build profile.
pub struct Budget {
    pub scenario_id: &'static str,
    /// The effective gate ceiling (after an explicit env override, or the exact default).
    pub ceiling: u128,
    /// The contract default (the SHIPPED-binary target), unmodified — recorded for transparency.
    pub contract_default: u128,
    pub env_var: &'static str,
    pub overridden: bool,
}

impl Budget {
    /// Resolve the effective ceiling. An explicit operator/CI override wins; otherwise the exact contract
    /// default is binding in every build profile. Debug builds may be slower, but silently widening the
    /// acceptance gate would turn a hard performance proof into a profile-dependent claim.
    pub fn resolve(scenario_id: &'static str, env_var: &'static str, default: u128) -> Self {
        if let Some(v) = std::env::var(env_var)
            .ok()
            .and_then(|v| v.trim().parse::<u128>().ok())
        {
            return Budget {
                scenario_id,
                ceiling: v,
                contract_default: default,
                env_var,
                overridden: true,
            };
        }
        Budget {
            scenario_id,
            ceiling: default,
            contract_default: default,
            env_var,
            overridden: false,
        }
    }

    /// `true` when `measured <= ceiling`. Use this to assert; on PASS the caller records via [`record`].
    pub fn passes(&self, measured: u128) -> bool {
        measured <= self.ceiling
    }

    /// A short provenance suffix for the printed PASS line, naming the build profile + effective ceiling.
    pub fn provenance(&self) -> String {
        if self.overridden {
            format!("ceiling {} ms via {} override", self.ceiling, self.env_var)
        } else {
            format!("ceiling {} ms (hard contract default)", self.ceiling)
        }
    }
}

/// One measured value written into a terminal receipt.
pub fn measurement(name: &str, value: f64, unit: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "value": (value * 1000.0).round() / 1000.0,
        "unit": unit,
    })
}

#[derive(Clone)]
struct FailurePayload {
    measurements: serde_json::Value,
    evidence: serde_json::Value,
}

impl FailurePayload {
    fn empty() -> Self {
        Self {
            measurements: serde_json::json!([]),
            evidence: serde_json::json!({}),
        }
    }

    fn measured(measurements: serde_json::Value, evidence: serde_json::Value) -> Self {
        assert!(
            measurements
                .as_array()
                .is_some_and(|values| !values.is_empty()),
            "staged MT-045 failure evidence must retain at least one real measurement"
        );
        Self {
            measurements,
            evidence,
        }
    }
}

fn stage_failure_payload(
    slot: &RefCell<FailurePayload>,
    measurements: serde_json::Value,
    evidence: serde_json::Value,
) {
    slot.replace(FailurePayload::measured(measurements, evidence));
}

fn snapshot_failure_payload(slot: &RefCell<FailurePayload>) -> FailurePayload {
    slot.borrow().clone()
}

/// A fail-closed scenario attempt. Construction atomically replaces any prior current receipt with
/// `RUNNING`; a terminal method replaces it with `PASS`/`SKIPPED`; unwinding or an early return writes
/// `FAIL`. Thus a stale PASS can never remain authoritative after a new attempt starts.
pub struct ScenarioAttempt {
    scenario_id: String,
    proof_id: String,
    attempt_id: String,
    suite_run_id: String,
    started_at: String,
    budgets: serde_json::Value,
    staged_failure: RefCell<FailurePayload>,
    terminal: bool,
}

impl ScenarioAttempt {
    pub fn begin(scenario_id: &str, proof_id: &str, budgets: &[(&str, &Budget, &str)]) -> Self {
        // Adversarial review B5: run the repo-local artifact-hygiene guard on EVERY scenario attempt (not
        // only the two scenarios that called it explicitly), so any scenario that regresses into writing a
        // repo-local artifact directory is caught universally.
        assert_no_local_artifact_dir();
        let attempt_id = uuid::Uuid::new_v4().to_string();
        let started_at = chrono::Utc::now().to_rfc3339();
        let suite_run_id = begin_scenario_run(scenario_id, proof_id, &attempt_id, &started_at);
        let attempt = Self {
            scenario_id: scenario_id.to_owned(),
            proof_id: proof_id.to_owned(),
            attempt_id,
            suite_run_id,
            started_at,
            budgets: serde_json::Value::Array(
                budgets
                    .iter()
                    .map(|(name, budget, unit)| {
                        serde_json::json!({
                            "metric": name,
                            "unit": unit,
                            "contract_default": budget.contract_default,
                            "effective_ceiling": budget.ceiling,
                            "override_env": budget.env_var,
                            "override_applied": budget.overridden,
                        })
                    })
                    .collect(),
            ),
            staged_failure: RefCell::new(FailurePayload::empty()),
            terminal: false,
        };
        attempt.write(
            "RUNNING",
            serde_json::json!([]),
            serde_json::json!({}),
            None,
        );
        attempt
    }

    /// Start a fresh attempt and apply the suite-wide operator skip only after the `RUNNING` receipt
    /// has superseded any prior terminal result. Callers must use this as their first fallible/gated
    /// lifecycle operation, before backend health checks, fixture setup, artifact assertions, or
    /// measured work. A requested skip is terminal here, so the caller can only return `None`; no
    /// callsite can accidentally leave `RUNNING` behind or preserve a stale `PASS`.
    pub fn begin_or_skip(
        scenario_id: &str,
        proof_id: &str,
        budgets: &[(&str, &Budget, &str)],
    ) -> Option<Self> {
        let attempt = Self::begin(scenario_id, proof_id, budgets);
        if skip_all() {
            attempt.skipped("SKIP_PERF_TESTS=1");
            None
        } else {
            Some(attempt)
        }
    }

    pub fn pass(mut self, measurements: serde_json::Value, evidence: serde_json::Value) {
        self.write("PASS", measurements, evidence, None);
        self.terminal = true;
    }

    /// Stage real measurements immediately after a measured operation and before any correctness or
    /// budget assertion. If a later assertion unwinds, [`Drop`] publishes this exact payload rather than
    /// replacing the measured result with an empty generic failure.
    pub fn stage(&self, measurements: serde_json::Value, evidence: serde_json::Value) {
        stage_failure_payload(&self.staged_failure, measurements, evidence);
    }

    /// Persist terminal FAIL evidence before the caller panics on a measured budget or runtime gate.
    /// This keeps the actual measurement authoritative instead of letting `Drop` replace it with an
    /// empty generic abort receipt during unwinding.
    pub fn fail(
        mut self,
        measurements: serde_json::Value,
        evidence: serde_json::Value,
        reason: &str,
    ) {
        self.write("FAIL", measurements, evidence, Some(reason));
        self.terminal = true;
    }

    pub fn skipped(mut self, reason: &str) {
        self.write(
            "SKIPPED",
            serde_json::json!([]),
            serde_json::json!({}),
            Some(reason),
        );
        self.terminal = true;
    }

    fn write(
        &self,
        status: &str,
        measurements: serde_json::Value,
        evidence: serde_json::Value,
        terminal_reason: Option<&str>,
    ) {
        let run_measurements = measurements.clone();
        write_entry(
            &self.scenario_id,
            &self.proof_id,
            &self.attempt_id,
            &self.suite_run_id,
            &self.started_at,
            &self.budgets,
            measurements,
            evidence,
            status,
            terminal_reason,
        );
        if status != "RUNNING" {
            // A terminal state may reference its attempt receipt only after that receipt is durable.
            // If receipt IO fails, begin_scenario_run's latest projection remains RUNNING and therefore
            // fails closed instead of publishing an immutable completion with missing evidence.
            update_scenario_run(
                &self.suite_run_id,
                &self.scenario_id,
                &self.proof_id,
                &self.attempt_id,
                status,
                terminal_reason,
                &run_measurements,
            );
        }
    }
}

impl Drop for ScenarioAttempt {
    fn drop(&mut self) {
        if !self.terminal {
            let staged = snapshot_failure_payload(&self.staged_failure);
            // RUNNING has already invalidated prior PASS state. This best-effort terminal overwrite avoids
            // a double-panic abort if the filesystem itself failed while another assertion was unwinding.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.write(
                    "FAIL",
                    staged.measurements,
                    staged.evidence,
                    Some("scenario_aborted_before_terminal_receipt"),
                );
            }));
        }
    }
}

/// Compatibility entry point for old callers. New proof scenarios must construct [`ScenarioAttempt`]
/// before assertions; this function remains only for source compatibility during migration.
pub fn record(scenario_id: &str, measured_value: f64, status: &str) {
    let attempt = ScenarioAttempt::begin(scenario_id, "primary", &[]);
    if status == "PASS" {
        attempt.pass(
            serde_json::json!([measurement(
                "legacy_measurement",
                measured_value,
                "unspecified"
            )]),
            serde_json::json!({"legacy_record_api": true}),
        );
    } else {
        attempt.skipped(status);
    }
}

#[allow(clippy::too_many_arguments)]
fn write_entry(
    scenario_id: &str,
    proof_id: &str,
    attempt_id: &str,
    suite_run_id: &str,
    started_at: &str,
    budgets: &serde_json::Value,
    measurements: serde_json::Value,
    evidence: serde_json::Value,
    status: &str,
    terminal_reason: Option<&str>,
) {
    let directory = external_artifact_root().join("mt-045").join("measurements");
    std::fs::create_dir_all(&directory)
        .unwrap_or_else(|error| panic!("perf receipt directory {directory:?}: {error}"));
    let lock_path = directory.join("measurements.lock");

    let _guard = match FileLock::acquire(&lock_path, Duration::from_secs(15)) {
        Some(g) => g,
        None => panic!("perf receipt lock {lock_path:?} unavailable for {scenario_id}"),
    };

    let mut system = sysinfo::System::new_all();
    system.refresh_all();
    let receipt = serde_json::json!({
        "schema_id": "hsk.wp_kernel_012.performance_measurement@2",
        "work_packet_id": "WP-KERNEL-012",
        "micro_task_id": "MT-045",
        "scenario_id": scenario_id,
        "proof_id": proof_id,
        "attempt_id": attempt_id,
        "suite_run_id": suite_run_id,
        "started_at": started_at,
        "status": status,
        "terminal_reason": terminal_reason,
        "budgets": budgets,
        "measurements": measurements,
        "evidence": evidence,
        "measured_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "recorded_at": chrono::Utc::now().to_rfc3339(),
        "process_id": std::process::id(),
        "manifest_reference": {
            "path": "tests/perf_proof/perf_manifest.json",
            "scenario_id": scenario_id,
            "authority": "contract_authoritative_runtime_updated",
            "receipt_role": "additional_immutable_and_current_evidence"
        },
        "runtime": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "logical_cpus": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
            "total_memory_bytes": system.total_memory(),
            "rust_debug_assertions": cfg!(debug_assertions),
        }
    });
    let mut out = serde_json::to_string_pretty(&receipt)
        .unwrap_or_else(|error| panic!("serialize perf receipt {scenario_id}: {error}"));
    out.push('\n');
    let receipt_stem = if proof_id == "primary" {
        scenario_id.to_ascii_lowercase()
    } else {
        format!(
            "{}--{}",
            scenario_id.to_ascii_lowercase(),
            proof_id.to_ascii_lowercase()
        )
    };
    let path = directory.join(format!("{receipt_stem}.json"));
    let attempts = directory.join("attempts");
    std::fs::create_dir_all(&attempts)
        .unwrap_or_else(|error| panic!("perf attempt directory {attempts:?}: {error}"));
    // Preserve every lifecycle transition as a distinct immutable event. RUNNING and its later
    // PASS/FAIL share an attempt id, so status is part of the filename rather than overwriting the
    // first event. Commit history before publishing `current`, ensuring a crash can never expose a
    // current verdict with no retained attempt evidence.
    let status_slug = status.to_ascii_lowercase();
    let history_path = attempts.join(format!("{receipt_stem}--{attempt_id}--{status_slug}.json"));
    let history_tmp = attempts.join(format!(
        ".{receipt_stem}--{attempt_id}--{status_slug}.tmp.{}.json",
        std::process::id()
    ));
    write_synced_new(&history_tmp, out.as_bytes())
        .unwrap_or_else(|error| panic!("write perf attempt history temp {history_tmp:?}: {error}"));
    assert!(
        !history_path.exists(),
        "immutable perf attempt receipt already exists: {history_path:?}"
    );
    atomic_replace_file(&history_tmp, &history_path).unwrap_or_else(|error| {
        panic!("commit immutable perf attempt history {history_path:?}: {error}")
    });

    let tmp_path = directory.join(format!(
        ".{receipt_stem}--{attempt_id}--{status_slug}.tmp.{}.json",
        std::process::id()
    ));
    write_synced_new(&tmp_path, out.as_bytes())
        .unwrap_or_else(|error| panic!("write perf receipt temp {tmp_path:?}: {error}"));
    atomic_replace_file(&tmp_path, &path)
        .unwrap_or_else(|error| panic!("atomically commit perf receipt {scenario_id}: {error}"));
}

fn expected_scenario_ids() -> HashSet<String> {
    EXPECTED_SCENARIO_IDS
        .iter()
        .map(|scenario_id| (*scenario_id).to_owned())
        .collect()
}

fn expected_proof_ids(scenario_id: &str) -> HashSet<&'static str> {
    if scenario_id == "LR-05" {
        HashSet::from(["linear-50", "cyclic-5"])
    } else {
        HashSet::from(["primary"])
    }
}

fn begin_scenario_run(
    scenario_id: &str,
    proof_id: &str,
    attempt_id: &str,
    started_at: &str,
) -> String {
    assert!(
        EXPECTED_SCENARIO_IDS.contains(&scenario_id),
        "MT-045 attempt names unknown scenario {scenario_id}"
    );
    assert!(
        expected_proof_ids(scenario_id).contains(proof_id),
        "MT-045 attempt names unexpected proof {scenario_id}/{proof_id}"
    );
    let directory = external_artifact_root().join("mt-045").join("measurements");
    std::fs::create_dir_all(&directory)
        .unwrap_or_else(|error| panic!("perf run directory {directory:?}: {error}"));
    let lock_path = directory.join("run-state.lock");
    let _guard = FileLock::acquire(&lock_path, Duration::from_secs(15))
        .unwrap_or_else(|| panic!("perf run-state lock unavailable for {scenario_id}/{proof_id}"));
    let state_path = directory.join("current-run.json");
    let requested_run = std::env::var("HSK_MT045_RUN_ID")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let existing = std::fs::read_to_string(&state_path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    let existing_run_id = existing
        .as_ref()
        .and_then(|state| state.get("run_id"))
        .and_then(serde_json::Value::as_str);
    let existing_completed = existing
        .as_ref()
        .and_then(|state| state.get("completed_at"))
        .is_some_and(|value| !value.is_null());
    let existing_proof_terminal = existing
        .as_ref()
        .and_then(|state| {
            state.pointer(&format!(
                "/scenarios/{scenario_id}/proofs/{proof_id}/status"
            ))
        })
        .and_then(serde_json::Value::as_str)
        .is_some_and(|status| status != "RUNNING");
    assert!(
        !(existing_proof_terminal && requested_run.as_deref() == existing_run_id),
        "MT-045 run {existing_run_id:?} already finalized {scenario_id}/{proof_id}; retry with a new HSK_MT045_RUN_ID"
    );
    let must_start = requested_run
        .as_deref()
        .is_some_and(|requested| existing_run_id != Some(requested))
        || existing_run_id.is_none()
        || existing_completed
        || existing_proof_terminal;
    let run_id = if must_start {
        requested_run.unwrap_or_else(|| format!("MT045-RUN-{}", uuid::Uuid::now_v7().simple()))
    } else {
        existing_run_id.expect("checked present").to_owned()
    };
    let immutable_run_path = directory.join("runs").join(format!("{run_id}.json"));
    assert!(
        !must_start || !immutable_run_path.exists(),
        "MT-045 run id {run_id} already has an immutable completed summary; choose a new HSK_MT045_RUN_ID"
    );
    let mut state = if must_start {
        serde_json::json!({
            "schema_id": "hsk.wp_kernel_012.performance_run@1",
            "work_packet_id": "WP-KERNEL-012",
            "micro_task_id": "MT-045",
            "run_id": run_id,
            "started_at": started_at,
            "status": "RUNNING",
            "expected_scenario_count": 20,
            "manifest_reference": "tests/perf_proof/perf_manifest.json",
            "manifest_semantics": "contract_authoritative_runtime_updated_on_every_attempt",
            "scenarios": {},
        })
    } else {
        existing.expect("checked present")
    };
    state["updated_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
    state["scenarios"][scenario_id]["proofs"][proof_id] = serde_json::json!({
        "attempt_id": attempt_id,
        "started_at": started_at,
        "status": "RUNNING",
        "measurements": [],
    });
    refresh_scenario_status(&mut state, scenario_id);
    refresh_run_projection(&mut state);
    if must_start {
        assert_eq!(
            state["status"].as_str(),
            Some("RUNNING"),
            "a fresh MT-045 run must publish current-run.json as RUNNING"
        );
    }
    // Invalidate a previous PASS on BOTH mutable projections before any other fallible publication. If
    // manifest reset or a preflight proof then fails, neither operator-facing projection can retain a
    // prior PASS. Read both files back before proceeding so the invalidation itself is a proof gate.
    publish_and_assert_incomplete_projections(&directory, &state);
    if must_start {
        // A run is an exact 20-scenario lineage unit. Reset and stamp every manifest row before a
        // single scenario can publish a current result, preventing 19 stale PASS rows from combining
        // with one result from this run.
        reset_manifest_for_run(&run_id);
        state["file_lock_contract_proof"] =
            assert_file_lock_contention_and_recovery(&directory.join("file-lock-contract.lock"));
        state["failure_measurement_contract_proof"] = assert_staged_failure_measurement_retention();
        publish_and_assert_incomplete_projections(&directory, &state);
    }
    update_manifest_from_run_state(&state, scenario_id);
    write_json_atomic(&state_path, &state);
    assert_incomplete_projections(
        &directory,
        &run_id,
        state["status"].as_str().unwrap_or("FAIL"),
    );
    run_id
}

fn update_scenario_run(
    run_id: &str,
    scenario_id: &str,
    proof_id: &str,
    attempt_id: &str,
    status: &str,
    terminal_reason: Option<&str>,
    measurements: &serde_json::Value,
) {
    let directory = external_artifact_root().join("mt-045").join("measurements");
    let lock_path = directory.join("run-state.lock");
    let _guard = FileLock::acquire(&lock_path, Duration::from_secs(15))
        .unwrap_or_else(|| panic!("perf run-state lock unavailable for {scenario_id}/{proof_id}"));
    let state_path = directory.join("current-run.json");
    let text = std::fs::read_to_string(&state_path)
        .unwrap_or_else(|error| panic!("read MT-045 run state {state_path:?}: {error}"));
    let mut state: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("parse MT-045 run state {state_path:?}: {error}"));
    assert_eq!(
        state["run_id"].as_str(),
        Some(run_id),
        "{scenario_id}/{proof_id} attempted to finalize a stale MT-045 run"
    );
    assert_eq!(
        state["scenarios"][scenario_id]["proofs"][proof_id]["attempt_id"].as_str(),
        Some(attempt_id),
        "{scenario_id}/{proof_id} attempted to finalize a superseded attempt"
    );
    let proof = &mut state["scenarios"][scenario_id]["proofs"][proof_id];
    proof["status"] = serde_json::json!(status);
    proof["terminal_reason"] = serde_json::json!(terminal_reason);
    proof["measurements"] = measurements.clone();
    proof["recorded_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
    let receipt_stem = if proof_id == "primary" {
        scenario_id.to_ascii_lowercase()
    } else {
        format!(
            "{}--{}",
            scenario_id.to_ascii_lowercase(),
            proof_id.to_ascii_lowercase()
        )
    };
    let attempt_receipt_path = format!(
        "attempts/{receipt_stem}--{attempt_id}--{}.json",
        status.to_ascii_lowercase()
    );
    let durable_receipt_path = directory.join(&attempt_receipt_path);
    let durable_receipt: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&durable_receipt_path).unwrap_or_else(|error| {
            panic!("read durable attempt receipt {durable_receipt_path:?}: {error}")
        }),
    )
    .unwrap_or_else(|error| {
        panic!("parse durable attempt receipt {durable_receipt_path:?}: {error}")
    });
    assert_eq!(durable_receipt["attempt_id"].as_str(), Some(attempt_id));
    assert_eq!(durable_receipt["suite_run_id"].as_str(), Some(run_id));
    assert_eq!(durable_receipt["scenario_id"].as_str(), Some(scenario_id));
    assert_eq!(durable_receipt["proof_id"].as_str(), Some(proof_id));
    assert_eq!(durable_receipt["status"].as_str(), Some(status));
    proof["attempt_receipt_path"] = serde_json::json!(attempt_receipt_path);

    refresh_scenario_status(&mut state, scenario_id);
    state["updated_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
    let complete = refresh_run_projection(&mut state);
    if state["status"].as_str() != Some("PASS") {
        // Failure publication outranks manifest/current-state maintenance: even if a later write fails,
        // latest can no longer expose a stale PASS. RUNNING is likewise refreshed with this proof state.
        write_json_atomic(&directory.join("latest-run-summary.json"), &state);
    }
    update_manifest_from_run_state(&state, scenario_id);
    if complete {
        if state["status"].as_str() == Some("PASS") {
            assert_manifest_all_pass_current(&state);
        }
        state["completed_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
        let runs = directory.join("runs");
        std::fs::create_dir_all(&runs)
            .unwrap_or_else(|error| panic!("create MT-045 run summaries {runs:?}: {error}"));
        write_json_immutable(&runs.join(format!("{run_id}.json")), &state);
        write_json_atomic(&directory.join("latest-run-summary.json"), &state);
        write_json_atomic(&state_path, &state);
        return;
    }
    write_json_atomic(&state_path, &state);
    assert_incomplete_projections(
        &directory,
        run_id,
        state["status"].as_str().unwrap_or("FAIL"),
    );
}

fn refresh_scenario_status(state: &mut serde_json::Value, scenario_id: &str) {
    let (actual, expected, unexpected, any_failure, all_expected_terminal, all_expected_pass) = {
        let proofs = state["scenarios"][scenario_id]["proofs"]
            .as_object()
            .unwrap_or_else(|| panic!("MT-045 scenario {scenario_id} proofs must be an object"));
        let expected: HashSet<String> = expected_proof_ids(scenario_id)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let actual: HashSet<String> = proofs.keys().cloned().collect();
        let unexpected = !actual.is_subset(&expected);
        let any_failure = proofs.values().any(|proof| {
            proof
                .get("status")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|status| status != "RUNNING" && status != "PASS")
        });
        let all_expected_terminal = actual == expected
            && proofs.values().all(|proof| {
                proof
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|status| status != "RUNNING")
            });
        let all_expected_pass = actual == expected
            && proofs.values().all(|proof| {
                proof.get("status").and_then(serde_json::Value::as_str) == Some("PASS")
            });
        (
            actual,
            expected,
            unexpected,
            any_failure,
            all_expected_terminal,
            all_expected_pass,
        )
    };
    let scenario_status = if unexpected || any_failure {
        "FAIL"
    } else if all_expected_pass {
        "PASS"
    } else {
        "RUNNING"
    };
    state["scenarios"][scenario_id]["status"] = serde_json::json!(scenario_status);
    state["scenarios"][scenario_id]["exact_proof_set"] = serde_json::json!(actual == expected);
    state["scenarios"][scenario_id]["all_expected_proofs_terminal"] =
        serde_json::json!(all_expected_terminal);
}

/// Refresh the mutable latest/current projection. Early proof failure projects FAIL immediately, but
/// immutable completion requires the exact 20-scenario set to be terminal. This separation is the
/// stale-PASS regression guard: latest may be RUNNING/FAIL while no completed run file is written.
fn refresh_run_projection(state: &mut serde_json::Value) -> bool {
    let expected = expected_scenario_ids();
    let (exact_ids, terminal_count, any_failure, all_pass) = {
        let scenarios = state["scenarios"]
            .as_object()
            .expect("MT-045 run scenarios object");
        let actual: HashSet<String> = scenarios.keys().cloned().collect();
        let exact_ids = actual == expected;
        let terminal_count = expected
            .iter()
            .filter(|scenario_id| {
                scenarios.get(*scenario_id).is_some_and(|scenario| {
                    scenario["all_expected_proofs_terminal"].as_bool() == Some(true)
                        && scenario["status"]
                            .as_str()
                            .is_some_and(|status| status == "PASS" || status == "FAIL")
                })
            })
            .count();
        let any_failure = !actual.is_subset(&expected)
            || scenarios.values().any(|scenario| {
                scenario.get("status").and_then(serde_json::Value::as_str) == Some("FAIL")
            });
        let all_pass = exact_ids
            && expected.iter().all(|scenario_id| {
                scenarios
                    .get(scenario_id)
                    .and_then(|scenario| scenario.get("status"))
                    .and_then(serde_json::Value::as_str)
                    == Some("PASS")
            });
        (exact_ids, terminal_count, any_failure, all_pass)
    };
    let lr05_exact = state
        .pointer("/scenarios/LR-05/exact_proof_set")
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    let lr05_cycle_complete = state
        .pointer("/scenarios/LR-05/proofs/cyclic-5/status")
        .and_then(serde_json::Value::as_str)
        == Some("PASS");
    let complete = exact_ids && terminal_count == EXPECTED_SCENARIO_IDS.len();
    let status = if any_failure {
        "FAIL"
    } else if complete && all_pass && lr05_exact && lr05_cycle_complete {
        "PASS"
    } else if complete {
        "FAIL"
    } else {
        "RUNNING"
    };
    state["status"] = serde_json::json!(status);
    state["terminal_scenario_count"] = serde_json::json!(terminal_count);
    state["exact_scenario_set"] = serde_json::json!(exact_ids);
    state["all_scenarios_passed"] = serde_json::json!(all_pass);
    state["lr05_exact_proof_set"] = serde_json::json!(lr05_exact);
    state["lr05_cycle_proof_complete"] = serde_json::json!(lr05_cycle_complete);
    assert!(
        complete || status != "PASS",
        "an incomplete MT-045 run must never project stale PASS"
    );
    complete
}

fn publish_and_assert_incomplete_projections(directory: &Path, state: &serde_json::Value) {
    let run_id = state["run_id"]
        .as_str()
        .expect("incomplete MT-045 projection must carry run_id");
    let status = state["status"].as_str().unwrap_or("FAIL");
    let current_path = directory.join("current-run.json");
    write_json_atomic(&current_path, state);
    assert_incomplete_projection(&current_path, run_id, status);
    write_json_atomic(&directory.join("latest-run-summary.json"), state);
    assert_incomplete_projections(directory, run_id, status);
}

fn assert_incomplete_projections(directory: &Path, run_id: &str, status: &str) {
    for name in ["latest-run-summary.json", "current-run.json"] {
        let path = directory.join(name);
        assert_incomplete_projection(&path, run_id, status);
    }
}

fn assert_incomplete_projection(path: &Path, run_id: &str, status: &str) {
    let projection: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("read MT-045 projection {path:?}: {error}")),
    )
    .unwrap_or_else(|error| panic!("parse MT-045 projection {path:?}: {error}"));
    assert_eq!(projection["run_id"].as_str(), Some(run_id));
    assert_eq!(projection["status"].as_str(), Some(status));
    assert_ne!(
        projection["status"].as_str(),
        Some("PASS"),
        "an incomplete attempt must invalidate a previous MT-045 PASS in {path:?}"
    );
}

fn assert_staged_failure_measurement_retention() -> serde_json::Value {
    let staged = RefCell::new(FailurePayload::empty());
    stage_failure_payload(
        &staged,
        serde_json::json!([measurement("negative_probe", 17.25, "ms")]),
        serde_json::json!({"probe": "post_measurement_assertion_failure"}),
    );
    let retained = snapshot_failure_payload(&staged);
    assert_eq!(
        retained.measurements[0]["name"].as_str(),
        Some("negative_probe")
    );
    assert_eq!(retained.measurements[0]["value"].as_f64(), Some(17.25));
    assert_ne!(
        retained.measurements.as_array().map(Vec::len),
        Some(0),
        "a post-measurement failure must not degrade to an empty measurement list"
    );
    serde_json::json!({
        "post_measurement_failure_retains_metric": true,
        "retained_metric": retained.measurements[0],
        "retained_evidence": retained.evidence,
        "status": "PASS",
    })
}

fn update_manifest_from_run_state(state: &serde_json::Value, scenario_id: &str) {
    let scenario = &state["scenarios"][scenario_id];
    let status = scenario["status"].as_str().unwrap_or("FAIL");
    let mut rows = read_manifest_rows_checked();
    let row = rows
        .iter_mut()
        .find(|row| row["scenario_id"].as_str() == Some(scenario_id))
        .unwrap_or_else(|| panic!("MT-045 manifest lacks scenario {scenario_id}"));
    let unit = row["unit"]
        .as_str()
        .unwrap_or_else(|| panic!("MT-045 manifest {scenario_id} lacks unit"))
        .to_owned();
    row["status"] = serde_json::json!(status);
    row["measured_value"] = scenario_measurement_value(scenario, &unit)
        .map_or(serde_json::Value::Null, |value| serde_json::json!(value));
    row["measured_profile"] = serde_json::json!(current_profile());
    row["gated"] = serde_json::json!(false);
    row["suite_run_id"] = state["run_id"].clone();
    stamp_budget_provenance(row);
    write_json_atomic(&manifest_path(), &serde_json::Value::Array(rows));
}

/// Adversarial review B1: derive the budget provenance for a manifest row from its declared env-override
/// var and contract-default budget, so the contract-authoritative manifest visibly records whether the
/// gate was WIDENED (`override_applied`) and the ceiling actually in force (`effective_budget`). Without
/// this, a run using a PERF_BUDGET_*_MS/_MB override could publish a PASS whose measured_value exceeds the
/// shown contract budget with no marker that the gate was widened — a dishonest "honest dashboard".
fn stamp_budget_provenance(row: &mut serde_json::Value) {
    let env_override = row["env_override"].as_str().unwrap_or("").to_owned();
    let override_value = if env_override.is_empty() {
        None
    } else {
        std::env::var(&env_override)
            .ok()
            .and_then(|value| value.trim().parse::<u128>().ok())
    };
    let contract_budget = row["budget_ms"]
        .as_u64()
        .or_else(|| row["budget_mb"].as_u64())
        .map(u128::from);
    row["override_applied"] = serde_json::json!(override_value.is_some());
    row["effective_budget"] = override_value
        .or(contract_budget)
        .map_or(serde_json::Value::Null, |value| serde_json::json!(value));
}

fn reset_manifest_for_run(run_id: &str) {
    let mut rows = read_manifest_rows_checked();
    for row in &mut rows {
        row["status"] = serde_json::json!("RUNNING");
        row["measured_value"] = serde_json::Value::Null;
        row["measured_profile"] = serde_json::json!(current_profile());
        row["gated"] = serde_json::json!(false);
        row["suite_run_id"] = serde_json::json!(run_id);
        stamp_budget_provenance(row);
    }
    write_json_atomic(&manifest_path(), &serde_json::Value::Array(rows));
}

fn scenario_measurement_value(scenario: &serde_json::Value, manifest_unit: &str) -> Option<f64> {
    let wanted_unit = normalized_measurement_unit(manifest_unit);
    scenario["proofs"]
        .as_object()
        .into_iter()
        .flat_map(|proofs| proofs.values())
        .flat_map(|proof| proof["measurements"].as_array().into_iter().flatten())
        .filter(|measurement| {
            measurement["unit"]
                .as_str()
                .is_some_and(|unit| normalized_measurement_unit(unit) == wanted_unit)
        })
        .filter_map(|measurement| measurement["value"].as_f64())
        .max_by(|left, right| left.total_cmp(right))
}

fn normalized_measurement_unit(unit: &str) -> String {
    match unit.to_ascii_lowercase().as_str() {
        "mib" => "mb".to_owned(),
        other => other.to_owned(),
    }
}

fn current_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

fn read_manifest_rows_checked() -> Vec<serde_json::Value> {
    let path = manifest_path();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read MT-045 manifest {path:?}: {error}"));
    let rows: Vec<serde_json::Value> = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("parse MT-045 manifest {path:?}: {error}"));
    assert_eq!(rows.len(), EXPECTED_SCENARIO_IDS.len());
    let actual: HashSet<String> = rows
        .iter()
        .map(|row| {
            row["scenario_id"]
                .as_str()
                .unwrap_or_else(|| panic!("MT-045 manifest row lacks scenario_id: {row}"))
                .to_owned()
        })
        .collect();
    assert_eq!(
        actual,
        expected_scenario_ids(),
        "MT-045 manifest must contain the exact 20 contract scenario ids"
    );
    let lk02 = rows
        .iter()
        .find(|row| row["scenario_id"].as_str() == Some("LK-02"))
        .expect("checked exact scenario set");
    let description = lk02["description"].as_str().unwrap_or_default();
    assert!(description.contains("1000"));
    assert!(
        !description.contains("NODE_CAP=200"),
        "LK-02 manifest description must not retain the obsolete 200-node limitation"
    );
    rows
}

fn assert_manifest_all_pass_current(state: &serde_json::Value) {
    let run_id = state["run_id"]
        .as_str()
        .expect("completed MT-045 run must carry run_id");
    assert_eq!(
        state
            .pointer("/file_lock_contract_proof/concurrent_contender_timed_out")
            .and_then(serde_json::Value::as_bool),
        Some(true),
        "completed MT-045 PASS requires the bounded lock-contention negative proof"
    );
    assert_eq!(
        state
            .pointer("/file_lock_contract_proof/release_reacquire")
            .and_then(serde_json::Value::as_str),
        Some("PASS"),
        "completed MT-045 PASS requires release/reacquire recovery proof"
    );
    assert_eq!(
        state
            .pointer("/failure_measurement_contract_proof/post_measurement_failure_retains_metric",)
            .and_then(serde_json::Value::as_bool),
        Some(true),
        "completed MT-045 PASS requires the post-measurement failure retention negative proof"
    );
    let rows = read_manifest_rows_checked();
    for row in rows {
        let scenario_id = row["scenario_id"].as_str().unwrap_or("UNKNOWN");
        assert_eq!(
            row["status"].as_str(),
            Some("PASS"),
            "completed MT-045 PASS requires manifest row {scenario_id} PASS"
        );
        assert_eq!(
            row["measured_profile"].as_str(),
            Some(current_profile()),
            "completed MT-045 PASS requires manifest row {scenario_id} in the current profile"
        );
        assert!(
            row["measured_value"].is_number(),
            "completed MT-045 PASS requires manifest row {scenario_id} measured_value"
        );
        assert_eq!(
            row["gated"].as_bool(),
            Some(false),
            "completed MT-045 PASS requires manifest row {scenario_id} ungated"
        );
        assert_eq!(
            row["suite_run_id"].as_str(),
            Some(run_id),
            "completed MT-045 PASS requires manifest row {scenario_id} from current run {run_id}"
        );
    }
}

fn write_json_immutable(path: &Path, value: &serde_json::Value) {
    assert!(
        !path.exists(),
        "immutable MT-045 run summary already exists: {path:?}"
    );
    let mut output = serde_json::to_string_pretty(value)
        .unwrap_or_else(|error| panic!("serialize immutable JSON artifact {path:?}: {error}"));
    output.push('\n');
    let temporary = path.with_extension(format!(
        "tmp.{}.{}.json",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    write_synced_new(&temporary, output.as_bytes())
        .unwrap_or_else(|error| panic!("write immutable JSON temp {temporary:?}: {error}"));
    std::fs::hard_link(&temporary, path)
        .unwrap_or_else(|error| panic!("publish immutable JSON artifact {path:?}: {error}"));
    // The hard-linked target is already the durable immutable publication. Temp-name cleanup is not
    // allowed to turn that successful publication into a false failure or trigger a duplicate retry.
    let _ = std::fs::remove_file(&temporary);
}

fn write_json_atomic(path: &Path, value: &serde_json::Value) {
    let mut output = serde_json::to_string_pretty(value)
        .unwrap_or_else(|error| panic!("serialize JSON artifact {path:?}: {error}"));
    output.push('\n');
    let temporary = path.with_extension(format!(
        "tmp.{}.{}.json",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    write_synced_new(&temporary, output.as_bytes())
        .unwrap_or_else(|error| panic!("write JSON artifact temp {temporary:?}: {error}"));
    atomic_replace_file(&temporary, path)
        .unwrap_or_else(|error| panic!("commit JSON artifact {path:?}: {error}"));
}

fn write_synced_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

#[cfg(windows)]
fn atomic_replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
    }
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    let source = windows_verbatim_wide_path(source)?;
    let target = windows_verbatim_wide_path(target)?;
    let succeeded = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if succeeded != 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(windows)]
fn windows_verbatim_wide_path(path: &Path) -> std::io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;

    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let file_name = absolute.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "atomic replacement path has no file name: {}",
                path.display()
            ),
        )
    })?;
    let parent = absolute.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("atomic replacement path has no parent: {}", path.display()),
        )
    })?;
    let normalized = parent.canonicalize()?.join(file_name);
    let encoded: Vec<u16> = normalized.as_os_str().encode_wide().collect();
    let mut verbatim =
        if encoded.starts_with(&[b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16]) {
            encoded
        } else if encoded.starts_with(&[b'\\' as u16, b'\\' as u16]) {
            let mut value = "\\\\?\\UNC\\".encode_utf16().collect::<Vec<_>>();
            value.extend_from_slice(&encoded[2..]);
            value
        } else {
            let mut value = "\\\\?\\".encode_utf16().collect::<Vec<_>>();
            value.extend_from_slice(&encoded);
            value
        };
    verbatim.push(0);
    Ok(verbatim)
}

#[cfg(not(windows))]
fn atomic_replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::rename(source, target)
}

/// The deterministic manifest path under the crate root, independent of the test's working directory.
pub fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("perf_proof")
        .join("perf_manifest.json")
}

pub fn external_artifact_root() -> PathBuf {
    if let Some(root) = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT") {
        return PathBuf::from(root).join("wp-kernel-012");
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("native crate must live below a worktree root")
        .join("Handshake_Artifacts")
        .join("wp-kernel-012")
}

// ── Memory measurement (RISK-5 / CTRL-5: median of 3) ────────────────────────────────────────────

/// Current process resident-set-size (RSS) in bytes via `sysinfo`. Cross-platform (Linux /proc, Windows
/// GetProcessMemoryInfo, macOS task_info). Fails closed when the process is not visible to sysinfo.
pub fn process_rss_bytes() -> u64 {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
    let pid = Pid::from_u32(std::process::id());
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing().with_memory(),
    );
    sys.process(pid)
        .map(|p| p.memory())
        .unwrap_or_else(|| panic!("RSS measurement unavailable for process {pid}"))
}

/// Measure the RSS delta of running `workload` as the WORST (max) of 3 runs. Each run: read RSS, run the
/// workload (keeping its result alive via the returned guard so the allocation is not freed before the
/// "after" reading), read RSS again, delta = after - before (saturating at 0). Returns the MAX delta in
/// MEGABYTES. The workload's output is dropped between runs so each run measures a fresh load.
///
/// Adversarial review B3: the prior median-of-3 could UNDER-report. On an allocator that retains freed
/// pages (typical Windows working-set behavior for large freed blocks), runs 2 and 3 reuse the first
/// run's pages and read a ~0 delta, so a median of [X, 0, 0] collapses to 0 and the memory budget becomes
/// non-binding. The MAX is the honest worst-case single-load cost — the cold first run reflects the real
/// allocation — and is strictly MORE conservative than the median. This tightens, never loosens, the gate.
pub fn measure_rss_delta_worst<T>(mut workload: impl FnMut() -> T) -> f64 {
    let mut deltas_mb: Vec<f64> = Vec::with_capacity(3);
    for _ in 0..3 {
        let before = process_rss_bytes();
        let held = workload();
        let after = process_rss_bytes();
        // Keep `held` alive across the "after" reading so its allocation is counted, then drop it.
        let delta_bytes = after.saturating_sub(before);
        drop(held);
        deltas_mb.push(delta_bytes as f64 / (1024.0 * 1024.0));
    }
    deltas_mb.into_iter().fold(0.0_f64, f64::max) // worst-case of three
}

// ── Artifact hygiene guard (CX-212E) ─────────────────────────────────────────────────────────────

/// Fail if a repo-local artifact directory exists (`test_output/` OR `tests/screenshots/`). The perf
/// suites write NO image artifacts; this guard catches a future regression that adds a repo-local one.
pub fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "CX-212E artifact hygiene: no repo-local '{local}' dir may exist — perf artifacts/records go \
             to the external Handshake_Artifacts/wp-kernel-012 root only \
             (found {})",
            p.display()
        );
    }
}

// ── Honest gates ─────────────────────────────────────────────────────────────────────────────────

/// `true` when `SKIP_PERF_TESTS=1` is set; prints the explicit skip line the contract mandates.
pub fn skip_all() -> bool {
    if std::env::var("SKIP_PERF_TESTS").as_deref() == Ok("1") {
        println!("PERF TESTS SKIPPED: SKIP_PERF_TESTS=1 is set");
        true
    } else {
        false
    }
}

// ── A deterministic Instant-elapsed millisecond helper ───────────────────────────────────────────

/// Elapsed wall-time of `op` in milliseconds (u128), measured with `std::time::Instant`. The caller is
/// responsible for placing this AFTER all fixture setup (RISK-2 / CTRL-2: never time the setup).
pub fn time_ms<T>(op: impl FnOnce() -> T) -> (T, u128) {
    let t0 = Instant::now();
    let out = op();
    (out, t0.elapsed().as_millis())
}

// ── Cross-process advisory lock ─────────────────────────────────────────────────────────────────

struct FileLock {
    file: File,
}

impl FileLock {
    fn acquire(path: &Path, budget: Duration) -> Option<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok()?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .ok()?;
        let started = Instant::now();
        loop {
            match try_lock_file(&file) {
                Ok(true) => return Some(Self { file }),
                Ok(false) if started.elapsed() < budget => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Ok(false) => return None,
                Err(error) => panic!("OS file-lock operation failed for {path:?}: {error}"),
            }
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = unlock_file(&self.file);
    }
}

/// Exercise the negative and recovery paths without adding a 21st catalog test. This runs once when
/// a fresh exact-suite run is created, before any measured workload begins.
fn assert_file_lock_contention_and_recovery(path: &Path) -> serde_json::Value {
    let first = FileLock::acquire(path, Duration::from_millis(250))
        .unwrap_or_else(|| panic!("MT-045 file-lock proof could not acquire first lock {path:?}"));
    let contender_path = path.to_path_buf();
    let (contender_acquired, contention_elapsed) = std::thread::spawn(move || {
        let started = Instant::now();
        let contender = FileLock::acquire(&contender_path, Duration::from_millis(50));
        (contender.is_some(), started.elapsed())
    })
    .join()
    .expect("MT-045 file-lock contention thread must not panic");
    assert!(
        !contender_acquired,
        "MT-045 held OS lock must reject a concurrent contender"
    );
    assert!(
        contention_elapsed < Duration::from_secs(1),
        "MT-045 lock contention must terminate within a bounded time (elapsed {contention_elapsed:?})"
    );

    drop(first);
    let recovered = FileLock::acquire(path, Duration::from_millis(250)).unwrap_or_else(|| {
        panic!("MT-045 released OS lock must be immediately recoverable {path:?}")
    });
    drop(recovered);

    serde_json::json!({
        "concurrent_contender_timed_out": true,
        "contention_elapsed_ms": contention_elapsed.as_millis(),
        "release_reacquire": "PASS",
    })
}

#[cfg(unix)]
fn try_lock_file(file: &File) -> std::io::Result<bool> {
    use std::os::fd::AsRawFd;
    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;
    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) } == 0 {
        Ok(true)
    } else {
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::WouldBlock {
            Ok(false)
        } else {
            Err(error)
        }
    }
}

#[cfg(unix)]
fn unlock_file(file: &File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    const LOCK_UN: i32 = 8;
    unsafe extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    if unsafe { flock(file.as_raw_fd(), LOCK_UN) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(windows)]
#[repr(C)]
struct Overlapped {
    internal: usize,
    internal_high: usize,
    offset_or_pointer: usize,
    event: *mut std::ffi::c_void,
}

#[cfg(windows)]
fn try_lock_file(file: &File) -> std::io::Result<bool> {
    use std::os::windows::io::AsRawHandle;
    const LOCKFILE_FAIL_IMMEDIATELY: u32 = 0x0000_0001;
    const LOCKFILE_EXCLUSIVE_LOCK: u32 = 0x0000_0002;
    const ERROR_LOCK_VIOLATION: i32 = 33;
    unsafe extern "system" {
        fn LockFileEx(
            file: *mut std::ffi::c_void,
            flags: u32,
            reserved: u32,
            bytes_low: u32,
            bytes_high: u32,
            overlapped: *mut Overlapped,
        ) -> i32;
    }
    let mut overlapped = Overlapped {
        internal: 0,
        internal_high: 0,
        offset_or_pointer: 0,
        event: std::ptr::null_mut(),
    };
    let locked = unsafe {
        LockFileEx(
            file.as_raw_handle(),
            LOCKFILE_FAIL_IMMEDIATELY | LOCKFILE_EXCLUSIVE_LOCK,
            0,
            1,
            0,
            &mut overlapped,
        )
    };
    if locked != 0 {
        Ok(true)
    } else {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(ERROR_LOCK_VIOLATION) {
            Ok(false)
        } else {
            Err(error)
        }
    }
}

#[cfg(windows)]
fn unlock_file(file: &File) -> std::io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    unsafe extern "system" {
        fn UnlockFileEx(
            file: *mut std::ffi::c_void,
            reserved: u32,
            bytes_low: u32,
            bytes_high: u32,
            overlapped: *mut Overlapped,
        ) -> i32;
    }
    let mut overlapped = Overlapped {
        internal: 0,
        internal_high: 0,
        offset_or_pointer: 0,
        event: std::ptr::null_mut(),
    };
    if unsafe { UnlockFileEx(file.as_raw_handle(), 0, 1, 0, &mut overlapped) } != 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
