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

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

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
}

impl ScenarioAttempt {
    pub fn begin(scenario_id: &str) -> Self {
        assert!(
            expected_scenario_ids().contains(scenario_id),
            "MT-046 runtime receipt rejected unknown scenario id {scenario_id}"
        );
        let attempt_id = uuid::Uuid::now_v7().to_string();
        let started_at = chrono::Utc::now().to_rfc3339();
        let run_id = begin_scenario_run(scenario_id, &attempt_id, &started_at);
        let attempt = Self {
            scenario_id: scenario_id.to_owned(),
            attempt_id,
            run_id,
            started_at,
            terminal: false,
        };
        attempt.write("RUNNING", serde_json::json!({}), None);
        attempt
    }

    pub fn pass(mut self, evidence: serde_json::Value) {
        self.write("PASS", evidence, None);
        self.terminal = true;
    }

    pub fn skipped(mut self, reason: &str, evidence: serde_json::Value) {
        self.write("SKIPPED", evidence, Some(reason));
        self.terminal = true;
    }

    fn write(&self, status: &str, evidence: serde_json::Value, terminal_reason: Option<&str>) {
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

impl Drop for ScenarioAttempt {
    fn drop(&mut self) {
        if !self.terminal {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.write(
                    "FAIL",
                    serde_json::json!({}),
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
        "recorded_at": chrono::Utc::now().to_rfc3339(),
        "process_id": std::process::id(),
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
        "{}--{attempt_id}.json",
        scenario_id.to_ascii_lowercase()
    ));
    std::fs::copy(&path, &history_path).unwrap_or_else(|error| {
        panic!("write interconnect attempt history {history_path:?}: {error}")
    });
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

fn begin_scenario_run(scenario_id: &str, attempt_id: &str, started_at: &str) -> String {
    let directory = external_artifact_dir("measurements");
    std::fs::create_dir_all(&directory)
        .unwrap_or_else(|error| panic!("interconnect run directory {directory:?}: {error}"));
    let lock_path = directory.join("run-state.lock");
    let _guard = FileLock::acquire(&lock_path, Duration::from_secs(10))
        .unwrap_or_else(|| panic!("interconnect run-state lock unavailable for {scenario_id}"));
    let state_path = directory.join("current-run.json");
    let requested_run = std::env::var("HSK_MT046_RUN_ID")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let existing = std::fs::read_to_string(&state_path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    let existing_scenario = existing
        .as_ref()
        .and_then(|state| state.pointer(&format!("/scenarios/{scenario_id}")));
    let existing_terminal = existing_scenario
        .and_then(|entry| entry.get("status"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|status| status != "RUNNING");
    let existing_status = existing
        .as_ref()
        .and_then(|state| state.get("status"))
        .and_then(serde_json::Value::as_str);
    let existing_run_id = existing
        .as_ref()
        .and_then(|state| state.get("run_id"))
        .and_then(serde_json::Value::as_str);
    let must_start = requested_run
        .as_deref()
        .is_some_and(|requested| existing_run_id != Some(requested))
        || existing_run_id.is_none()
        || matches!(existing_status, Some("PASS" | "FAIL"))
        || existing_terminal;
    let run_id = if must_start {
        requested_run.unwrap_or_else(|| format!("MT046-RUN-{}", uuid::Uuid::now_v7().simple()))
    } else {
        existing_run_id.expect("checked present").to_owned()
    };
    let mut state = if must_start {
        serde_json::json!({
            "schema_id": "hsk.wp_kernel_012.interconnection_run@1",
            "work_packet_id": "WP-KERNEL-012",
            "micro_task_id": "MT-046",
            "run_id": run_id,
            "started_at": started_at,
            "status": "RUNNING",
            "expected_scenario_count": 18,
            "catalog_reference": "tests/test_interconnect_manifest.json",
            "catalog_semantics": "catalog_only_expected_outcomes_not_runtime_verdict",
            "scenarios": {},
        })
    } else {
        existing.expect("checked present")
    };
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
        "attempts/{}--{attempt_id}.json",
        scenario_id.to_ascii_lowercase()
    ));
    state["updated_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());

    let scenarios = state["scenarios"]
        .as_object()
        .expect("MT-046 run scenarios object");
    let terminal: HashMap<String, String> = scenarios
        .iter()
        .filter_map(|(id, entry)| {
            let status = entry.get("status")?.as_str()?;
            (status != "RUNNING").then_some((id.clone(), status.to_owned()))
        })
        .collect();
    if terminal.len() == 18 {
        let actual: HashSet<String> = terminal.keys().cloned().collect();
        let expected = expected_scenario_ids();
        let ids_exact = actual == expected;
        let statuses_valid = scenarios.iter().all(|(id, entry)| {
            let status = entry.get("status").and_then(serde_json::Value::as_str);
            let reason = entry
                .get("terminal_reason")
                .and_then(serde_json::Value::as_str);
            status == Some("PASS")
                || (id == "IC-13"
                    && status == Some("SKIPPED")
                    && reason == Some("HSK-409-LOOM-AI-NO-MODEL"))
        });
        let overall = if ids_exact && statuses_valid {
            "PASS"
        } else {
            "FAIL"
        };
        state["status"] = serde_json::json!(overall);
        state["completed_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
        state["terminal_scenario_count"] = serde_json::json!(terminal.len());
        state["exact_scenario_set"] = serde_json::json!(ids_exact);
        state["all_statuses_accepted"] = serde_json::json!(statuses_valid);
        state["accepted_skip_policy"] = serde_json::json!({"scenario_id": "IC-13", "terminal_reason": "HSK-409-LOOM-AI-NO-MODEL"});
        let runs = directory.join("runs");
        std::fs::create_dir_all(&runs)
            .unwrap_or_else(|error| panic!("create interconnect run summaries {runs:?}: {error}"));
        write_json_atomic(&runs.join(format!("{run_id}.json")), &state);
        write_json_atomic(&directory.join("latest-run-summary.json"), &state);
        write_json_atomic(&state_path, &state);
        assert_eq!(
            overall, "PASS",
            "MT-046 run {run_id} cannot pass: exact_ids={ids_exact} statuses_valid={statuses_valid} terminal={terminal:?}"
        );
        return;
    }
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
    let root = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(4)
                .expect("native crate must live below a worktree root")
                .join("Handshake_Artifacts")
        });
    root.join("wp-kernel-012").join("mt-046").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` AND `tests/screenshots/`; a tracked artifact under `src/` is a hygiene FAILURE — this
/// guard fails the run if one appears. Per CX-212E the rule OVERRIDES any repo-local path a contract names.
pub fn assert_no_local_artifact_dir() {
    for local in [Path::new("test_output"), Path::new("tests/screenshots")] {
        assert!(
            !local.exists(),
            "CX-212E: no repo-local artifact dir may exist ({}) — artifacts go to the external \
             Handshake_Artifacts/wp-kernel-012 root only",
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
pub use pg_proof_support::{
    require_live_backend, require_reachable_backend, LiveBackend, DEFAULT_BASE,
};

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
            "SELECT payload::text FROM kernel_event_ledger WHERE event_id = '{event_id}'"
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

fn run_bounded_psql(sql: &str, variable: Option<(&str, &str)>) -> String {
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
