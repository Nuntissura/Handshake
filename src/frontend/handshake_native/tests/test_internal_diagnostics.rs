//! WP-KERNEL-012 MT-082 (D2 — internal_diagnostics, Tier 2) runtime proofs.
//!
//! Maps each acceptance criterion to a REAL runtime proof (no tautologies, no mocks for spec-required
//! behavior):
//!
//! - AC-002-1 / PT-002-A (`record_writes_ring_and_buffer`): a `DiagnosticsRecorder` built with a REAL
//!   `DiagRingWriter` over a temp backing file records N events; `snapshot_last_n` returns them
//!   in-process AND a SEPARATELY-constructed `DiagRingReader::open` on the SAME backing file reads them
//!   back — proving `record()` writes the shared-memory ring (what Palmistry maps).
//! - AC-002-2 / PT-002-B (`record_is_nonblocking_panic_free_under_stress`): many threads hammer
//!   `record()` thousands of times against a recorder whose buffer is far smaller than the write count;
//!   no panic, no deadlock, the buffer stays `<= BUFFER_CAP`, and `dropped_count` accounts for every
//!   shed event (writes == survivors + dropped).
//! - AC-002-3 / PT (`record_degrades_gracefully_with_no_writer`): with NO writer installed, `record()`
//!   still buffers in-process and never errors/panics (the headless/test path).
//! - AC-002-4 / PT-002-C (`live_startup_marker_is_recorded`): a kittest drives the REAL `HandshakeApp`
//!   startup (`HandshakeApp::new(cc)` via `build_eframe`) and asserts the process-global
//!   `diagnostics::snapshot_last_n` is non-empty AND contains the `PaneMounted` startup marker — the
//!   event came from PRODUCTION code (the live call site), not from the test (anti-dead-code gate).
//! - AC-002-5 / PT (`public_api_has_no_free_text_surface`): a source scan of the diagnostics module
//!   confirms the public `record` / `record_with` API declares NO `String` / `&str` / `[u8]` content
//!   parameter — the typed-allowlist invariant (§5.8.3) held at the API boundary.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use handshake_diag_ring::{
    DiagEvent, DiagEventCode, DiagPhase, DiagRingReader, DiagRingWriter, DiagSeverity,
    DEFAULT_CAPACITY,
};
use handshake_native::diagnostics::{self, DiagnosticsRecorder, BUFFER_CAP};

// ── artifact hygiene (CX-212E): no repo-local artifact dir may exist ───────────────────────────────

/// The external artifact root for any MT-082 test output (none is written here — the proofs are all
/// in-memory/ring — but the guard is called so the hygiene contract is enforced uniformly).
#[allow(dead_code)]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Fail if a repo-local `test_output/` OR `tests/screenshots/` dir exists — artifacts must go to the
/// EXTERNAL `Handshake_Artifacts/handshake-test` root only (CX-212E). A tracked artifact under `src/`
/// is a hygiene FAILURE.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "no repo-local {} dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            local,
            p.display()
        );
    }
}

/// A unique temp backing-file path for a ring (per test, no collisions between parallel test threads).
fn temp_ring_path(tag: &str) -> PathBuf {
    let unique = format!(
        "handshake-mt082-{}-{}-{:?}.ring",
        tag,
        std::process::id(),
        std::thread::current().id()
    );
    std::env::temp_dir().join(unique)
}

/// A deterministic marker event with a known sequence id, so a read-back can be matched 1:1.
fn marker(seq: u64) -> DiagEvent {
    DiagEvent::generic(
        DiagEventCode::Other,
        DiagPhase::Tick,
        DiagSeverity::Info,
        0,
        seq,
        seq, // counter_a == seq so the ring read-back can be matched
        0,
        0,
        seq,
    )
}

// ── AC-002-1 / PT-002-A: record() writes the ring + the in-process buffer ─────────────────────────

#[test]
fn record_writes_ring_and_buffer() {
    let path = temp_ring_path("roundtrip");
    let _ = std::fs::remove_file(&path);

    let writer = DiagRingWriter::create(&path, DEFAULT_CAPACITY).expect("create ring writer");
    let recorder = DiagnosticsRecorder::with_writer(writer);

    let n: u64 = 50;
    for i in 0..n {
        recorder.record(marker(i));
    }

    // (a) in-process snapshot returns them, newest at the end (oldest-first order).
    let snap = recorder.snapshot_last_n(BUFFER_CAP);
    assert_eq!(snap.len(), n as usize, "all N events buffered in-process");
    assert_eq!(snap.first().unwrap().sequence_id, 0);
    assert_eq!(snap.last().unwrap().sequence_id, n - 1);

    // (b) a SEPARATE reader on the SAME backing file reads them back from the shared-memory ring —
    //     proving record() actually wrote the ring (what Palmistry will map).
    let reader = DiagRingReader::open(&path).expect("open ring reader on the same backing file");
    let back = reader.read_last_n(n as u32);
    assert_eq!(
        back.len(),
        n as usize,
        "the separate ring reader reads back every record record() wrote"
    );
    // read_last_n is newest-first; the most recent is seq n-1.
    assert_eq!(back.first().unwrap().sequence_id, n - 1);
    assert_eq!(back.first().unwrap().counter_a, n - 1);
    assert_eq!(back.last().unwrap().sequence_id, 0);

    drop(reader);
    drop(recorder);
    let _ = std::fs::remove_file(&path);
    assert_no_local_artifact_dir();
}

// ── AC-002-2 / PT-002-B: non-blocking + panic-free + bounded + dropped accounting under stress ────

#[test]
fn record_is_nonblocking_panic_free_under_stress() {
    let path = temp_ring_path("stress");
    let _ = std::fs::remove_file(&path);
    let writer = DiagRingWriter::create(&path, DEFAULT_CAPACITY).expect("create ring writer");
    // Arc so many threads share the ONE recorder (single in-process buffer + single ring writer; the
    // ring writer is single-producer by design, but here we are proving the recorder's buffer side is
    // panic-free + bounded under contention — the canonical "diagnostics from many call sites" case).
    let recorder = Arc::new(DiagnosticsRecorder::with_writer(writer));

    let threads = 8usize;
    let per_thread = 5_000u64;
    let total = threads as u64 * per_thread;

    let handles: Vec<_> = (0..threads)
        .map(|t| {
            let rec = Arc::clone(&recorder);
            std::thread::spawn(move || {
                for i in 0..per_thread {
                    // record() must never panic and never deadlock, even with the buffer long since
                    // full (per_thread * threads >> BUFFER_CAP).
                    rec.record(marker(t as u64 * per_thread + i));
                }
            })
        })
        .collect();
    for h in handles {
        h.join().expect("no thread panicked inside record()");
    }

    // Buffer stayed bounded.
    let snap = recorder.snapshot_last_n(BUFFER_CAP * 4);
    assert!(
        snap.len() <= BUFFER_CAP,
        "buffer must stay bounded at the cap under stress (got {})",
        snap.len()
    );
    // Every write is accounted: survivors-in-buffer + dropped == total writes (no unbounded growth,
    // no silently-lost-without-counting).
    let dropped = recorder.dropped_count();
    assert_eq!(
        snap.len() as u64 + dropped,
        total,
        "writes ({total}) must equal survivors ({}) + dropped ({dropped})",
        snap.len()
    );
    assert!(dropped > 0, "with total >> cap, some events must have been shed");

    drop(recorder);
    let _ = std::fs::remove_file(&path);
    assert_no_local_artifact_dir();
}

// ── AC-002-3: graceful degradation — record() buffers in-process with NO writer ───────────────────

#[test]
fn record_degrades_gracefully_with_no_writer() {
    // No ring writer at all (the headless/test path). record() must still buffer + never panic.
    let recorder = DiagnosticsRecorder::in_process_only();
    for i in 0..10 {
        recorder.record(marker(i));
    }
    let snap = recorder.snapshot_last_n(100);
    assert_eq!(snap.len(), 10, "events buffer in-process even with no writer");
    assert_eq!(snap.first().unwrap().sequence_id, 0);
    assert_eq!(snap.last().unwrap().sequence_id, 9);
    assert_eq!(recorder.dropped_count(), 0);
    assert_no_local_artifact_dir();
}

// ── AC-002-4 / PT-002-C: the LIVE call site — a real HandshakeApp startup records a marker ─────────

#[test]
fn live_startup_marker_is_recorded() {
    use egui_kittest::Harness;
    use handshake_native::app::HandshakeApp;

    // Snapshot the process-global buffer BEFORE we drive the real startup. (Other tests in this binary
    // may or may not have recorded already; we assert the startup ADDS the marker, so we look for the
    // PaneMounted marker specifically rather than just non-emptiness — that makes the proof robust to
    // test ordering AND proves the event came from the production live call site, not this test.)
    let before_has_marker = diagnostics::snapshot_last_n(BUFFER_CAP)
        .iter()
        .any(|e| e.event_code == DiagEventCode::PaneMounted.as_u16());

    // Drive the REAL production constructor `HandshakeApp::new(cc)` via the eframe kittest harness. The
    // closure body is PRODUCTION code; `new` calls `record_startup_marker()` at the end — the live
    // consumer of the open record() API. No test scaffolding records the event.
    let _harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));

    // After the real startup ran, the process-global buffer is non-empty AND carries the PaneMounted
    // startup marker the production code recorded.
    let after = diagnostics::snapshot_last_n(BUFFER_CAP);
    assert!(
        !after.is_empty(),
        "the live HandshakeApp startup recorded at least one DiagEvent into the global buffer"
    );
    let after_has_marker = after
        .iter()
        .any(|e| e.event_code == DiagEventCode::PaneMounted.as_u16());
    assert!(
        after_has_marker,
        "the production startup live call site recorded a PaneMounted marker (before={before_has_marker})"
    );
    assert_no_local_artifact_dir();
}

// ── AC-002-5: the public API exposes NO free-text / blob surface ──────────────────────────────────

#[test]
fn public_api_has_no_free_text_surface() {
    // Source-scan the diagnostics module: the public `record` / `record_with` signatures must accept
    // only DiagEvent / typed enums + integers — NO String/&str/[u8] content parameter. This keeps the
    // typed-allowlist invariant (§5.8.3) structural at the boundary.
    let recorder_src = include_str!("../src/diagnostics/recorder.rs");
    let mod_src = include_str!("../src/diagnostics/mod.rs");

    // Extract the public free-function signatures (the open API surface).
    for (label, src) in [("recorder.rs", recorder_src), ("mod.rs", mod_src)] {
        // The public record/record_with fns must not name a String/&str/[u8] PARAMETER. We check the
        // function signatures of the public surface specifically.
        assert!(
            !public_fn_has_text_param(src, "record"),
            "public `record*` API in {label} must not accept a String/&str/[u8] content parameter"
        );
    }

    // Belt-and-braces: the whole recorder source must not declare a String/Vec<u8> FIELD on the public
    // record path's payload (DiagEvent is the only stored type, and it is the MT-081 typed allowlist).
    // The recorder struct stores `VecDeque<DiagEvent>` only — assert the stored element type is
    // DiagEvent (no String buffer).
    assert!(
        recorder_src.contains("VecDeque<DiagEvent>"),
        "the in-process buffer must store only the typed DiagEvent (no String/blob buffer)"
    );
    assert_no_local_artifact_dir();
}

/// Heuristic source check: does any PUBLIC `record`-family fn signature declare a `String`/`&str`/
/// `[u8]` parameter? Walks each `pub fn record...(` signature up to its closing `)` and scans the
/// parameter list. (A robust-enough scan for the allowlist gate; the real guarantee is the type
/// system — there is no text field on DiagEvent — but the contract asks for a source scan too.)
fn public_fn_has_text_param(src: &str, name_prefix: &str) -> bool {
    let needle = format!("pub fn {name_prefix}");
    let mut idx = 0usize;
    while let Some(rel) = src[idx..].find(&needle) {
        let start = idx + rel;
        // Find the parameter list: from the '(' after the fn name to the matching ')'.
        if let Some(open_rel) = src[start..].find('(') {
            let open = start + open_rel;
            if let Some(close_rel) = src[open..].find(')') {
                let params = &src[open + 1..open + close_rel];
                if params.contains("String")
                    || params.contains("&str")
                    || params.contains("[u8]")
                    || params.contains("Vec<u8>")
                {
                    return true;
                }
            }
        }
        idx = start + needle.len();
    }
    false
}
