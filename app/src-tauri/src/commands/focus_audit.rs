//! MT-027 — Focus-audit Tauri IPC.
//!
//! HBR-QUIET-001 forbids any Handshake-owned window from stealing the
//! foreground while a model is working. `operator_foreground::focus_audit`
//! (FocusAuditLedger / FocusAuditHandle / FocusAuditReport) implements the
//! real passive Win32 foreground-audit, but before this module it had **no IPC
//! surface** — nothing in the running app could START or STOP it, so the a2
//! visual smoke had to hardcode `handshake_owned_events: []` (a tautology that
//! never exercised the real ledger).
//!
//! This module exposes two real commands that wrap
//! `FocusAuditHandle::start` / `FocusAuditHandle::stop`:
//!
//!   * `kernel_operator_foreground_focus_audit_start(run_id, runtime_root?)`
//!     installs the real `WindowEventHook` foreground hook (via the core
//!     `FocusAuditHandle`), parks the live handle in managed state keyed by
//!     `run_id`, and returns the ledger path so the caller can locate the
//!     JSONL evidence.
//!   * `kernel_operator_foreground_focus_audit_stop(run_id)` removes the parked
//!     handle, unhooks, drains the ledger, and returns the **real**
//!     `FocusAuditReport` — including the genuine `handshake_owned_events`
//!     vector. The a2 smoke asserts that vector is empty to prove the QUIET
//!     guarantee for the captured run.
//!
//! The handle map keys on the caller-supplied `run_id` so a visual run can
//! correlate the start/stop pair with the `run_id` baked into its capture
//! report. Starting a second audit for an already-active `run_id` is a typed
//! error rather than a silent overwrite (which would orphan the first hook).

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

use handshake_core::operator_foreground::focus_audit::{
    FocusAuditError, FocusAuditHandle, FocusAuditReport, OwnedProcessPidSet,
};
use serde::{Deserialize, Serialize};

pub const KERNEL_OPERATOR_FOREGROUND_FOCUS_AUDIT_START_IPC_CHANNEL: &str =
    "kernel_operator_foreground_focus_audit_start";
pub const KERNEL_OPERATOR_FOREGROUND_FOCUS_AUDIT_STOP_IPC_CHANNEL: &str =
    "kernel_operator_foreground_focus_audit_stop";

/// Managed Tauri state holding the live focus-audit handles keyed by `run_id`.
///
/// A `FocusAuditHandle` owns a real `WindowEventHook` plus a background tokio
/// task draining foreground events into the ledger, so it must live across the
/// start -> (visual run) -> stop IPC round-trip. We park it here between calls.
#[derive(Default)]
pub struct FocusAuditIpcState {
    handles: Mutex<HashMap<String, FocusAuditHandle>>,
}

impl FocusAuditIpcState {
    pub fn new() -> Self {
        Self::default()
    }

    fn insert(&self, run_id: String, handle: FocusAuditHandle) -> Result<(), String> {
        let mut guard = self
            .handles
            .lock()
            .map_err(|_| "FOCUS_AUDIT_IPC_STATE_POISONED".to_string())?;
        if guard.contains_key(&run_id) {
            return Err(format!(
                "FOCUS_AUDIT_ALREADY_ACTIVE: a focus audit is already running for run_id {run_id}"
            ));
        }
        guard.insert(run_id, handle);
        Ok(())
    }

    fn remove(&self, run_id: &str) -> Result<Option<FocusAuditHandle>, String> {
        let mut guard = self
            .handles
            .lock()
            .map_err(|_| "FOCUS_AUDIT_IPC_STATE_POISONED".to_string())?;
        Ok(guard.remove(run_id))
    }

    /// Test/diagnostic helper: number of currently parked audits.
    pub fn active_count(&self) -> usize {
        self.handles.lock().map(|guard| guard.len()).unwrap_or(0)
    }
}

/// Result of starting a focus audit. The ledger path lets the caller locate the
/// real JSONL evidence the running hook appends foreground events to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusAuditStartReport {
    pub run_id: String,
    pub ledger_path: String,
    pub runtime_root: String,
}

/// Resolve the runtime root the ledger is written under. Honors an explicit
/// caller override (used by the a2 smoke so its evidence lands in the test
/// artifact tree), otherwise falls back to `HANDSHAKE_WORKSPACE_ROOT` and
/// finally the process current directory — never a hardcoded absolute path, per
/// the disk-agnostic portability policy.
fn resolve_runtime_root(runtime_root: Option<String>) -> PathBuf {
    if let Some(root) = runtime_root {
        let trimmed = root.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    if let Ok(value) = std::env::var("HANDSHAKE_WORKSPACE_ROOT") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn map_focus_audit_error(error: FocusAuditError) -> String {
    error.to_string()
}

/// Start the real foreground focus audit for `run_id`.
///
/// Wraps `FocusAuditHandle::start`, which installs the live Win32
/// `WindowEventHook` (`SYSTEM_FOREGROUND`, `skip_own_process(true)`) and spawns
/// the ledger-draining task. The owned-process set seeds with the running app's
/// own pid via `FocusAuditReport::from_events`' `current_pid` check; additional
/// owned pids can be threaded in later from the process ledger without changing
/// this IPC contract.
#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_operator_foreground_focus_audit_start(
    run_id: String,
    runtime_root: Option<String>,
    state: tauri::State<'_, FocusAuditIpcState>,
) -> Result<FocusAuditStartReport, String> {
    let _ = KERNEL_OPERATOR_FOREGROUND_FOCUS_AUDIT_START_IPC_CHANNEL;
    let run_id = run_id.trim().to_string();
    if run_id.is_empty() {
        return Err("FOCUS_AUDIT_INVALID_RUN_ID: run_id must not be empty".to_string());
    }
    let root = resolve_runtime_root(runtime_root);

    let handle = FocusAuditHandle::start(run_id.clone(), &root, OwnedProcessPidSet::default())
        .await
        .map_err(map_focus_audit_error)?;

    let ledger_path = handle.ledger_path().to_path_buf();
    state.insert(run_id.clone(), handle)?;

    Ok(FocusAuditStartReport {
        run_id,
        ledger_path: path_to_string(&ledger_path),
        runtime_root: path_to_string(&root),
    })
}

/// Stop the focus audit for `run_id` and return the **real** report.
///
/// Wraps `FocusAuditHandle::stop`: unhooks the Win32 hook, joins the drain
/// task, reads the ledger, and classifies events into
/// `handshake_owned_events` / `foreign_events` / `expected_foreground_events`.
/// The a2 smoke asserts `handshake_owned_events.is_empty()` to prove no
/// Handshake-owned window stole the foreground during the captured run
/// (HBR-QUIET-001).
#[tauri::command(rename_all = "snake_case")]
pub async fn kernel_operator_foreground_focus_audit_stop(
    run_id: String,
    state: tauri::State<'_, FocusAuditIpcState>,
) -> Result<FocusAuditReport, String> {
    let _ = KERNEL_OPERATOR_FOREGROUND_FOCUS_AUDIT_STOP_IPC_CHANNEL;
    let run_id = run_id.trim().to_string();
    let handle = state.remove(&run_id)?.ok_or_else(|| {
        format!("FOCUS_AUDIT_NOT_ACTIVE: no focus audit is running for run_id {run_id}")
    })?;

    handle.stop().await.map_err(map_focus_audit_error)
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_runtime_root_prefers_explicit_override() {
        let root = resolve_runtime_root(Some("  /tmp/focus-audit-root  ".to_string()));
        assert_eq!(root, PathBuf::from("/tmp/focus-audit-root"));
    }

    #[test]
    fn resolve_runtime_root_ignores_blank_override() {
        // A blank override falls through to env / cwd rather than producing an
        // empty path; we only assert it does not equal the blank string.
        let root = resolve_runtime_root(Some("   ".to_string()));
        assert_ne!(root, PathBuf::from(""));
    }

    #[test]
    fn ipc_state_starts_empty() {
        let state = FocusAuditIpcState::new();
        assert_eq!(state.active_count(), 0);
    }

    // The non-Windows `FocusAuditHandle::start` returns `UnsupportedPlatform`,
    // so this exercises the real error-mapping path end-to-end on dev (Linux)
    // lanes without faking a hook.
    #[cfg(not(windows))]
    #[tokio::test]
    async fn start_surfaces_unsupported_platform_error_off_windows() {
        let temp = tempfile::tempdir().expect("temp dir");
        let result =
            FocusAuditHandle::start("RUN-MT-027", temp.path(), OwnedProcessPidSet::default()).await;
        let error = result.err().expect("non-windows start must error");
        assert_eq!(
            map_focus_audit_error(error),
            "FOCUS_AUDIT_UNSUPPORTED_PLATFORM"
        );
    }
}
