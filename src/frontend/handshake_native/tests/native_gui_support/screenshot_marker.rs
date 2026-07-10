//! WP-KERNEL-012 MT-108 residual item (11) / AC-108-2 — typed screenshot-proof marker.
//!
//! ## What this is and why it exists
//!
//! Several MT screenshot proofs (`test_find_bar_accesskit.rs`, `test_app_host_mount.rs`,
//! `test_canvas_board.rs`, …) call `egui_kittest::Harness::render()` to capture a PNG, and on a host
//! with no usable wgpu adapter they fall into an `Err(_)` branch that just prints a "BLOCKER(non-fatal)"
//! note and lets the test pass GREEN. The wave-A adversarial audit flagged this as a systemic hazard:
//! a GREEN suite on a headless host silently implies "pixels were captured" when in fact none were, so
//! a real screenshot regression could hide behind an environment that never renders.
//!
//! This module makes that gap explicit. Instead of a silent pass, a screenshot proof emits a **typed,
//! machine-readable marker** recording exactly what happened — `CAPTURED` with a frame path, or
//! `DEFERRED`/`BLOCKED` with a reason — as one JSONL row. GREEN can still be green on a headless host,
//! but the artifact now proves whether pixels were actually produced, so downstream tooling (and a
//! no-context reader) can tell a real pixel proof from an environment-skipped one.
//!
//! ## Schema
//!
//! `schema_id = "hsk.native_gui.screenshot_marker@1"`. Serialised with serde_json so each JSONL line is
//! parseable with `serde_json::from_str`. Lives under `tests/native_gui_support/` and is `#[path]`
//! -included by the screenshot proof test binaries, mirroring the sibling `proof_report.rs` convention.

#![allow(dead_code)] // each test binary uses a subset of this shared module's surface.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// The stable schema id for the screenshot-marker artifact.
pub const SCREENSHOT_MARKER_SCHEMA_ID: &str = "hsk.native_gui.screenshot_marker@1";

/// The screenshot-marker JSONL artifact file name. One marker is appended per screenshot-proof outcome.
pub const SCREENSHOT_MARKER_FILE: &str = "screenshot_marker.jsonl";

/// Env var a real-GPU host sets (to `1`/`true`) to declare that pixel screenshots are expected to
/// succeed. Absent/false means the run is treated as headless: a screenshot proof records a `DEFERRED`
/// marker rather than pretending pixels were captured. This is the deterministic, crash-free signal —
/// `Harness::render()` readback can raise an uncatchable STATUS_ACCESS_VIOLATION on a headless-GPU host,
/// so we must NOT probe by attempting a render in an always-run test.
pub const GPU_SCREENSHOT_ENV: &str = "HANDSHAKE_GPU_SCREENSHOT";

/// Whether pixel screenshots are declared available on this host (see [`GPU_SCREENSHOT_ENV`]).
pub fn gpu_screenshot_enabled() -> bool {
    matches!(
        std::env::var(GPU_SCREENSHOT_ENV)
            .ok()
            .as_deref()
            .map(str::trim),
        Some("1") | Some("true") | Some("TRUE") | Some("yes")
    )
}

/// Outcome of a screenshot proof. `Captured` is the only status that asserts real pixels were produced;
/// `Deferred`/`Blocked` are honest "no pixels here" records that keep GREEN from implying a capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ScreenshotStatus {
    /// A frame PNG was rendered and saved (real pixels).
    Captured,
    /// No wgpu adapter / GPU screenshots disabled on this host; the pixel proof is gated to a real-GPU
    /// host. Not a failure — a recorded environment gap.
    Deferred,
    /// A screenshot was expected here but could not be produced for a non-environment reason.
    Blocked,
}

/// One screenshot-proof outcome, written as a single JSONL row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenshotMarker {
    pub schema_id: String,
    pub mt_id: String,
    /// Stable scenario id (e.g. `"MT-004-find-highlight"`, `"MT-079-editors-mounted"`).
    pub scenario_id: String,
    pub status: ScreenshotStatus,
    /// Why the screenshot was deferred/blocked, or a short capture note.
    pub reason: String,
    /// Path to the saved frame PNG when `status == Captured`; `None` otherwise.
    pub frame_path: Option<String>,
    /// Snapshot of [`gpu_screenshot_enabled`] at write time, so a reader sees the host posture without
    /// re-deriving it.
    pub gpu_screenshot_enabled: bool,
    pub timestamp_nanos: u128,
}

impl ScreenshotMarker {
    fn build(
        mt_id: impl Into<String>,
        scenario_id: impl Into<String>,
        status: ScreenshotStatus,
        reason: impl Into<String>,
        frame_path: Option<String>,
    ) -> Self {
        Self {
            schema_id: SCREENSHOT_MARKER_SCHEMA_ID.to_owned(),
            mt_id: mt_id.into(),
            scenario_id: scenario_id.into(),
            status,
            reason: reason.into(),
            frame_path,
            gpu_screenshot_enabled: gpu_screenshot_enabled(),
            timestamp_nanos: now_nanos(),
        }
    }

    /// A real-pixels marker: a frame was rendered and saved to `frame_path`.
    pub fn captured(
        mt_id: impl Into<String>,
        scenario_id: impl Into<String>,
        frame_path: impl Into<String>,
    ) -> Self {
        Self::build(
            mt_id,
            scenario_id,
            ScreenshotStatus::Captured,
            "frame rendered and saved",
            Some(frame_path.into()),
        )
    }

    /// A headless/deferred marker: no pixels were produced; the reason records why.
    pub fn deferred(
        mt_id: impl Into<String>,
        scenario_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::build(
            mt_id,
            scenario_id,
            ScreenshotStatus::Deferred,
            reason,
            None,
        )
    }

    /// A blocked marker: a screenshot was expected but could not be produced for a non-environment
    /// reason (records the reason so it is never a silent pass).
    pub fn blocked(
        mt_id: impl Into<String>,
        scenario_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::build(mt_id, scenario_id, ScreenshotStatus::Blocked, reason, None)
    }

    /// Serialize as a single compact JSON line (no interior newline) — one JSONL row.
    pub fn to_jsonl_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_owned())
    }

    /// Append this marker as one line to `<dir>/screenshot_marker.jsonl`, creating the dir if needed.
    /// Returns the path written so the caller can print it as proof output.
    pub fn write_jsonl(&self, dir: &Path) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(SCREENSHOT_MARKER_FILE);
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

/// Resolve the marker artifact directory. Honors `HANDSHAKE_PROOF_ARTIFACT_DIR` (CI override), else the
/// protocol external artifact root `../Handshake_Artifacts/handshake-test/native_gui/` beside the crate
/// (CODER_PROTOCOL [CX-212E]) — the same location `proof_report.rs` writes to.
pub fn marker_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("HANDSHAKE_PROOF_ARTIFACT_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../../../Handshake_Artifacts/handshake-test/native_gui")
}

/// Convenience wiring for a screenshot proof site: given the render result of `Harness::render()`
/// (mapped to either a saved PNG path or an error string), emit the correct typed marker to the default
/// [`marker_dir`] and return the marker written. Call this on BOTH branches so the artifact always
/// records the real outcome. Returns the marker (already written best-effort; a write error is
/// swallowed so proof flow is never blocked by an artifact-dir issue, but the marker value is still
/// returned for in-test assertions).
pub fn record_screenshot_outcome(
    mt_id: &str,
    scenario_id: &str,
    render_result: Result<String, String>,
) -> ScreenshotMarker {
    let marker = match render_result {
        Ok(frame_path) => ScreenshotMarker::captured(mt_id, scenario_id, frame_path),
        Err(reason) => ScreenshotMarker::deferred(
            mt_id,
            scenario_id,
            format!("no wgpu adapter / pixel readback unavailable: {reason}"),
        ),
    };
    let _ = marker.write_jsonl(&marker_dir());
    marker
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}
