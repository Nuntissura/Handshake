//! MT-019 — Controlled foreground-exception window Tauri IPC.
//!
//! Spec §6.6.7 / HBR-QUIET-004 require that a declared foreground exception
//! ACTUALLY create + show + focus a single bounded window and REALLY close it at
//! the deadline. `handshake_core` owns the declaration + bounded-window state
//! machine (`ForegroundException` / `ControlledWindow`) and the app layer
//! (`foreground_exception_window`) owns the real `tauri::WebviewWindow` surface.
//!
//! Before this module the real-window lifecycle was **unreachable**: nothing in
//! the running app invoked `build_foreground_exception_window`, so the live app
//! could never create/show/focus the real window and never auto-dismiss it. This
//! command closes that gap — it is the single runtime caller that:
//!
//!   1. DECLARES the exception (`ForegroundException::declare`), which appends
//!      the `FOREGROUND_EXCEPTION_START` audit row and emits the operator warning
//!      (notification + `DIAG_BANNER_REQUEST`) through the real app sink.
//!   2. CREATES + SHOWS + FOCUSES the real bounded Tauri window via
//!      `build_foreground_exception_window`.
//!   3. Spawns `ControlledWindow::auto_dismiss_at_deadline` on the live tokio
//!      runtime so the real window is REALLY closed at `max_duration`, with the
//!      `CONTROLLED_WINDOW_DISMISSED` / `FOREGROUND_EXCEPTION_END` rows appended.
//!
//! The command returns immediately with the exception/window identity and the
//! audit log path so the caller can locate the JSONL evidence; the auto-dismiss
//! runs in the background and tears the window down at the bounded deadline.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use handshake_core::operator_foreground::foreground_exception::{
    ForegroundException, ForegroundExceptionError, ForegroundPacketPolicy,
    ForegroundWarningRequest, ForegroundWarningSink,
};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::foreground_exception_window::build_foreground_exception_window;
use crate::foreground_warning::{
    emit_foreground_warning, ForegroundWarningRequest as AppForegroundWarningRequest,
};

pub const FOREGROUND_EXCEPTION_WINDOW_OPEN_IPC_CHANNEL: &str = "foreground_exception_window_open";

/// Default controlled-window URL when the caller does not supply one. This is an
/// app-relative (bundled) path so the window shows a local operator-warning
/// surface without depending on a live server.
const DEFAULT_FOREGROUND_EXCEPTION_WINDOW_URL: &str = "index.html";

/// Warning sink that emits the operator notification + `DIAG_BANNER_REQUEST`
/// through the real app surface (`foreground_warning::emit_foreground_warning`)
/// when an exception is declared. This is the production wiring of the
/// `ForegroundWarningSink` seam: declaring a foreground exception in the running
/// app raises the same operator warning the headless tests record.
struct AppForegroundWarningSink<'a> {
    app: &'a AppHandle,
}

impl ForegroundWarningSink for AppForegroundWarningSink<'_> {
    fn emit_warning(
        &self,
        request: ForegroundWarningRequest,
    ) -> Result<(), ForegroundExceptionError> {
        let app_request = AppForegroundWarningRequest {
            event_type: request.event_type.to_string(),
            diagnostics_event_type: request.diagnostics_event_type.to_string(),
            exception_id: request.exception_id.to_string(),
            wp_id: request.wp_id,
            reason: request.reason,
            max_duration_ms: request.max_duration_ms,
            notification_title: request.notification_title,
            notification_body: request.notification_body,
            diagnostics_banner_body: request.diagnostics_banner_body,
            timestamp_utc: request.timestamp_utc.to_rfc3339(),
        };
        emit_foreground_warning(self.app, &app_request)
            .map_err(ForegroundExceptionError::WindowSurface)
    }
}

/// Result of opening a controlled foreground-exception window. The window is
/// already shown + focused when this returns; the auto-dismiss runs in the
/// background and closes it at `max_duration_ms`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForegroundExceptionWindowReport {
    pub exception_id: String,
    pub wp_id: String,
    pub window_label: String,
    pub window_url: String,
    pub max_duration_ms: u64,
    pub log_path: String,
}

/// Resolve the repo root the foreground-exception audit log is written under.
///
/// Honors `HANDSHAKE_REPO_ROOT` (must contain a `.GOV` dir), otherwise walks up
/// from the current directory looking for a `.GOV` ancestor. Never a hardcoded
/// absolute path, per the disk-agnostic portability policy.
fn resolve_repo_root() -> Result<PathBuf, String> {
    if let Ok(value) = std::env::var("HANDSHAKE_REPO_ROOT") {
        let root = PathBuf::from(value);
        if root.join(".GOV").exists() {
            return Ok(root);
        }
    }
    let mut current = std::env::current_dir()
        .map_err(|error| format!("FOREGROUND_EXCEPTION_CWD_UNAVAILABLE: {error}"))?;
    for _ in 0..8 {
        if current.join(".GOV").exists() {
            return Ok(current);
        }
        if !current.pop() {
            break;
        }
    }
    Err("FOREGROUND_EXCEPTION_REPO_ROOT_UNAVAILABLE: no .GOV ancestor found".to_string())
}

/// Open the real, shown, focused, bounded controlled foreground-exception window
/// and arm its auto-dismiss.
///
/// This is the single runtime caller of `build_foreground_exception_window`.
/// It declares the exception (raising the operator warning + START audit row),
/// creates the real Tauri window, then spawns the bounded auto-dismiss on the
/// live tokio runtime so the window is REALLY closed at `max_duration_ms`.
#[tauri::command(rename_all = "snake_case")]
pub async fn foreground_exception_window_open(
    app: AppHandle,
    wp_id: String,
    requires_foreground: bool,
    reason: String,
    max_duration_ms: u64,
    window_label: String,
    window_url: Option<String>,
    repo_root: Option<String>,
) -> Result<ForegroundExceptionWindowReport, String> {
    let _ = FOREGROUND_EXCEPTION_WINDOW_OPEN_IPC_CHANNEL;

    let wp_id = normalize_wp_id(wp_id)?;
    let window_label = window_label.trim().to_string();
    if window_label.is_empty() {
        return Err(
            "FOREGROUND_EXCEPTION_INVALID_WINDOW_LABEL: window_label must not be empty".to_string(),
        );
    }
    if max_duration_ms == 0 {
        return Err(
            "FOREGROUND_EXCEPTION_INVALID_DURATION: max_duration_ms must be > 0".to_string(),
        );
    }
    let window_url = window_url
        .map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty())
        .unwrap_or_else(|| DEFAULT_FOREGROUND_EXCEPTION_WINDOW_URL.to_string());

    let repo_root = match repo_root {
        Some(root) if !root.trim().is_empty() => PathBuf::from(root.trim()),
        _ => resolve_repo_root()?,
    };
    let packet =
        foreground_packet_policy_from_packet(&repo_root, wp_id.clone(), requires_foreground)?;
    let wp_id = packet.wp_id.clone();

    let max_duration = Duration::from_millis(max_duration_ms);

    // Declare the exception: this appends FOREGROUND_EXCEPTION_START and raises
    // the operator warning (notification + DIAG_BANNER_REQUEST) through the real
    // app sink. The packet declaration comes from the caller; the command does
    // not self-authorize foreground control.
    let sink = AppForegroundWarningSink { app: &app };
    let handle = ForegroundException::declare(packet, reason, max_duration, &repo_root, &sink)
        .map_err(|error| format!("FOREGROUND_EXCEPTION_DECLARE_FAILED: {error}"))?;

    let exception_id = handle.exception_id().to_string();
    let log_path = path_to_string(handle.log_path());

    // Create + show + focus the REAL bounded Tauri window and attach the
    // bounded-window state machine. On success a real window is already on
    // screen and focused.
    let controlled =
        build_foreground_exception_window(&app, &handle, window_label.clone(), window_url.clone())
            .map_err(|error| format!("FOREGROUND_EXCEPTION_WINDOW_BUILD_FAILED: {error}"))?;

    // Arm the real auto-dismiss on the live tokio runtime: at max_duration the
    // window is actually closed and the CONTROLLED_WINDOW_DISMISSED /
    // FOREGROUND_EXCEPTION_END rows are appended. Capture the label for the
    // report before the ControlledWindow is moved into the background task.
    let report_label = controlled.label().to_string();
    let dismiss_label = report_label.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = controlled.auto_dismiss_at_deadline().await {
            eprintln!(
                "MT-019 foreground-exception window auto-dismiss failed for label \
                 {dismiss_label}: {error}"
            );
        }
    });

    Ok(ForegroundExceptionWindowReport {
        exception_id,
        wp_id,
        window_label: report_label,
        window_url,
        max_duration_ms,
        log_path,
    })
}

fn foreground_packet_policy_from_packet(
    repo_root: &Path,
    wp_id: String,
    caller_requires_foreground: bool,
) -> Result<ForegroundPacketPolicy, String> {
    let wp_id = normalize_wp_id(wp_id)?;
    if !caller_requires_foreground {
        return Err(format!(
            "FOREGROUND_REQUIRES_CALLER_ASSERTION: caller for packet {wp_id} must assert \
             requires_foreground=true before any foreground run"
        ));
    }
    let packet_path = resolve_packet_contract_path(repo_root, &wp_id)?;
    let raw = std::fs::read_to_string(&packet_path).map_err(|error| {
        format!(
            "FOREGROUND_PACKET_READ_FAILED: failed to read {}: {error}",
            packet_path.display()
        )
    })?;
    let packet: serde_json::Value = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "FOREGROUND_PACKET_JSON_INVALID: failed to parse {}: {error}",
            packet_path.display()
        )
    })?;
    if packet
        .get("requires_foreground")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return Err(format!(
            "FOREGROUND_REQUIRES_PACKET_DECLARATION: packet {} must set top-level \
             requires_foreground=true before any foreground run",
            packet_path.display()
        ));
    }
    let declared_wp_id = packet
        .get("wp_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "FOREGROUND_PACKET_WP_ID_MISSING: packet {} must declare a non-empty wp_id",
                packet_path.display()
            )
        })?;
    Ok(ForegroundPacketPolicy::new(declared_wp_id, true))
}

fn normalize_wp_id(wp_id: String) -> Result<String, String> {
    let wp_id = wp_id.trim().to_string();
    if wp_id.is_empty() {
        return Err("FOREGROUND_EXCEPTION_INVALID_WP_ID: wp_id must not be empty".to_string());
    }
    if wp_id.contains('/') || wp_id.contains('\\') || wp_id.contains("..") {
        return Err(format!(
            "FOREGROUND_EXCEPTION_INVALID_WP_ID: wp_id `{wp_id}` must be a packet id, not a path"
        ));
    }
    Ok(wp_id)
}

fn resolve_packet_contract_path(repo_root: &Path, wp_id: &str) -> Result<PathBuf, String> {
    let task_packets = repo_root.join(".GOV").join("task_packets");
    let direct = task_packets.join(wp_id).join("packet.json");
    if direct.exists() {
        return Ok(direct);
    }
    let entries = std::fs::read_dir(&task_packets).map_err(|error| {
        format!(
            "FOREGROUND_PACKET_ROOT_UNAVAILABLE: failed to read {}: {error}",
            task_packets.display()
        )
    })?;
    let mut matches = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "FOREGROUND_PACKET_ROOT_UNAVAILABLE: failed to enumerate {}: {error}",
                task_packets.display()
            )
        })?;
        if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            continue;
        }
        let packet_path = entry.path().join("packet.json");
        if !packet_path.exists() {
            continue;
        }
        let raw = match std::fs::read_to_string(&packet_path) {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        let Ok(packet) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };
        let wp_match = packet
            .get("wp_id")
            .and_then(serde_json::Value::as_str)
            .map(|value| value == wp_id)
            .unwrap_or(false);
        let base_match = packet
            .get("base_wp_id")
            .and_then(serde_json::Value::as_str)
            .map(|value| value == wp_id)
            .unwrap_or(false);
        if wp_match || base_match {
            matches.push(packet_path);
        }
    }
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => Err(format!(
            "FOREGROUND_PACKET_NOT_FOUND: no .GOV/task_packets packet.json found for wp_id `{wp_id}`"
        )),
        _ => Err(format!(
            "FOREGROUND_PACKET_AMBIGUOUS: multiple packet.json files match wp_id `{wp_id}`"
        )),
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    static HANDSHAKE_REPO_ROOT_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn resolve_repo_root_honors_env_with_gov_dir() {
        let _guard = HANDSHAKE_REPO_ROOT_ENV_LOCK
            .lock()
            .expect("env lock must not be poisoned");
        let temp = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(temp.path().join(".GOV")).expect(".GOV dir");
        // SAFETY: serialized by HANDSHAKE_REPO_ROOT_ENV_LOCK; we set + clear
        // the env var around the assertion so other tests are unaffected.
        std::env::set_var("HANDSHAKE_REPO_ROOT", temp.path());
        let resolved = resolve_repo_root().expect("repo root resolves from env");
        std::env::remove_var("HANDSHAKE_REPO_ROOT");
        assert_eq!(resolved, temp.path());
    }

    #[test]
    fn resolve_repo_root_ignores_env_without_gov_dir() {
        let _guard = HANDSHAKE_REPO_ROOT_ENV_LOCK
            .lock()
            .expect("env lock must not be poisoned");
        let temp = tempfile::tempdir().expect("temp dir");
        // No .GOV under temp -> the env override is rejected and resolution
        // falls back to the cwd walk (which must not return the bogus root).
        std::env::set_var("HANDSHAKE_REPO_ROOT", temp.path());
        let resolved = resolve_repo_root();
        std::env::remove_var("HANDSHAKE_REPO_ROOT");
        if let Ok(root) = resolved {
            assert_ne!(
                root,
                temp.path(),
                "a root without a .GOV dir must not be accepted from the env override"
            );
        }
    }

    #[test]
    fn report_serializes_with_snake_case_fields() {
        let report = ForegroundExceptionWindowReport {
            exception_id: "0190abcd-ef01-7000-8000-000000000000".to_string(),
            wp_id: "WP-KERNEL-004-FOREGROUND".to_string(),
            window_label: "foreground-exception-real-window".to_string(),
            window_url: "index.html".to_string(),
            max_duration_ms: 5_000,
            log_path: "/tmp/foreground_log/wp.jsonl".to_string(),
        };
        let value = serde_json::to_value(&report).expect("report serializes");
        assert_eq!(value["wp_id"], "WP-KERNEL-004-FOREGROUND");
        assert_eq!(value["window_label"], "foreground-exception-real-window");
        assert_eq!(value["max_duration_ms"], 5_000);
    }

    #[test]
    fn packet_policy_requires_caller_assertion_and_packet_declaration() {
        let temp = tempfile::tempdir().expect("temp dir");
        let packet_dir = temp.path().join(".GOV/task_packets/WP-FOREGROUND-v1");
        std::fs::create_dir_all(&packet_dir).expect("packet dir");
        std::fs::write(
            packet_dir.join("packet.json"),
            r#"{"wp_id":"WP-FOREGROUND-v1","base_wp_id":"WP-FOREGROUND","requires_foreground":true}"#,
        )
        .expect("packet json");

        let error = foreground_packet_policy_from_packet(
            temp.path(),
            "WP-FOREGROUND-v1".to_string(),
            false,
        )
        .expect_err("caller must assert the packet foreground declaration");
        assert!(
            error.contains("FOREGROUND_REQUIRES_CALLER_ASSERTION"),
            "{error}"
        );

        let packet = foreground_packet_policy_from_packet(
            temp.path(),
            "  WP-FOREGROUND-v1  ".to_string(),
            true,
        )
        .expect("true packet declaration accepted");
        assert_eq!(packet.wp_id, "WP-FOREGROUND-v1");
        assert!(packet.requires_foreground);
    }

    #[test]
    fn packet_policy_rejects_packet_without_requires_foreground() {
        let temp = tempfile::tempdir().expect("temp dir");
        let packet_dir = temp.path().join(".GOV/task_packets/WP-BACKGROUND-v1");
        std::fs::create_dir_all(&packet_dir).expect("packet dir");
        std::fs::write(
            packet_dir.join("packet.json"),
            r#"{"wp_id":"WP-BACKGROUND-v1","requires_foreground":false}"#,
        )
        .expect("packet json");
        let error =
            foreground_packet_policy_from_packet(temp.path(), "WP-BACKGROUND-v1".to_string(), true)
                .expect_err("packet authority must declare foreground");
        assert!(
            error.contains("FOREGROUND_REQUIRES_PACKET_DECLARATION"),
            "{error}"
        );
    }

    #[test]
    fn packet_policy_resolves_by_base_wp_id() {
        let temp = tempfile::tempdir().expect("temp dir");
        let packet_dir = temp.path().join(".GOV/task_packets/WP-FOREGROUND-v2");
        std::fs::create_dir_all(&packet_dir).expect("packet dir");
        std::fs::write(
            packet_dir.join("packet.json"),
            r#"{"wp_id":"WP-FOREGROUND-v2","base_wp_id":"WP-FOREGROUND","requires_foreground":true}"#,
        )
        .expect("packet json");

        let packet =
            foreground_packet_policy_from_packet(temp.path(), "WP-FOREGROUND".to_string(), true)
                .expect("base wp id resolves");
        assert_eq!(packet.wp_id, "WP-FOREGROUND-v2");
    }

    #[test]
    fn packet_policy_rejects_missing_declared_wp_id() {
        let temp = tempfile::tempdir().expect("temp dir");
        let packet_dir = temp.path().join(".GOV/task_packets/WP-FOREGROUND-v1");
        std::fs::create_dir_all(&packet_dir).expect("packet dir");
        std::fs::write(
            packet_dir.join("packet.json"),
            r#"{"base_wp_id":"WP-FOREGROUND","requires_foreground":true}"#,
        )
        .expect("packet json");

        let error =
            foreground_packet_policy_from_packet(temp.path(), "WP-FOREGROUND".to_string(), true)
                .expect_err("resolved packet authority must declare wp_id");
        assert!(error.contains("FOREGROUND_PACKET_WP_ID_MISSING"), "{error}");
    }
}
