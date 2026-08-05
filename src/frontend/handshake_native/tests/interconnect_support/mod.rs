//! WP-KERNEL-012 MT-046 — shared support for the four `test_interconnect_*.rs` interconnection-proof
//! suites (cluster E8, the melt-together capstone). Lives in a `tests/` SUBDIRECTORY so Cargo does NOT
//! compile it as a standalone test binary (only top-level `tests/*.rs` are test targets); each suite pulls
//! it in with `#[path = "interconnect_support/mod.rs"] mod interconnect_support;`.
//!
//! ## What it provides
//!
//! - [`mark_status`] / [`record_scenario`] — atomic external receipts under the dedicated MT-046 artifact
//!   root. The protected 18-entry manifest is referenced as a catalog and is never treated as the current
//!   runtime verdict or rewritten.
//! - [`require_reachable_backend`] / [`require_live_backend`] / [`LiveBackend`] — managed product-backend
//!   fixture. It attaches to a healthy root-managed process or starts an already-built executable, then
//!   creates/deletes an owned workspace through production HTTP. It never invokes Cargo, accepts no
//!   operator-preseeded ids, uses no SQLite/mock substitute, and only stops a process it started.
//! - [`assert_no_local_artifact_dir`] — CX-212E hygiene guard (checks `test_output/` AND
//!   `tests/screenshots/`); a tracked artifact under `src/` is a hygiene FAILURE.
//! - [`author_ids`] / [`author_node_value`] — AccessKit tree readers for the in-process substrate proofs.

#![allow(dead_code)] // each suite uses a subset of the helpers; the others are not dead in aggregate.

use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;
use sha2::{Digest, Sha256};

thread_local! {
    static CURRENT_BACKEND_BINDING: std::cell::RefCell<Option<serde_json::Value>> = const {
        std::cell::RefCell::new(None)
    };
    static CURRENT_BACKEND_RUNTIME_RECEIPT: std::cell::RefCell<Option<serde_json::Value>> = const {
        std::cell::RefCell::new(None)
    };
}

/// Wait for the managed-workspace FEMS refresh to terminalize, then remove only its incidental
/// notice before a cross-feature MT-046 screenshot. The product seam refuses to clear operator-owned
/// proposals, decisions, submissions, or an in-flight refresh.
pub fn settle_incidental_fems_for_capture(
    harness: &mut crate::screenshot_harness::ScreenshotHarness<
        '_,
        handshake_native::app::HandshakeApp,
    >,
    scenario_id: &str,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let cleared = harness
            .state_mut()
            .clear_incidental_fems_notice_for_integration_test();
        harness.run_steps(1);
        if cleared
            && harness
                .state_mut()
                .clear_incidental_fems_notice_for_integration_test()
        {
            harness.run_steps(1);
            if harness
                .state_mut()
                .clear_incidental_fems_notice_for_integration_test()
            {
                return;
            }
        }
        assert!(
            Instant::now() < deadline,
            "{scenario_id}: incidental FEMS notice did not terminalize before visual proof"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

// ── External scenario receipts; protected manifest is catalog-only ───────────────────────────────────

/// The protected contract catalog path. V2 proof runs never rewrite it.
pub fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test_interconnect_manifest.json")
}

/// Write a minimal external receipt. Scenario tests with ids/events/negative-path evidence should call
/// [`record_scenario`] directly; this compatibility wrapper keeps existing in-process proofs concise.
pub struct ScenarioAttempt {
    scenario_id: String,
    attempt_id: String,
    run_id: String,
    started_at: String,
    terminal: bool,
    canonical: bool,
}

impl ScenarioAttempt {
    pub fn begin(scenario_id: &str) -> Self {
        CURRENT_BACKEND_BINDING.with(|binding| {
            binding.borrow_mut().take();
        });
        CURRENT_BACKEND_RUNTIME_RECEIPT.with(|receipt| {
            receipt.borrow_mut().take();
        });
        assert!(
            expected_scenario_ids().contains(scenario_id),
            "MT-046 runtime receipt rejected unknown scenario id {scenario_id}"
        );
        let expected_proof = expected_proof_fn(scenario_id);
        let current_thread = std::thread::current();
        let test_thread = current_thread.name().unwrap_or("unnamed-test");
        assert_eq!(test_thread, expected_proof, "MT-046 scenario {scenario_id} must be emitted only by manifest proof_fn {expected_proof}");
        let attempt_id = uuid::Uuid::now_v7().to_string();
        let started_at = chrono::Utc::now().to_rfc3339();
        let canonical = std::env::var("HSK_MT046_CANONICAL").as_deref() == Ok("1");
        if !canonical {
            return Self {
                scenario_id: scenario_id.to_owned(),
                attempt_id,
                run_id: "NONCANONICAL-DIRECT-TEST".to_owned(),
                started_at,
                terminal: false,
                canonical: false,
            };
        }
        let run_id = begin_scenario_run(scenario_id, &attempt_id, &started_at);
        let attempt = Self {
            scenario_id: scenario_id.to_owned(),
            attempt_id,
            run_id,
            started_at,
            terminal: false,
            canonical: true,
        };
        // `begin_scenario_run` is the sole transient RUNNING projection. Immutable attempt history is
        // terminal-only so every file under `measurements/attempts` can be sealed by the supervisor.
        attempt
    }

    pub fn pass(mut self, evidence: serde_json::Value) {
        self.write("PASS", bind_backend_evidence(evidence), None);
        self.terminal = true;
    }

    pub fn skipped(mut self, reason: &str, evidence: serde_json::Value) {
        self.write("SKIPPED", bind_backend_evidence(evidence), Some(reason));
        self.terminal = true;
    }

    fn write(&self, status: &str, evidence: serde_json::Value, terminal_reason: Option<&str>) {
        if !self.canonical {
            return;
        }
        write_entry(
            &self.scenario_id,
            &self.attempt_id,
            &self.run_id,
            &self.started_at,
            status,
            evidence,
            terminal_reason,
        );
        update_scenario_run(
            &self.run_id,
            &self.scenario_id,
            &self.attempt_id,
            status,
            terminal_reason,
        );
    }
}

fn bind_backend_evidence(mut evidence: serde_json::Value) -> serde_json::Value {
    let binding = CURRENT_BACKEND_BINDING.with(|current| current.borrow().clone());
    let object = evidence
        .as_object_mut()
        .expect("MT-046 scenario evidence must be a JSON object");
    match binding {
        Some(binding) => {
            object.insert("backend_binding".to_owned(), binding);
        }
        None => {
            object.insert("backend_not_used".to_owned(), serde_json::json!(true));
        }
    }
    let runtime_receipt = CURRENT_BACKEND_RUNTIME_RECEIPT
        .with(|current| current.borrow().clone())
        .or_else(|| object.get("runtime_diagnostics").cloned());
    if let Some(receipt) = runtime_receipt {
        object.insert("backend_runtime_receipt".to_owned(), receipt);
    }
    evidence
}

impl Drop for ScenarioAttempt {
    fn drop(&mut self) {
        if self.canonical && !self.terminal {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.write(
                    "FAIL",
                    bind_backend_evidence(serde_json::json!({})),
                    Some("scenario_aborted_before_terminal_receipt"),
                );
            }));
        }
    }
}

pub fn mark_status(scenario_id: &str, status: &str) {
    let attempt = ScenarioAttempt::begin(scenario_id);
    if status == "PASS" {
        attempt.pass(serde_json::json!({"legacy_record_api": true}));
    } else {
        attempt.skipped(status, serde_json::json!({"legacy_record_api": true}));
    }
}

pub fn record_scenario(scenario_id: &str, status: &str, evidence: serde_json::Value) {
    let attempt = ScenarioAttempt::begin(scenario_id);
    if status == "PASS" {
        attempt.pass(evidence);
    } else {
        attempt.skipped(status, evidence);
    }
}

fn write_entry(
    scenario_id: &str,
    attempt_id: &str,
    run_id: &str,
    started_at: &str,
    status: &str,
    evidence: serde_json::Value,
    terminal_reason: Option<&str>,
) {
    let directory = external_artifact_dir("measurements");
    std::fs::create_dir_all(&directory)
        .unwrap_or_else(|error| panic!("interconnect receipt directory {directory:?}: {error}"));
    let lock_path = directory.join("measurements.lock");

    let _guard = match FileLock::acquire(&lock_path, Duration::from_secs(10)) {
        Some(g) => g,
        None => panic!("interconnect receipt lock {lock_path:?} unavailable for {scenario_id}"),
    };

    let provenance = required_supervisor_provenance();
    let recorded_at = chrono::Utc::now().to_rfc3339();
    let receipt = serde_json::json!({
        "schema_id": "hsk.wp_kernel_012.interconnection_proof@2",
        "work_packet_id": "WP-KERNEL-012",
        "micro_task_id": "MT-046",
        "scenario_id": scenario_id,
        "attempt_id": attempt_id,
        "run_id": run_id,
        "started_at": started_at,
        "status": status,
        "terminal_reason": terminal_reason,
        "recorded_at": &recorded_at,
        "completed_at": (status != "RUNNING").then_some(recorded_at),
        "process_id": std::process::id(),
        "test_thread": std::thread::current().name().unwrap_or("unnamed-test"),
        "provenance": provenance,
        "catalog_reference": {
            "path": "tests/test_interconnect_manifest.json",
            "scenario_id": scenario_id,
            "authority": "catalog_only_not_current_runtime_verdict",
            "superseded_by": "this_external_attempt_receipt"
        },
        "evidence": evidence,
        "runtime": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "debug_assertions": cfg!(debug_assertions),
        }
    });
    let path = directory.join(format!("{}.json", scenario_id.to_ascii_lowercase()));
    let temporary = directory.join(format!(
        ".{}.tmp.{}.json",
        scenario_id.to_ascii_lowercase(),
        std::process::id()
    ));
    let mut output = serde_json::to_string_pretty(&receipt)
        .unwrap_or_else(|error| panic!("serialize interconnect receipt {scenario_id}: {error}"));
    output.push('\n');
    std::fs::write(&temporary, output)
        .unwrap_or_else(|error| panic!("write interconnect receipt temp {temporary:?}: {error}"));
    let _ = std::fs::remove_file(&path);
    std::fs::rename(&temporary, &path)
        .unwrap_or_else(|error| panic!("commit interconnect receipt {path:?}: {error}"));

    let attempts = directory.join("attempts");
    std::fs::create_dir_all(&attempts)
        .unwrap_or_else(|error| panic!("interconnect attempt directory {attempts:?}: {error}"));
    let history_path = attempts.join(format!(
        "{}--{attempt_id}--{}.json",
        scenario_id.to_ascii_lowercase(),
        status.to_ascii_lowercase()
    ));
    let history = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("read interconnect attempt receipt {path:?}: {error}"));
    let mut immutable = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&history_path)
        .unwrap_or_else(|error| {
            panic!("create immutable attempt history {history_path:?}: {error}")
        });
    immutable.write_all(&history).unwrap_or_else(|error| {
        panic!("write immutable attempt history {history_path:?}: {error}")
    });
    immutable.sync_all().unwrap_or_else(|error| {
        panic!("flush immutable attempt history {history_path:?}: {error}")
    });
    let digest = format!("{:x}", Sha256::digest(&history));
    std::fs::write(
        history_path.with_extension("json.sha256"),
        format!(
            "{digest}  {}\n",
            history_path.file_name().unwrap().to_string_lossy()
        ),
    )
    .unwrap_or_else(|error| panic!("write immutable attempt digest {history_path:?}: {error}"));
}

fn required_env(name: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| panic!("canonical MT-046 proof requires non-empty {name}"))
}

fn required_supervisor_provenance() -> serde_json::Value {
    static TEST_EXECUTABLE: OnceLock<(String, String)> = OnceLock::new();
    let (executable, executable_sha256) = TEST_EXECUTABLE.get_or_init(|| {
        let path = std::env::current_exe()
            .unwrap_or_else(|error| panic!("resolve current MT-046 test executable: {error}"));
        let bytes = std::fs::read(&path).unwrap_or_else(|error| {
            panic!("read current MT-046 test executable {path:?}: {error}")
        });
        (
            path.display().to_string(),
            format!("{:x}", Sha256::digest(bytes)),
        )
    });
    serde_json::json!({
        "supervisor_run_id": required_env("HSK_MT046_RUN_ID"),
        "source_sha": required_env("HSK_MT046_SOURCE_SHA"),
        "candidate_source_id": required_env("HSK_MT046_CANDIDATE_SOURCE_ID"),
        "source_dirty_policy": required_env("HSK_MT046_SOURCE_DIRTY_POLICY"),
        "source_dirty_result_sha256": required_env("HSK_MT046_SOURCE_DIRTY_RESULT_SHA256"),
        "candidate_source_binding_path": required_env("HSK_MT046_CANDIDATE_BINDING_PATH"),
        "candidate_source_binding_sha256": required_env("HSK_MT046_CANDIDATE_BINDING_SHA256"),
        "test_binary": required_env("HSK_MT046_TEST_BINARY"),
        "cargo_profile": required_env("HSK_MT046_CARGO_PROFILE"),
        "cargo_locked": required_env("HSK_MT046_CARGO_LOCKED") == "true",
        "backend_path": required_env("HSK_MT046_BACKEND_PATH"),
        "backend_sha256": required_env("HSK_MT046_BACKEND_SHA256"),
        "postgres_identity": required_env("HSK_MT046_POSTGRES_IDENTITY"),
        "manifest_sha256": required_env("HSK_MT046_MANIFEST_SHA256"),
        "supervisor_pid": required_env("HSK_MT046_SUPERVISOR_PID"),
        "command_receipt_path": required_env("HSK_MT046_COMMAND_RECEIPT_PATH"),
        "stdout_path": required_env("HSK_MT046_STDOUT_PATH"),
        "stderr_path": required_env("HSK_MT046_STDERR_PATH"),
        "test_executable_path": executable,
        "test_executable_sha256": executable_sha256,
    })
}

fn expected_scenario_ids() -> HashSet<String> {
    let path = manifest_path();
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read MT-046 catalog {path:?}: {error}"));
    let rows: Vec<serde_json::Value> = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("parse MT-046 catalog {path:?}: {error}"));
    assert_eq!(
        rows.len(),
        18,
        "MT-046 catalog must contain exactly 18 rows"
    );
    let ids: HashSet<String> = rows
        .iter()
        .map(|row| {
            row.get("scenario_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| panic!("MT-046 catalog row lacks scenario_id: {row}"))
                .to_owned()
        })
        .collect();
    assert_eq!(ids.len(), 18, "MT-046 catalog scenario ids must be unique");
    ids
}

fn expected_proof_fn(scenario_id: &str) -> String {
    let path = manifest_path();
    let rows: Vec<serde_json::Value> = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read MT-046 catalog {path:?}: {error}")),
    )
    .unwrap_or_else(|error| panic!("parse MT-046 catalog {path:?}: {error}"));
    rows.iter()
        .find(|row| row["scenario_id"].as_str() == Some(scenario_id))
        .and_then(|row| row["proof_fn"].as_str())
        .unwrap_or_else(|| panic!("MT-046 catalog lacks proof_fn for {scenario_id}"))
        .to_owned()
}

fn begin_scenario_run(scenario_id: &str, attempt_id: &str, started_at: &str) -> String {
    let directory = external_artifact_dir("measurements");
    std::fs::create_dir_all(&directory)
        .unwrap_or_else(|error| panic!("interconnect run directory {directory:?}: {error}"));
    let lock_path = directory.join("run-state.lock");
    let _guard = FileLock::acquire(&lock_path, Duration::from_secs(10))
        .unwrap_or_else(|| panic!("interconnect run-state lock unavailable for {scenario_id}"));
    let state_path = directory.join("current-run.json");
    let run_id = required_env("HSK_MT046_RUN_ID");
    let provenance = required_supervisor_provenance();
    let text = std::fs::read_to_string(&state_path).unwrap_or_else(|error| {
        panic!(
            "canonical supervisor must initialize MT-046 current-run before {scenario_id}: {error}"
        )
    });
    let mut state: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("parse supervisor-owned MT-046 current-run: {error}"));
    assert_eq!(
        state["run_id"].as_str(),
        Some(run_id.as_str()),
        "MT-046 scenario rejected stale/mixed supervisor run id"
    );
    assert_eq!(
        state["status"].as_str(),
        Some("RUNNING"),
        "MT-046 scenario requires a supervisor-owned RUNNING run"
    );
    assert_eq!(
        state["provenance"]["source_sha"], provenance["source_sha"],
        "MT-046 source SHA changed after supervisor preflight"
    );
    assert_eq!(
        state["provenance"]["candidate_source_id"], provenance["candidate_source_id"],
        "MT-046 candidate source identity changed after supervisor preflight"
    );
    assert_eq!(
        state["provenance"]["source_dirty_policy"], provenance["source_dirty_policy"],
        "MT-046 dirty-source policy changed after supervisor preflight"
    );
    assert_eq!(
        state["provenance"]["source_dirty_result_sha256"], provenance["source_dirty_result_sha256"],
        "MT-046 dirty-source result changed after supervisor preflight"
    );
    assert_eq!(
        state["provenance"]["candidate_source_binding"],
        provenance["candidate_source_binding_path"],
        "MT-046 candidate binding path changed after supervisor preflight"
    );
    assert_eq!(
        state["provenance"]["candidate_source_binding_sha256"],
        provenance["candidate_source_binding_sha256"],
        "MT-046 candidate binding digest changed after supervisor preflight"
    );
    assert_eq!(
        state["provenance"]["cargo_profile"], provenance["cargo_profile"],
        "MT-046 Cargo profile changed after supervisor preflight"
    );
    assert_eq!(
        state["provenance"]["cargo_locked"], provenance["cargo_locked"],
        "MT-046 Cargo locked state changed after supervisor preflight"
    );
    assert_eq!(
        state["provenance"]["backend_path"], provenance["backend_path"],
        "MT-046 backend path changed after supervisor preflight"
    );
    assert_eq!(
        state["provenance"]["backend_sha256"], provenance["backend_sha256"],
        "MT-046 backend hash changed after supervisor preflight"
    );
    assert_eq!(
        state["provenance"]["postgres"]["dsn"], provenance["postgres_identity"],
        "MT-046 PostgreSQL identity changed after supervisor preflight"
    );
    assert_eq!(
        state["provenance"]["manifest_sha256"], provenance["manifest_sha256"],
        "MT-046 manifest hash changed after supervisor preflight"
    );
    assert_eq!(
        state["provenance"]["supervisor_pid"]
            .as_u64()
            .map(|value| value.to_string()),
        provenance["supervisor_pid"].as_str().map(str::to_owned),
        "MT-046 supervisor PID changed after preflight"
    );
    assert!(
        provenance["test_executable_path"]
            .as_str()
            .is_some_and(|path| path.contains(provenance["test_binary"].as_str().unwrap())),
        "MT-046 test binary env does not match current executable"
    );
    assert!(
        state["scenarios"].get(scenario_id).is_none(),
        "MT-046 run {run_id} rejected duplicate scenario {scenario_id}"
    );
    state["updated_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
    state["scenarios"][scenario_id] = serde_json::json!({
        "attempt_id": attempt_id,
        "started_at": started_at,
        "status": "RUNNING",
    });
    write_json_atomic(&state_path, &state);
    run_id
}

fn update_scenario_run(
    run_id: &str,
    scenario_id: &str,
    attempt_id: &str,
    status: &str,
    terminal_reason: Option<&str>,
) {
    let directory = external_artifact_dir("measurements");
    let lock_path = directory.join("run-state.lock");
    let _guard = FileLock::acquire(&lock_path, Duration::from_secs(10))
        .unwrap_or_else(|| panic!("interconnect run-state lock unavailable for {scenario_id}"));
    let state_path = directory.join("current-run.json");
    let text = std::fs::read_to_string(&state_path)
        .unwrap_or_else(|error| panic!("read interconnect run state {state_path:?}: {error}"));
    let mut state: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("parse interconnect run state {state_path:?}: {error}"));
    assert_eq!(
        state["run_id"].as_str(),
        Some(run_id),
        "scenario {scenario_id} attempted to finalize a stale MT-046 run"
    );
    assert_eq!(
        state["scenarios"][scenario_id]["attempt_id"].as_str(),
        Some(attempt_id),
        "scenario {scenario_id} attempted to finalize a superseded attempt"
    );
    state["scenarios"][scenario_id]["status"] = serde_json::json!(status);
    state["scenarios"][scenario_id]["terminal_reason"] = serde_json::json!(terminal_reason);
    state["scenarios"][scenario_id]["recorded_at"] =
        serde_json::json!(chrono::Utc::now().to_rfc3339());
    state["scenarios"][scenario_id]["attempt_receipt_path"] = serde_json::json!(format!(
        "attempts/{}--{attempt_id}--{}.json",
        scenario_id.to_ascii_lowercase(),
        status.to_ascii_lowercase()
    ));
    state["updated_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());

    // Only the source-bound supervisor may seal/terminalize the overall run because process exit,
    // stdout/stderr hashes, timeout, and descendant-leak evidence do not exist inside this process yet.
    write_json_atomic(&state_path, &state);
}

fn write_json_atomic(path: &Path, value: &serde_json::Value) {
    let temporary = path.with_extension(format!("tmp.{}.json", std::process::id()));
    let mut output = serde_json::to_string_pretty(value)
        .unwrap_or_else(|error| panic!("serialize JSON artifact {path:?}: {error}"));
    output.push('\n');
    std::fs::write(&temporary, output)
        .unwrap_or_else(|error| panic!("write JSON artifact temp {temporary:?}: {error}"));
    let _ = std::fs::remove_file(path);
    std::fs::rename(&temporary, path)
        .unwrap_or_else(|error| panic!("commit JSON artifact {path:?}: {error}"));
}

/// A minimal cross-process advisory lock: an O_EXCL `.lock` file removed on drop, with bounded spin.
struct FileLock {
    path: PathBuf,
}

impl FileLock {
    fn acquire(path: &Path, budget: Duration) -> Option<Self> {
        let start = Instant::now();
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                Ok(_) => {
                    return Some(FileLock {
                        path: path.to_path_buf(),
                    });
                }
                Err(_) if start.elapsed() < budget => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(_) => {
                    if let Ok(meta) = std::fs::metadata(path) {
                        if let Ok(modified) = meta.modified() {
                            if modified.elapsed().map(|e| e > budget).unwrap_or(false) {
                                let _ = std::fs::remove_file(path);
                                continue;
                            }
                        }
                    }
                    return None;
                }
            }
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

// ── Artifact hygiene (CX-212E / CX-212F): artifacts go to the EXTERNAL root ONLY ──────────────────────

/// Dedicated external V2 receipt root, resolved without a hardcoded drive/user path.
pub fn external_artifact_dir(subdir: &str) -> PathBuf {
    let root = std::env::var_os("HANDSHAKE_TEST_ARTIFACTS_ROOT")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
                .map(PathBuf::from)
                .map(|root| root.join("handshake-test"))
        })
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(4)
                .expect("native crate must live below a worktree root")
                .join("Handshake_Artifacts")
                .join("handshake-test")
        });
    root.join("wp-kernel-012").join("mt-046").join(subdir)
}

pub fn write_immutable_external_json(path: &Path, value: &serde_json::Value) {
    let parent = path
        .parent()
        .expect("MT-046 immutable evidence path has a parent");
    std::fs::create_dir_all(parent)
        .unwrap_or_else(|error| panic!("create MT-046 evidence directory {parent:?}: {error}"));
    let mut bytes = serde_json::to_vec_pretty(value).expect("serialize MT-046 immutable evidence");
    bytes.push(b'\n');
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap_or_else(|error| panic!("create immutable MT-046 evidence {path:?}: {error}"));
    file.write_all(&bytes)
        .unwrap_or_else(|error| panic!("write immutable MT-046 evidence {path:?}: {error}"));
    file.sync_all()
        .unwrap_or_else(|error| panic!("flush immutable MT-046 evidence {path:?}: {error}"));
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let digest_path = path.with_extension("json.sha256");
    let mut digest_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&digest_path)
        .unwrap_or_else(|error| panic!("create MT-046 evidence digest {digest_path:?}: {error}"));
    digest_file
        .write_all(
            format!(
                "{digest}  {}\n",
                path.file_name().unwrap().to_string_lossy()
            )
            .as_bytes(),
        )
        .unwrap_or_else(|error| panic!("write MT-046 evidence digest {digest_path:?}: {error}"));
    digest_file
        .sync_all()
        .unwrap_or_else(|error| panic!("flush MT-046 evidence digest {digest_path:?}: {error}"));
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` AND `tests/screenshots/`; a tracked artifact under `src/` is a hygiene FAILURE — this
/// guard fails the run if one appears. Per CX-212E the rule OVERRIDES any repo-local path a contract names.
pub fn assert_no_local_artifact_dir() {
    for local in [Path::new("test_output"), Path::new("tests/screenshots")] {
        assert!(
            !local.exists(),
            "CX-212E: no repo-local artifact dir may exist ({}) — artifacts go to the external \
             Handshake_Artifacts/handshake-test/wp-kernel-012 root only",
            local.display()
        );
    }
}

// ── AccessKit tree readers (the in-process substrate proofs) ──────────────────────────────────────────

/// Every author_id present in the live AccessKit tree.
pub fn author_ids<S>(harness: &Harness<'_, S>) -> HashSet<String> {
    let mut ids = HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// The `value` of the AccessKit node carrying `author_id`, or `None` when absent.
pub fn author_node_value<S>(harness: &Harness<'_, S>, author_id: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return ak.value().map(|v| v.to_owned());
        }
    }
    None
}

// ── Real managed backend fixture (shared with parity/performance proofs) ────────────────────────────

#[path = "../pg_proof_support/mod.rs"]
mod pg_proof_support;

#[allow(unused_imports)]
// Each integration test crate consumes a different fixture entrypoint.
pub use pg_proof_support::DEFAULT_BASE;

pub struct LiveBackend {
    inner: pg_proof_support::LiveBackend,
    cleanup_complete: bool,
}

impl std::ops::Deref for LiveBackend {
    type Target = pg_proof_support::LiveBackend;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for LiveBackend {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl LiveBackend {
    pub fn assert_cleanup_and_publish_runtime_diagnostics(
        &mut self,
        scenario_id: &str,
    ) -> Result<serde_json::Value, String> {
        let receipt = self
            .inner
            .assert_cleanup_and_publish_runtime_diagnostics(scenario_id)?;
        CURRENT_BACKEND_RUNTIME_RECEIPT.with(|current| {
            *current.borrow_mut() = Some(receipt.clone());
        });
        self.cleanup_complete = true;
        Ok(receipt)
    }

    pub fn assert_cleanup(&mut self) {
        if std::env::var("HSK_MT046_CANONICAL").as_deref() == Ok("1") {
            let current_thread = std::thread::current();
            let scenario = current_thread.name().unwrap_or("unnamed-test-thread");
            self.assert_cleanup_and_publish_runtime_diagnostics(scenario)
                .unwrap_or_else(|error| {
                    panic!("publish MT-046 owned backend runtime diagnostics: {error}")
                });
        } else {
            self.inner.assert_cleanup();
            self.cleanup_complete = true;
        }
    }
}

impl Drop for LiveBackend {
    fn drop(&mut self) {
        if self.cleanup_complete || std::env::var("HSK_MT046_CANONICAL").as_deref() != Ok("1") {
            return;
        }
        let current_thread = std::thread::current();
        let scenario = current_thread.name().unwrap_or("unnamed-test-thread");
        let publication = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.inner
                .assert_cleanup_and_publish_runtime_diagnostics(scenario)
        }));
        if let Ok(Ok(receipt)) = publication {
            CURRENT_BACKEND_RUNTIME_RECEIPT.with(|current| {
                *current.borrow_mut() = Some(receipt);
            });
            self.cleanup_complete = true;
        }
    }
}

/// Start a fixture-owned current-source backend and immediately publish an immutable, run-bound
/// ownership receipt while the child is live. The supervisor independently binds this receipt to the
/// exact test process, backend binary, PostgreSQL identity, and retained runtime diagnostics.
pub fn require_live_backend() -> LiveBackend {
    let backend = pg_proof_support::require_live_backend();
    publish_owned_backend_binding_receipt(&backend);
    LiveBackend {
        inner: backend,
        cleanup_complete: false,
    }
}

pub fn require_reachable_backend() -> LiveBackend {
    let backend = pg_proof_support::require_reachable_backend();
    publish_owned_backend_binding_receipt(&backend);
    LiveBackend {
        inner: backend,
        cleanup_complete: false,
    }
}

fn publish_owned_backend_binding_receipt(backend: &pg_proof_support::LiveBackend) {
    if std::env::var("HSK_MT046_CANONICAL").as_deref() != Ok("1") {
        return;
    }
    let run_id = required_env("HSK_MT046_RUN_ID");
    let correlation_id = required_env("HANDSHAKE_PROOF_PROCESS_CORRELATION_ID");
    let provenance = required_supervisor_provenance();
    let backend_binding = backend.owned_backend_binding_receipt();
    CURRENT_BACKEND_BINDING.with(|current| {
        *current.borrow_mut() = Some(backend_binding.clone());
    });
    let backend_pid = backend_binding["backend_pid"]
        .as_u64()
        .expect("owned backend binding requires a child PID");
    let current_thread = std::thread::current();
    let test_thread = current_thread.name().unwrap_or("unnamed-test-thread");
    let directory = external_artifact_dir("backend-bindings").join(&run_id);
    std::fs::create_dir_all(&directory)
        .unwrap_or_else(|error| panic!("create MT-046 backend binding directory: {error}"));
    let filename = format!(
        "{}--{}--{}.json",
        test_thread.replace(|character: char| !character.is_ascii_alphanumeric(), "-"),
        std::process::id(),
        backend_pid
    );
    let path = directory.join(filename);
    let receipt = serde_json::json!({
        "schema_id": "hsk.wp_kernel_012.mt046_owned_backend_binding@1",
        "run_id": run_id,
        "source_sha": provenance["source_sha"],
        "candidate_source_id": provenance["candidate_source_id"],
        "test_binary": provenance["test_binary"],
        "test_thread": test_thread,
        "test_process_id": std::process::id(),
        "test_executable_path": provenance["test_executable_path"],
        "test_executable_sha256": provenance["test_executable_sha256"],
        "process_correlation_id": correlation_id,
        "process_scenario_id": std::env::var("HANDSHAKE_PROOF_PROCESS_SCENARIO_ID").ok(),
        "backend_parent_process_id": std::process::id(),
        "backend": backend_binding,
        "recorded_at": chrono::Utc::now().to_rfc3339(),
    });
    let mut bytes = serde_json::to_vec_pretty(&receipt)
        .expect("serialize MT-046 owned backend binding receipt");
    bytes.push(b'\n');
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap_or_else(|error| {
            panic!("create immutable MT-046 backend binding {path:?}: {error}")
        });
    file.write_all(&bytes)
        .unwrap_or_else(|error| panic!("write MT-046 backend binding {path:?}: {error}"));
    file.sync_all()
        .unwrap_or_else(|error| panic!("flush MT-046 backend binding {path:?}: {error}"));
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let digest_path = path.with_extension("json.sha256");
    let mut digest_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&digest_path)
        .unwrap_or_else(|error| panic!("create MT-046 backend binding digest: {error}"));
    digest_file
        .write_all(
            format!(
                "{digest}  {}\n",
                path.file_name().unwrap().to_string_lossy()
            )
            .as_bytes(),
        )
        .unwrap_or_else(|error| panic!("write MT-046 backend binding digest: {error}"));
    digest_file
        .sync_all()
        .unwrap_or_else(|error| panic!("flush MT-046 backend binding digest: {error}"));
}

/// Evidence returned by the same production SaveManager + RichDocSaveBackend path mounted by the
/// native rich editor. Interconnect proofs use this instead of issuing their own direct save PUT.
pub struct ProductionSaveProof {
    pub doc_version: u64,
    pub backlinks_persisted: usize,
    pub save_receipt_event_id: String,
}

pub fn save_rich_document_via_production_manager(
    backend: &LiveBackend,
    document_id: &str,
    expected_version: u64,
    content_json: serde_json::Value,
) -> ProductionSaveProof {
    use handshake_native::rich_editor::save::save_manager::{SaveManager, SaveOutcome};

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("interconnect SaveManager runtime");
    let mut save = SaveManager::new(
        Arc::new(
            handshake_native::backend_client::RichDocSaveBackend::new_with_actor(
                backend.base.clone(),
                format!("mt046-interconnect-{}", std::process::id()),
            ),
        ),
        Some(runtime.handle().clone()),
        document_id,
        expected_version,
    );
    save.set_workspace_id(backend.workspace_id.clone());
    save.mark_dirty();
    save.request_save(content_json);

    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        // Drive the runtime briefly so the production async transport can complete, then drain exactly
        // as the mounted egui frame loop does. The bounded loop remains quiet and deterministic.
        runtime.block_on(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
        });
        if let Some(outcome) = save.drain() {
            return match outcome {
                SaveOutcome::Saved {
                    doc_version,
                    backlinks_persisted,
                    save_receipt_event_id,
                    ..
                } => ProductionSaveProof {
                    doc_version,
                    backlinks_persisted,
                    save_receipt_event_id: save_receipt_event_id
                        .expect("production rich save returns an EventLedger receipt"),
                },
                SaveOutcome::Conflict => {
                    panic!("production interconnect save unexpectedly version-conflicted")
                }
                SaveOutcome::Failed(error) => {
                    panic!("production interconnect save failed: {error}")
                }
            };
        }
        assert!(
            Instant::now() < deadline,
            "production SaveManager did not drain within 15 seconds"
        );
    }
}

/// Read the immutable PostgreSQL EventLedger authority row named by a production save receipt.
/// `/events` is the Flight Recorder UUID projection and therefore cannot accept a typed `KE-*` id.
/// The canonical payload remains at the top level; `_event_*`/`_aggregate_*` fields expose immutable
/// row identity so callers can prove a receipt belongs to the exact document, not merely a workspace.
pub fn event_ledger_payload(event_id: &str) -> serde_json::Value {
    assert!(
        event_id.starts_with("KE-")
            && uuid::Uuid::parse_str(event_id.trim_start_matches("KE-")).is_ok(),
        "production save receipt must carry a typed KE UUID"
    );
    let database_url = [
        "HANDSHAKE_TEST_PG_DSN",
        "HSK_PROOF_DATABASE_URL",
        "POSTGRES_TEST_URL",
        "DATABASE_URL",
    ]
    .into_iter()
    .find_map(|name| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    })
    .expect("EventLedger proof requires a managed PostgreSQL DSN");
    let psql = std::env::var_os("HSK_PSQL_BIN").unwrap_or_else(|| "psql".into());
    let mut command = Command::new(psql);
    command
        .arg("--no-psqlrc")
        .arg("--set")
        .arg("ON_ERROR_STOP=1")
        .arg("--tuples-only")
        .arg("--no-align")
        .arg("--dbname")
        .arg(database_url)
        .arg("--command")
        .arg(format!(
            "SELECT (payload || jsonb_build_object(\
                '_event_id', event_id, \
                '_event_type', event_type, \
                '_aggregate_type', aggregate_type, \
                '_aggregate_id', aggregate_id\
            ))::text FROM kernel_event_ledger WHERE event_id = '{event_id}'"
        ))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PGCONNECT_TIMEOUT", "5");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .expect("start bounded psql EventLedger receipt query");
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("EventLedger receipt query exceeded 10 seconds and was reaped");
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("poll EventLedger receipt query: {error}");
            }
        }
    };
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .expect("capture EventLedger query stdout")
        .read_to_string(&mut stdout)
        .expect("read EventLedger query stdout");
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("capture EventLedger query stderr")
        .read_to_string(&mut stderr)
        .expect("read EventLedger query stderr");
    assert!(
        status.success(),
        "EventLedger receipt query failed with {status}: {stderr}"
    );
    serde_json::from_str(stdout.trim()).unwrap_or_else(|error| {
        panic!("EventLedger receipt {event_id} payload missing or invalid ({error}): {stdout}")
    })
}

/// Durable PostgreSQL residue relevant to the IC-13 typed no-model branch. The workspace-scoped
/// suggestion and joined EventLedger counts must remain zero; the fixture-owned session's recorded-event
/// count must not change across the request, catching an orphan append without observing another WP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoomAiResidueCounts {
    pub suggestion_rows: i64,
    pub recorded_event_rows: i64,
    pub fixture_session_recorded_events: i64,
}

pub fn loom_ai_residue_counts(workspace_id: &str) -> LoomAiResidueCounts {
    assert!(
        !workspace_id.is_empty()
            && workspace_id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_')),
        "IC-13 workspace id is not safe for a psql variable"
    );
    let query = r#"
SELECT COUNT(*) FROM loom_ai_suggestions WHERE workspace_id = :'workspace_id';
SELECT COUNT(*)
FROM kernel_event_ledger AS event
JOIN loom_ai_suggestions AS suggestion ON suggestion.recorded_event_id = event.event_id
WHERE suggestion.workspace_id = :'workspace_id'
  AND event.event_type = 'AI_EDIT_PROPOSAL_RECORDED'
  AND event.source_component = 'loom_ai_job';
SELECT COUNT(*) FROM kernel_event_ledger
WHERE event_type = 'AI_EDIT_PROPOSAL_RECORDED'
  AND source_component = 'loom_ai_job'
  AND session_run_id = 'wp-kernel-012-native-proof-session';
"#;
    let output = run_bounded_psql(query, Some(("workspace_id", workspace_id)));
    let counts: Vec<i64> = output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            (!trimmed.is_empty()).then(|| {
                trimmed.parse::<i64>().unwrap_or_else(|error| {
                    panic!("parse IC-13 PostgreSQL count {trimmed:?}: {error}")
                })
            })
        })
        .collect();
    assert_eq!(
        counts.len(),
        3,
        "IC-13 residue query must return exactly three scalar counts: {output:?}"
    );
    LoomAiResidueCounts {
        suggestion_rows: counts[0],
        recorded_event_rows: counts[1],
        fixture_session_recorded_events: counts[2],
    }
}

pub(crate) fn run_bounded_psql(sql: &str, variable: Option<(&str, &str)>) -> String {
    let database_url = [
        "HANDSHAKE_TEST_PG_DSN",
        "HSK_PROOF_DATABASE_URL",
        "POSTGRES_TEST_URL",
        "DATABASE_URL",
    ]
    .into_iter()
    .find_map(|name| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    })
    .expect("managed PostgreSQL proof requires a DSN");
    let psql = std::env::var_os("HSK_PSQL_BIN").unwrap_or_else(|| "psql".into());
    let mut command = Command::new(psql);
    command
        .arg("--no-psqlrc")
        .arg("--set")
        .arg("ON_ERROR_STOP=1")
        .arg("--tuples-only")
        .arg("--no-align")
        .arg("--dbname")
        .arg(database_url);
    if let Some((name, value)) = variable {
        command.arg("--set").arg(format!("{name}={value}"));
    }
    command
        // psql does not expand `:variable` references in text supplied through
        // `--command`. Feed the query through stdin so the `--set` value above
        // is expanded by psql while the query remains a real PostgreSQL proof.
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PGCONNECT_TIMEOUT", "5");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .expect("start bounded managed PostgreSQL query");
    child
        .stdin
        .take()
        .expect("open managed PostgreSQL query stdin")
        .write_all(sql.as_bytes())
        .expect("write managed PostgreSQL query");
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("managed PostgreSQL query exceeded 10 seconds and was reaped");
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("poll managed PostgreSQL query: {error}");
            }
        }
    };
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .expect("capture managed PostgreSQL query stdout")
        .read_to_string(&mut stdout)
        .expect("read managed PostgreSQL query stdout");
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .expect("capture managed PostgreSQL query stderr")
        .read_to_string(&mut stderr)
        .expect("read managed PostgreSQL query stderr");
    assert!(
        status.success(),
        "managed PostgreSQL query failed with {status}: {stderr}"
    );
    stdout
}
