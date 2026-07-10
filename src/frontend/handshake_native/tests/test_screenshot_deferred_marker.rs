//! WP-KERNEL-012 MT-108 residual item (11) / AC-108-2 — the screenshot proof harness must emit an
//! explicit typed DEFERRED/BLOCKED marker instead of silently passing when no wgpu adapter exists, so a
//! GREEN suite on a headless host never implies pixels were captured.
//!
//! These tests exercise the shared `screenshot_marker` module at runtime:
//!  1. On a headless host (the default: `HANDSHAKE_GPU_SCREENSHOT` unset) a screenshot proof records a
//!     typed `DEFERRED` marker artifact, parseable back from JSONL, NOT a silent green with no record.
//!  2. The `record_screenshot_outcome` wiring maps a render `Err(_)` to `DEFERRED` and a render `Ok`
//!     (a saved PNG path) to `CAPTURED`, so a real capture and an environment skip are distinguishable
//!     in the artifact.
//!  3. The marker is real JSONL: each line round-trips through serde with the mandated schema id.
//!
//! Runs unconditionally (NOT `#[ignore]`) so the marker discipline is enforced on every headless run.

use std::path::PathBuf;

#[path = "native_gui_support/screenshot_marker.rs"]
mod screenshot_marker;

use screenshot_marker::{
    gpu_screenshot_enabled, record_screenshot_outcome, ScreenshotMarker, ScreenshotStatus,
    SCREENSHOT_MARKER_FILE, SCREENSHOT_MARKER_SCHEMA_ID,
};

const MT_ID: &str = "MT-108";

/// A unique temp marker dir under the OS temp root so the test never writes into the repo and never
/// collides with a parallel run. (We assert on the returned marker value + a directly written file,
/// not the shared external artifact dir, to keep the assertion deterministic.)
fn temp_marker_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("hsk-mt108-screenshot-marker-{tag}-{nanos}"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

// ── AC-108-2: headless run produces a typed DEFERRED marker, not silent GREEN ─────────────────────

#[test]
fn headless_screenshot_proof_emits_typed_deferred_marker() {
    // Default host posture: pixel screenshots are NOT declared available.
    assert!(
        !gpu_screenshot_enabled(),
        "this proof asserts the headless path; run without HANDSHAKE_GPU_SCREENSHOT set"
    );

    // Simulate a screenshot proof whose Harness::render() returned Err (no wgpu adapter) — the exact
    // situation the audit flagged. The wiring must record a typed DEFERRED marker.
    let dir = temp_marker_dir("deferred");
    let marker = ScreenshotMarker::deferred(
        MT_ID,
        "MT-108-headless-marker",
        "no wgpu adapter on this host; pixel proof gated to a real-GPU host",
    );
    let path = marker
        .write_jsonl(&dir)
        .expect("marker JSONL is writable to a temp dir");

    assert_eq!(
        marker.status,
        ScreenshotStatus::Deferred,
        "AC-108-2: a headless screenshot proof records DEFERRED, never a silent capture"
    );
    assert_eq!(marker.schema_id, SCREENSHOT_MARKER_SCHEMA_ID);
    assert!(marker.frame_path.is_none(), "no pixels -> no frame path");
    assert!(
        !marker.gpu_screenshot_enabled,
        "the marker records the headless host posture"
    );

    // The artifact is real, on-disk, and typed: read it back and parse the JSONL row.
    assert_eq!(path.file_name().unwrap(), SCREENSHOT_MARKER_FILE);
    let contents = std::fs::read_to_string(&path).expect("marker file readable");
    let line = contents.lines().next().expect("one JSONL row written");
    let parsed: ScreenshotMarker =
        serde_json::from_str(line).expect("AC-108-2: the marker row is valid typed JSONL");
    assert_eq!(parsed.status, ScreenshotStatus::Deferred);
    assert_eq!(parsed.mt_id, MT_ID);
    assert_eq!(parsed.schema_id, SCREENSHOT_MARKER_SCHEMA_ID);

    let _ = std::fs::remove_dir_all(&dir);
    println!(
        "AC-108-2: headless screenshot proof emitted a typed DEFERRED marker ({}), not a silent GREEN",
        parsed.to_jsonl_line()
    );
}

// ── record_screenshot_outcome maps Err->DEFERRED and Ok(path)->CAPTURED ───────────────────────────

#[test]
fn record_outcome_distinguishes_captured_from_deferred() {
    // Err render result (headless) -> DEFERRED, no frame path.
    let deferred = record_screenshot_outcome(
        MT_ID,
        "MT-108-outcome-err",
        Err("adapter request returned None".to_owned()),
    );
    assert_eq!(deferred.status, ScreenshotStatus::Deferred);
    assert!(deferred.frame_path.is_none());
    assert!(
        deferred.reason.contains("no wgpu adapter"),
        "the deferred reason names the missing adapter: {}",
        deferred.reason
    );

    // Ok render result (a saved PNG path) -> CAPTURED, with the frame path recorded.
    let captured = record_screenshot_outcome(
        MT_ID,
        "MT-108-outcome-ok",
        Ok("/tmp/frame.png".to_owned()),
    );
    assert_eq!(
        captured.status,
        ScreenshotStatus::Captured,
        "a real render outcome records CAPTURED, distinguishable from a headless skip"
    );
    assert_eq!(captured.frame_path.as_deref(), Some("/tmp/frame.png"));
    println!(
        "AC-108-2: outcome wiring distinguishes CAPTURED({:?}) from DEFERRED({:?})",
        captured.frame_path, deferred.frame_path
    );
}

// ── the marker file is real JSONL: every line round-trips through serde ────────────────────────────

#[test]
fn marker_file_is_valid_jsonl_round_trip() {
    let dir = temp_marker_dir("jsonl");
    let markers = [
        ScreenshotMarker::deferred(MT_ID, "s1", "headless"),
        ScreenshotMarker::captured(MT_ID, "s2", "/tmp/s2.png"),
        ScreenshotMarker::blocked(MT_ID, "s3", "expected pixels but adapter lost"),
    ];
    let mut path = None;
    for m in &markers {
        path = Some(m.write_jsonl(&dir).expect("write marker"));
    }
    let path = path.unwrap();
    let contents = std::fs::read_to_string(&path).expect("read markers");
    let rows: Vec<&str> = contents.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(rows.len(), 3, "one JSONL row per marker appended");
    for row in rows {
        let parsed: ScreenshotMarker =
            serde_json::from_str(row).expect("each row is valid typed JSONL");
        assert_eq!(parsed.schema_id, SCREENSHOT_MARKER_SCHEMA_ID);
        assert_eq!(parsed.mt_id, MT_ID);
    }
    let _ = std::fs::remove_dir_all(&dir);
}
