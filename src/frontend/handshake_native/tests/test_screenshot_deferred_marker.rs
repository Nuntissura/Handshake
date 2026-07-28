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

#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;

use screenshot_harness::screenshot_marker::{
    gpu_screenshot_enabled, marker_dir, record_screenshot_outcome_to_dir, ScreenshotMarker,
    ScreenshotStatus, SCREENSHOT_MARKER_FILE, SCREENSHOT_MARKER_SCHEMA_ID,
};
use screenshot_harness::ScreenshotHarness as Harness;

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
    // Exercise the typed DEFERRED constructor independently of the host's declared GPU posture. A
    // real-GPU validation run must remain green while still proving the headless marker schema.
    let dir = temp_marker_dir("deferred");
    let marker = ScreenshotMarker::deferred(
        MT_ID,
        "MT-108-headless-marker",
        "headless-constructor-proof",
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
    assert_eq!(marker.gpu_screenshot_enabled, gpu_screenshot_enabled());

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
    let dir = temp_marker_dir("isolated-outcomes");
    // Err render result (headless) -> DEFERRED, no frame path.
    let deferred = record_screenshot_outcome_to_dir(
        &dir,
        MT_ID,
        "MT-108-outcome-err",
        "record-outcome-deferred",
        Err("adapter request returned None".to_owned()),
    )
    .expect("DEFERRED outcome is durably written");
    assert_eq!(deferred.status, ScreenshotStatus::Deferred);
    assert!(deferred.frame_path.is_none());
    assert!(
        deferred.reason.contains("no wgpu adapter"),
        "the deferred reason names the missing adapter: {}",
        deferred.reason
    );

    // CAPTURED is accepted only for a real, decodable non-zero PNG at the saved frame path.
    let frame = dir.join("frame.png");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([12, 34, 56, 255]))
        .save(&frame)
        .expect("create decodable PNG frame proof");
    let captured = record_screenshot_outcome_to_dir(
        &dir,
        MT_ID,
        "MT-108-outcome-ok",
        "record-outcome-captured",
        Ok(frame.display().to_string()),
    )
    .expect("CAPTURED outcome validates its frame and writes durably");
    assert_eq!(
        captured.status,
        ScreenshotStatus::Captured,
        "a real render outcome records CAPTURED, distinguishable from a headless skip"
    );
    assert_eq!(
        captured.frame_path.as_deref(),
        Some(frame.to_string_lossy().as_ref())
    );
    assert!(std::path::Path::new(captured.frame_path.as_deref().unwrap()).is_file());
    assert!(
        record_screenshot_outcome_to_dir(
            &dir,
            MT_ID,
            "MT-108-missing-frame",
            "record-outcome-missing-frame",
            Ok(dir.join("missing.png").display().to_string()),
        )
        .is_err(),
        "a fabricated/nonexistent CAPTURED path fails closed"
    );
    let invalid = dir.join("invalid.png");
    std::fs::write(&invalid, b"not-a-png").expect("create invalid image payload");
    assert!(
        record_screenshot_outcome_to_dir(
            &dir,
            MT_ID,
            "MT-108-invalid-frame",
            "record-outcome-invalid-frame",
            Ok(invalid.display().to_string()),
        )
        .is_err(),
        "a non-empty but undecodable CAPTURED artifact fails closed"
    );
    println!(
        "AC-108-2: outcome wiring distinguishes CAPTURED({:?}) from DEFERRED({:?})",
        captured.frame_path, deferred.frame_path
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── the marker file is real JSONL: every line round-trips through serde ────────────────────────────

#[test]
fn marker_file_is_valid_jsonl_round_trip() {
    let dir = temp_marker_dir("jsonl");
    let markers = [
        ScreenshotMarker::deferred(MT_ID, "s1", "jsonl-s1", "headless"),
        ScreenshotMarker::blocked(MT_ID, "s2", "jsonl-s2", "expected pixels but adapter lost"),
        ScreenshotMarker::blocked(MT_ID, "s3", "jsonl-s3", "durable proof example"),
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

#[test]
fn concurrent_marker_rows_remain_whole_and_distinct() {
    let dir = temp_marker_dir("concurrent-jsonl");
    let writers = (0..8)
        .map(|index| {
            let dir = dir.clone();
            std::thread::spawn(move || {
                ScreenshotMarker::deferred(
                    MT_ID,
                    format!("concurrent-{index}"),
                    format!("concurrent-outcome-{index}"),
                    "parallel marker integrity proof",
                )
                .write_jsonl(&dir)
                .expect("locked concurrent marker write")
            })
        })
        .collect::<Vec<_>>();
    for writer in writers {
        writer.join().expect("marker writer thread");
    }

    let contents = std::fs::read_to_string(dir.join(SCREENSHOT_MARKER_FILE))
        .expect("read concurrent marker artifact");
    let rows = contents
        .lines()
        .map(|line| serde_json::from_str::<ScreenshotMarker>(line).expect("whole JSONL row"))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 8);
    let outcome_ids = rows
        .iter()
        .map(|row| row.outcome_id.as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        outcome_ids.len(),
        8,
        "parallel rows retain distinct outcomes"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn isolated_render_writes_its_own_exact_runtime_outcome_for_declared_posture() {
    let mut harness = Harness::builder().wgpu().build_ui(|ui| {
        ui.label("runtime screenshot outcome proof");
    });
    harness.run();
    let rendered = harness.render();
    if gpu_screenshot_enabled() {
        assert!(
            rendered.is_ok(),
            "a declared real-GPU run must produce pixels and CAPTURED evidence: {rendered:?}"
        );
    } else {
        let error = rendered.expect_err("headless render is explicitly deferred");
        assert!(error.contains("typed DEFERRED"));
    }

    let contents = std::fs::read_to_string(marker_dir().join(SCREENSHOT_MARKER_FILE))
        .expect("isolated render wrote its run-scoped outcome");
    let current_test = "isolated_render_writes_its_own_exact_runtime_outcome_for_declared_posture";
    let rows = contents
        .lines()
        .filter_map(|line| serde_json::from_str::<ScreenshotMarker>(line).ok())
        .filter(|row| row.scenario_id.contains(current_test))
        .collect::<Vec<_>>();
    assert_eq!(
        rows.len(),
        1,
        "this isolated render has one exact runtime row"
    );
    assert_eq!(
        rows[0].status,
        if gpu_screenshot_enabled() {
            ScreenshotStatus::Captured
        } else {
            ScreenshotStatus::Deferred
        }
    );
    if rows[0].status == ScreenshotStatus::Captured {
        assert!(
            rows[0]
                .frame_path
                .as_deref()
                .is_some_and(|path| std::path::Path::new(path).is_file()),
            "CAPTURED runtime evidence retains a live frame"
        );
    }
    assert!(!rows[0].run_id.is_empty() && !rows[0].outcome_id.is_empty());
}

#[test]
fn every_render_site_is_routed_through_the_runtime_harness() {
    let tests_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut render_files = 0usize;
    for entry in std::fs::read_dir(tests_root).expect("integration-test source root readable") {
        let path = entry.expect("test entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("test source readable");
        if source.contains(".render()") {
            render_files += 1;
            assert!(
                source.contains("native_gui_support/screenshot_harness.rs"),
                "{} can render but bypasses the runtime outcome harness",
                path.display()
            );
        }
    }
    assert!(
        render_files >= 70,
        "unexpectedly small render corpus: {render_files}"
    );
}

#[test]
fn durable_marker_write_failure_is_returned() {
    let dir = temp_marker_dir("write-failure");
    let not_a_directory = dir.join("ordinary-file");
    std::fs::write(&not_a_directory, b"file").expect("create collision file");
    let marker = ScreenshotMarker::deferred(MT_ID, "write-failure", "write-failure", "proof");
    assert!(
        marker.write_jsonl(&not_a_directory).is_err(),
        "durable marker failure is propagated, not swallowed"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
