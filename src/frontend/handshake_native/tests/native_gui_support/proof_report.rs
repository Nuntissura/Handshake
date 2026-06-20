//! WP-KERNEL-011 MT-029 — shared proof-report struct for the visual + interaction proof harness.
//!
//! ## What this is
//!
//! `ProofReport` is the machine-readable evidence artifact every MT-029 harness run emits. Both
//! `visual_interaction_proof.rs` (in-process logical/AccessKit proofs across the whole shell) and
//! `uia_steer_proof.rs` (out-of-process steer proof) build one `ProofReport`, write it as JSONL, and
//! CI gates on `overall_status == PASS`. It is the native equivalent of the legacy React harness
//! publishing state on `window.__*` for Playwright (the contract's `ports_from_react` reference): the
//! native shell writes the proof to disk instead.
//!
//! ## Why this lives under `tests/` and not `src/frontend/test_harness/`
//!
//! The MT-029 contract body assumed a workspace-root layout (`src/frontend/test_harness/`,
//! `tests/native_gui/`). The ACTUAL crate is `src/frontend/handshake_native`, whose only proof home is
//! its own `tests/` dir (every prior MT — MT-025/026/027 — lands its proofs there as crate integration
//! tests, not at a workspace root that does not exist). Placing `ProofReport` in `src/` would put a
//! test-only struct into the shipped product crate, which the rubric (Architecture Fit) and the
//! Closure-Unit discipline both argue against. It is a `#[path]`-included test-support module so both
//! harness test binaries share the exact same struct, mirroring the contract's "shared report struct"
//! intent. See the MT-029 handoff DEVIATION notes.
//!
//! ## Schema
//!
//! `schema_id = "hsk.native_gui.proof_report@1"` — exactly the id the contract mandates. Serialised
//! via serde_json so CI can parse each JSONL line with `serde_json::from_str` (AC-029-07).

#![allow(dead_code)] // each test binary uses a subset of this shared module's surface.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// The stable schema id for the proof-report artifact (contract-mandated).
pub const PROOF_REPORT_SCHEMA_ID: &str = "hsk.native_gui.proof_report@1";

/// The proof-report JSONL artifact file name. One `ProofReport` is appended per harness run.
pub const PROOF_REPORT_FILE: &str = "proof_report.jsonl";

/// Per-scenario PASS/FAIL verdict. A scenario is only `Pass` once every assertion in it held; any
/// failed assertion records `Fail` with a human/model-readable `reason` so a no-context reader knows
/// exactly which invariant broke without re-running the harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ProofStatus {
    Pass,
    Fail,
    /// The scenario could not run here (e.g. a GPU-gated pixel proof on a headless host). Distinct
    /// from `Fail` so a skipped-by-environment scenario never turns the run red, but is still visible
    /// in the artifact (honest blocker, not a fake pass).
    Skipped,
}

impl ProofStatus {
    fn is_terminal_fail(self) -> bool {
        matches!(self, ProofStatus::Fail)
    }
}

/// One scenario's result. `snapshot_path` / `frame_path` point at the on-disk AccessKit-tree JSON and
/// (when a GPU host renders one) the frame PNG, so the artifact links a verdict to its evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioResult {
    /// Stable scenario id (e.g. `"split-h"`, `"dark-theme"`, `"steer-cmd-palette"`).
    pub id: String,
    pub status: ProofStatus,
    /// Why a scenario failed/was skipped, or a short PASS note. `None` only for a bare PASS.
    pub reason: Option<String>,
    /// Path to the scenario's AccessKit-tree JSON snapshot, when one was written.
    pub snapshot_path: Option<String>,
    /// Path to the scenario's rendered frame PNG, when a GPU host produced one (None on headless).
    pub frame_path: Option<String>,
    /// Number of stable author_id widgets asserted present in this scenario (proof the assertion set
    /// was non-trivial — a scenario that checked nothing must not read as a meaningful PASS).
    pub asserted_widget_count: usize,
}

impl ScenarioResult {
    pub fn pass(id: impl Into<String>, asserted_widget_count: usize, note: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: ProofStatus::Pass,
            reason: Some(note.into()),
            snapshot_path: None,
            frame_path: None,
            asserted_widget_count,
        }
    }

    pub fn fail(id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: ProofStatus::Fail,
            reason: Some(reason.into()),
            snapshot_path: None,
            frame_path: None,
            asserted_widget_count: 0,
        }
    }

    pub fn skipped(id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: ProofStatus::Skipped,
            reason: Some(reason.into()),
            snapshot_path: None,
            frame_path: None,
            asserted_widget_count: 0,
        }
    }

    pub fn with_snapshot_path(mut self, path: impl Into<String>) -> Self {
        self.snapshot_path = Some(path.into());
        self
    }

    pub fn with_frame_path(mut self, path: impl Into<String>) -> Self {
        self.frame_path = Some(path.into());
        self
    }
}

/// The full proof-report for one harness run. `overall_status` is derived: `Fail` if ANY scenario
/// failed, else `Pass` (skipped scenarios do not fail the run — they are honest environment gaps).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofReport {
    pub schema_id: String,
    pub mt_id: String,
    pub run_id: String,
    pub scenarios: Vec<ScenarioResult>,
    pub overall_status: ProofStatus,
    pub duration_ms: u128,
}

impl ProofReport {
    /// Build a report from a scenario list, deriving `overall_status` and stamping a unix-nanos run id.
    pub fn new(mt_id: impl Into<String>, scenarios: Vec<ScenarioResult>, duration_ms: u128) -> Self {
        let overall_status = if scenarios.iter().any(|s| s.status.is_terminal_fail()) {
            ProofStatus::Fail
        } else {
            ProofStatus::Pass
        };
        Self {
            schema_id: PROOF_REPORT_SCHEMA_ID.to_owned(),
            mt_id: mt_id.into(),
            run_id: run_id_now(),
            scenarios,
            overall_status,
            duration_ms,
        }
    }

    /// Count of scenarios with the given status.
    pub fn count(&self, status: ProofStatus) -> usize {
        self.scenarios.iter().filter(|s| s.status == status).count()
    }

    /// Serialize this report as a single JSON line (no embedded newlines) — one JSONL row.
    pub fn to_jsonl_line(&self) -> String {
        // `to_string` (compact) guarantees no interior newline, so each report is exactly one line.
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_owned())
    }

    /// Append this report as one line to `<dir>/proof_report.jsonl`, creating the dir if needed.
    /// Returns the path written, so the caller can print it as proof output.
    pub fn write_jsonl(&self, dir: &Path) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(PROOF_REPORT_FILE);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        let mut line = self.to_jsonl_line();
        line.push('\n');
        file.write_all(line.as_bytes())?;
        Ok(path)
    }
}

/// Resolve the proof-artifact directory.
///
/// Honors `HANDSHAKE_PROOF_ARTIFACT_DIR` (CI override). Otherwise defaults to the protocol-mandated
/// external artifact root `../Handshake_Artifacts/handshake-test/native_gui/` relative to the crate
/// (CODER_PROTOCOL [CX-212E]: build/test outputs live under `../Handshake_Artifacts/`, NEVER inside
/// the repo). The crate sits four levels below the worktree root (`src/frontend/handshake_native`),
/// so `CARGO_MANIFEST_DIR` + `../../../../Handshake_Artifacts/...` lands beside the worktree, exactly
/// where the cargo target dir already is (`.cargo/config.toml`).
pub fn artifact_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("HANDSHAKE_PROOF_ARTIFACT_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .join("../../../../Handshake_Artifacts/handshake-test/native_gui")
}

/// A unix-nanos run id string, distinct per run so concurrent harness runs do not collide reports.
fn run_id_now() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => format!("run-{}-{:09}", d.as_secs(), d.subsec_nanos()),
        Err(_) => "run-0".to_owned(),
    }
}
