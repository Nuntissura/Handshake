//! WP-KERNEL-012 MT-084 (D2 — internal_diagnostics, Tier 2: UI-thread heartbeat, §5.8.2) runtime
//! proofs.
//!
//! The heartbeat is THE liveness signal Palmistry (Tier 3) polls for freeze detection (MT-091): a
//! monotonic frame-loop counter + a monotonic timestamp published into the MT-081 ring's dedicated
//! heartbeat slot on EVERY egui frame, from the UI thread. A stalled UI thread stops advancing the
//! counter, and the staleness is observable out-of-process with zero cooperation.
//!
//! Each acceptance criterion maps to a REAL runtime proof (no tautologies, no test-side counter
//! bumping — the heartbeat must come from PRODUCTION code, read back by a SEPARATE ring reader the way
//! Palmistry will read it):
//!
//! - AC-004-1 / PT-004-A is proven by TWO complementary tests, mirroring the established MT-082 split
//!   (live-consumer via the app + ring read-back via a standalone recorder, so the deterministic
//!   ring read-back never races the process-global `OnceLock` that the live app owns):
//!   * `heartbeat_advances_by_n_over_n_frames` — drives the REAL `HandshakeApp` through N `step()`
//!     frames and asserts the in-app `frame_counter()` advanced by exactly N (the LIVE per-frame
//!     UI-thread wire-in). When this test happens to own the process-global ring writer, it ALSO reads
//!     the live ring back through a SEPARATE `DiagRingReader` and asserts the cross-process counter
//!     advanced by N.
//!   * `heartbeat_publishes_to_ring_cross_process` — installs a STANDALONE `DiagnosticsRecorder` over a
//!     real temp ring, calls the production `recorder.heartbeat(..)` N times, and a SEPARATELY
//!     constructed `DiagRingReader` on the SAME backing file observes the counter advance by exactly N
//!     (deterministic, never racing the global). This is the rock-solid cross-process publish proof.
//! - AC-004-2 / PT-004-B (`heartbeat_timestamp_is_monotonic`): the ring heartbeat timestamp_nanos
//!   strictly increases across writes and never goes backward — driven from a process-start `Instant`
//!   monotonic source, immune to wall-clock changes.
//! - AC-004-3 / PT (`heartbeat_write_is_wait_free_and_alloc_free`): a source review of the per-frame
//!   path (comments stripped — code only) asserts the heartbeat call is a single seqlock store (no
//!   record-buffer lock, no `format!`, no heap alloc), corroborated by a perf bound (a tight burst of
//!   heartbeat writes completes well inside a generous wall-clock budget).
//! - AC-004-4 / PT-004-C (`idle_repaint_cadence_is_bounded`): after an idle `step()`, the production
//!   `update` scheduled a repaint at the bounded ~250ms cadence — captured via egui's request-repaint
//!   callback (the EXACT delay, minus the harness's predicted frame time) — strictly between
//!   Palmistry's poll interval (~200–500ms) and the freeze threshold (~5s).
//! - AC-004-5 / PT (`heartbeat_no_op_without_writer`): with NO ring writer installed, the heartbeat
//!   call is a silent no-op and the recorder/app path runs normally (no panic).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui_kittest::Harness;

use handshake_diag_ring::{DiagRingReader, DiagRingWriter, DEFAULT_CAPACITY};
use handshake_native::app::{HandshakeApp, HEARTBEAT_IDLE_REPAINT_INTERVAL};
use handshake_native::diagnostics::DiagnosticsRecorder;

// ── artifact hygiene (CX-212E): no repo-local artifact dir may exist ───────────────────────────────

/// The external artifact root for any MT-084 test output. The proofs here are all in-memory / ring
/// round-trips (no screenshot/PNG is written), but the guard is invoked uniformly so the hygiene
/// contract is enforced and the helper is not dead.
#[allow(dead_code)]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Fail if a repo-local `test_output/` OR `tests/screenshots/` dir exists — artifacts must go to the
/// EXTERNAL `Handshake_Artifacts/handshake-test` root only (CX-212E). A tracked artifact under `src/`
/// is a hygiene FAILURE the reviewer also catches with `git ls-files "src/**/*.png"`.
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

/// A unique temp backing-file path for a standalone ring (no collisions between parallel test threads).
fn temp_ring_path(tag: &str) -> PathBuf {
    let unique = format!(
        "handshake-mt084-{}-{}-{:?}.ring",
        tag,
        std::process::id(),
        std::thread::current().id()
    );
    std::env::temp_dir().join(unique)
}

// ── AC-004-1 / PT-004-A (live wire-in): N frames advance the in-app + cross-process heartbeat ──────

/// Drive the REAL production `HandshakeApp` through N egui frames and prove the heartbeat is bumped
/// every frame on the UI thread. The in-app `frame_counter()` MUST advance by exactly N (deterministic
/// — this is the live per-frame wire-in). When this test owns the process-global ring writer (it is
/// the first app-constructing test to run in this binary), a SEPARATE `DiagRingReader` on the SAME
/// backing file the app created ALSO sees the cross-process counter advance by N — the way Palmistry
/// reads it. The standalone, never-racing cross-process publish proof is
/// `heartbeat_publishes_to_ring_cross_process`.
#[test]
fn heartbeat_advances_by_n_over_n_frames() {
    let mut harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));

    // Optional cross-process arm: read the live ring IF this test owns the global writer this run.
    let ring = harness
        .state()
        .diag_session()
        .and_then(|s| DiagRingReader::open(&s.ring_path).ok().map(|r| (s.ring_path.clone(), r)));

    let counter_before = harness.state().frame_counter();
    assert!(
        counter_before >= 1,
        "the initial settle pass ran at least one update() frame (got {counter_before})"
    );
    let hb_before = ring.as_ref().map(|(_, r)| {
        r.read_heartbeat()
            .expect("a consistent baseline heartbeat read from the live ring")
    });
    if let (Some(hb), c) = (hb_before, counter_before) {
        assert_eq!(
            hb.counter, c,
            "the live ring heartbeat counter matches the in-app frame_counter at baseline"
        );
    }

    let n: u64 = 7;
    let mut prev_counter = hb_before.map(|h| h.counter);
    let mut prev_ts = hb_before.map(|h| h.timestamp_nanos);
    for i in 0..n {
        harness.step();
        if let Some((_, reader)) = ring.as_ref() {
            let hb = reader
                .read_heartbeat()
                .unwrap_or_else(|| panic!("a consistent heartbeat read after frame {i}"));
            if let Some(pc) = prev_counter {
                assert!(
                    hb.counter > pc,
                    "frame {i}: live ring heartbeat counter advanced ({pc} -> {})",
                    hb.counter
                );
            }
            if let Some(pt) = prev_ts {
                assert!(
                    hb.timestamp_nanos >= pt,
                    "frame {i}: live ring heartbeat timestamp is monotonic ({pt} -> {})",
                    hb.timestamp_nanos
                );
            }
            prev_counter = Some(hb.counter);
            prev_ts = Some(hb.timestamp_nanos);
        }
    }

    // The LIVE per-frame wire-in: the in-app counter advanced by EXACTLY N over N frames.
    let counter_after = harness.state().frame_counter();
    assert_eq!(
        counter_after - counter_before,
        n,
        "the in-app frame_counter advanced by exactly N over N step() frames (the live per-frame \
         UI-thread heartbeat bump)"
    );

    if let Some((ring_path, reader)) = ring {
        let hb_after = reader.read_heartbeat().expect("a consistent final live-ring heartbeat read");
        let hb_base = hb_before.expect("baseline heartbeat was read when the ring exists");
        assert_eq!(
            hb_after.counter - hb_base.counter,
            n,
            "the CROSS-PROCESS live ring heartbeat counter advanced by exactly N over N frames (read \
             by a separate DiagRingReader, the way Palmistry reads it)"
        );
        assert!(
            hb_after.timestamp_nanos > hb_base.timestamp_nanos,
            "the monotonic live-ring heartbeat timestamp strictly advanced across the run"
        );
        drop(reader);
        let _ = std::fs::remove_file(&ring_path);
    }

    assert_no_local_artifact_dir();
}

// ── AC-004-1 / PT-004-A (cross-process publish) + AC-004-2 / PT-004-B (monotonic): standalone ring ─

/// Deterministic cross-process publish + monotonic-timestamp proof that never races the process-global
/// `OnceLock`. A STANDALONE `DiagnosticsRecorder::with_writer` over a real temp ring runs the
/// PRODUCTION `recorder.heartbeat(counter, timestamp_nanos)` N times with a MONOTONIC process-start
/// `Instant` clock (exactly the source `HandshakeApp::bump_heartbeat` uses); a SEPARATELY constructed
/// `DiagRingReader` on the SAME backing file observes the counter advance by exactly N and the
/// timestamp strictly increase, never backward. This is what Palmistry sees cross-process.
#[test]
fn heartbeat_publishes_to_ring_cross_process() {
    let path = temp_ring_path("xproc");
    let _ = std::fs::remove_file(&path);

    let writer = DiagRingWriter::create(&path, DEFAULT_CAPACITY).expect("create ring writer");
    let recorder = DiagnosticsRecorder::with_writer(writer);
    // A SEPARATE reader on the SAME backing file — exactly how Palmistry (Tier 3) maps the ring.
    let reader = DiagRingReader::open(&path).expect("open a separate ring reader on the same file");

    // The monotonic source the production bump uses: a process-start Instant, read as elapsed nanos.
    let clock = Instant::now();

    let n: u64 = 16;
    let mut last_counter: Option<u64> = None;
    let mut last_ts: Option<u64> = None;
    for counter in 1..=n {
        let ts = u64::try_from(clock.elapsed().as_nanos()).unwrap_or(u64::MAX);
        recorder.heartbeat(counter, ts);

        let hb = reader
            .read_heartbeat()
            .unwrap_or_else(|| panic!("a consistent heartbeat read after publish #{counter}"));
        // AC-004-1: the cross-process counter equals exactly what we published (advances by 1 each).
        assert_eq!(
            hb.counter, counter,
            "the separate ring reader sees the exact published heartbeat counter (#{counter})"
        );
        if let Some(pc) = last_counter {
            assert_eq!(hb.counter, pc + 1, "the cross-process counter advances by exactly 1 per write");
        }
        // AC-004-2: the timestamp is monotonic — strictly non-decreasing, never backward.
        if let Some(pt) = last_ts {
            assert!(
                hb.timestamp_nanos >= pt,
                "heartbeat timestamp is monotonic, never backward ({pt} -> {})",
                hb.timestamp_nanos
            );
        }
        last_counter = Some(hb.counter);
        last_ts = Some(hb.timestamp_nanos);
    }

    // Over the whole run the counter advanced by exactly N and the monotonic clock strictly advanced.
    let final_hb = reader.read_heartbeat().expect("final consistent heartbeat read");
    assert_eq!(final_hb.counter, n, "cross-process counter advanced by exactly N over N publishes");
    assert!(
        final_hb.timestamp_nanos >= last_ts.unwrap(),
        "final monotonic timestamp did not go backward"
    );

    drop(reader);
    drop(recorder);
    let _ = std::fs::remove_file(&path);
    assert_no_local_artifact_dir();
}

// ── AC-004-2 / PT-004-B: monotonic timestamp never goes backward (explicit, focused) ──────────────

/// Focused monotonicity proof: many production `recorder.heartbeat` writes sourced from a process-start
/// `Instant` produce a strictly non-decreasing timestamp sequence read back from the ring — never a
/// backward step (which a wall clock could produce on a time change; an `Instant` cannot).
#[test]
fn heartbeat_timestamp_is_monotonic() {
    let path = temp_ring_path("monotonic");
    let _ = std::fs::remove_file(&path);
    let writer = DiagRingWriter::create(&path, DEFAULT_CAPACITY).expect("create ring writer");
    let recorder = DiagnosticsRecorder::with_writer(writer);
    let reader = DiagRingReader::open(&path).expect("open ring reader");

    let clock = Instant::now();
    let mut prev: u64 = 0;
    for counter in 1..=500u64 {
        let ts = u64::try_from(clock.elapsed().as_nanos()).unwrap_or(u64::MAX);
        recorder.heartbeat(counter, ts);
        let hb = reader.read_heartbeat().expect("consistent heartbeat read");
        assert!(
            hb.timestamp_nanos >= prev,
            "monotonic timestamp must never go backward (write #{counter}: {prev} -> {})",
            hb.timestamp_nanos
        );
        prev = hb.timestamp_nanos;
    }

    drop(reader);
    drop(recorder);
    let _ = std::fs::remove_file(&path);
    assert_no_local_artifact_dir();
}

// ── AC-004-4 / PT-004-C: the idle repaint cadence is bounded (~250ms), between poll and threshold ──

/// Prove the production `update` requests a repaint at the bounded ~250ms idle cadence so the
/// heartbeat keeps advancing when the app is idle (RISK-004-1). The scheduled delay is captured from
/// egui's request-repaint callback during an idle `step()` — not asserted from a source scan — so this
/// proves the live `request_repaint_after(HEARTBEAT_IDLE_REPAINT_INTERVAL)` call actually fires.
///
/// egui reduces a scheduled delay by the predicted frame time (`delay -= predicted_dt`) before firing
/// the callback, so the OBSERVED delay is `cadence - predicted_dt`. The harness is configured with a
/// tiny `step_dt` (so `predicted_dt` is ~4ms) and the observed delay is asserted to be `cadence` within
/// that tolerance — proving the production cadence, not the harness's default 4 fps (which equals the
/// cadence and would saturate the observed delay to zero).
#[test]
fn idle_repaint_cadence_is_bounded() {
    use std::sync::{Arc, Mutex};

    // The constant itself sits in the required window: within Palmistry's poll window (~200–500ms) so
    // an idle app is fresh on every poll, and far below the ~5s freeze threshold so a real freeze is
    // unambiguous.
    let cadence = HEARTBEAT_IDLE_REPAINT_INTERVAL;
    assert!(
        cadence >= Duration::from_millis(100) && cadence <= Duration::from_millis(500),
        "idle repaint cadence ({cadence:?}) must be within Palmistry's poll window (~200-500ms)"
    );
    assert!(
        cadence < Duration::from_secs(5),
        "idle repaint cadence ({cadence:?}) must be far below the ~5s freeze threshold"
    );

    // Tiny step_dt (~4ms predicted frame time) so the heartbeat's (cadence - predicted_dt) scheduled
    // delay stays clearly non-zero and observable. The harness default (250ms / 4fps) exactly equals
    // the cadence and would saturate the observed delay to zero — a harness artifact, not production.
    let predicted_dt = 1.0f32 / 240.0;
    let mut harness: Harness<HandshakeApp> = Harness::builder()
        .with_step_dt(predicted_dt)
        .build_eframe(|cc| HandshakeApp::new(cc));

    // Capture the SHORTEST non-zero repaint delay egui is asked to schedule during one idle frame.
    let min_delay: Arc<Mutex<Option<Duration>>> = Arc::new(Mutex::new(None));
    {
        let sink = Arc::clone(&min_delay);
        harness.ctx.set_request_repaint_callback(move |info| {
            if info.delay == Duration::ZERO {
                return;
            }
            let mut g = sink.lock().unwrap();
            *g = Some(match *g {
                Some(prev) => prev.min(info.delay),
                None => info.delay,
            });
        });
    }

    // One idle frame (no queued input): `step()` runs the production `update`, whose FIRST action after
    // the heartbeat bump is `request_repaint_after(HEARTBEAT_IDLE_REPAINT_INTERVAL)` -> callback fires.
    harness.step();

    let observed = min_delay
        .lock()
        .unwrap()
        .expect("the idle frame scheduled a non-immediate repaint (the heartbeat cadence)");

    // egui fires the callback with (cadence - predicted_dt). Assert the observed delay equals the
    // cadence within the predicted-frame-time tolerance (plus a small slack for rounding).
    let predicted = Duration::from_secs_f32(predicted_dt);
    let lower = cadence.saturating_sub(predicted + Duration::from_millis(5));
    assert!(
        observed >= lower && observed <= cadence,
        "the production update() scheduled the bounded heartbeat idle cadence (~{cadence:?} minus the \
         ~{predicted:?} predicted frame time) on an idle frame; observed {observed:?} (expected in \
         [{lower:?}, {cadence:?}])"
    );

    assert_no_local_artifact_dir();
}

// ── AC-004-3 / PT: the per-frame heartbeat write is wait-free + allocation-free ────────────────────

/// Source-review corroboration (CODE only — comments stripped) that the per-frame heartbeat path is a
/// single seqlock store with NO record-buffer lock, NO `format!`, and NO heap allocation, plus a perf
/// bound: a tight burst of heartbeat writes through a REAL ring writer completes well within a generous
/// budget (a blocking / allocating path could not). The hard guarantee is structural (the recorder's
/// `heartbeat` touches only `write_heartbeat`, never the `Mutex<VecDeque>` buffer).
#[test]
fn heartbeat_write_is_wait_free_and_alloc_free() {
    // (a) Source review (code only): the recorder's `heartbeat` forwards ONLY to the ring
    //     `write_heartbeat` and must NOT take the buffer lock or allocate/format on the way.
    let recorder_src = strip_line_comments(include_str!("../src/diagnostics/recorder.rs"));
    let hb_fn = extract_fn_body(&recorder_src, "pub fn heartbeat(&self")
        .expect("recorder.rs declares `pub fn heartbeat(&self, ...)`");
    assert!(
        hb_fn.contains("write_heartbeat"),
        "recorder heartbeat() forwards to the wait-free ring write_heartbeat"
    );
    assert!(
        !hb_fn.contains("self.buffer"),
        "recorder heartbeat() must NOT touch the record buffer (no lock contention with record())"
    );
    assert!(
        !hb_fn.contains(".lock("),
        "recorder heartbeat() must NOT take any lock on the frame path"
    );
    assert!(
        !hb_fn.contains("format!") && !hb_fn.contains("to_string") && !hb_fn.contains("Vec::"),
        "recorder heartbeat() must NOT allocate/format on the frame path"
    );

    // The app-side bump must read a MONOTONIC clock (Instant elapsed), not a wall clock.
    let app_src = strip_line_comments(include_str!("../src/app.rs"));
    let bump_fn = extract_fn_body(&app_src, "fn bump_heartbeat(&mut self)")
        .expect("app.rs declares `fn bump_heartbeat(&mut self)`");
    assert!(
        bump_fn.contains("heartbeat_clock.elapsed()"),
        "bump_heartbeat reads the monotonic process-start Instant (heartbeat_clock.elapsed), never a \
         wall clock (RISK-004-2)"
    );
    assert!(
        !bump_fn.contains("SystemTime"),
        "bump_heartbeat must NOT use a wall clock (SystemTime) for the heartbeat timestamp"
    );

    // It is wired at the TOP of update, before self.ui(ctx) (RISK-004-4: a freeze in ui() then shows as
    // a stale heartbeat). Assert the call ordering in the update body (code only).
    let update_fn = extract_fn_body(&app_src, "fn update(&mut self, ctx: &egui::Context")
        .expect("app.rs declares the eframe::App update fn");
    let bump_pos = update_fn
        .find("self.bump_heartbeat()")
        .expect("update() calls self.bump_heartbeat()");
    let ui_pos = update_fn.find("self.ui(ctx)").expect("update() calls self.ui(ctx)");
    assert!(
        bump_pos < ui_pos,
        "the heartbeat bump must run at the TOP of update(), BEFORE self.ui(ctx), so a freeze inside \
         ui() surfaces as a stale heartbeat (RISK-004-4)"
    );

    // (b) Perf corroboration: a burst of real heartbeat writes is fast (no blocking / allocation).
    let path = temp_ring_path("perf");
    let _ = std::fs::remove_file(&path);
    let writer = DiagRingWriter::create(&path, DEFAULT_CAPACITY).expect("create ring writer");
    let recorder = DiagnosticsRecorder::with_writer(writer);

    let burst: u64 = 200_000;
    let start = Instant::now();
    for i in 0..burst {
        recorder.heartbeat(i, i);
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(500),
        "{burst} wait-free heartbeat writes completed in {elapsed:?} (must be < 500ms; a blocking / \
         allocating path would be far slower)"
    );

    drop(recorder);
    let _ = std::fs::remove_file(&path);
    assert_no_local_artifact_dir();
}

// ── AC-004-5 / PT: graceful degradation — heartbeat is a no-op with no writer ──────────────────────

/// With NO ring writer installed, the recorder's `heartbeat` is a silent no-op: it does not panic and
/// does not touch the in-process buffer (the heartbeat is its own header slot, not a record). Proves
/// the headless/test path runs normally even though no Palmistry-visible ring exists (AC-004-5).
#[test]
fn heartbeat_no_op_without_writer() {
    let recorder = DiagnosticsRecorder::in_process_only();
    for i in 0..1_000 {
        recorder.heartbeat(i, i);
    }
    let snap = recorder.snapshot_last_n(handshake_native::diagnostics::BUFFER_CAP);
    assert!(
        snap.is_empty(),
        "heartbeat() must NOT push into the record buffer (it is its own ring header slot); the \
         buffer stayed empty (got {} events)",
        snap.len()
    );
    assert_eq!(recorder.dropped_count(), 0, "no records, so nothing dropped");

    // The free-function global path must also be a no-op (no panic) when called with no writer in this
    // binary's global. (This deliberately initializes the global writer-less; it runs LAST relative to
    // the app tests in practice, but is correct regardless of order — it never asserts on the global.)
    handshake_native::diagnostics::heartbeat(1, 1);

    assert_no_local_artifact_dir();
}

// ── helpers ────────────────────────────────────────────────────────────────────────────────────────

/// Strip `//` line comments (so a source-review scan checks CODE, not explanatory prose that may
/// legitimately mention `format!` etc.). Conservative: it removes from the first `//` not inside a
/// string literal to end-of-line. Good enough for the heartbeat/bump/update bodies, which contain no
/// `//` inside string literals.
fn strip_line_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for line in src.lines() {
        let mut in_str = false;
        let mut prev = '\0';
        let bytes: Vec<char> = line.chars().collect();
        let mut cut = bytes.len();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i];
            if c == '"' && prev != '\\' {
                in_str = !in_str;
            }
            if !in_str && c == '/' && i + 1 < bytes.len() && bytes[i + 1] == '/' {
                cut = i;
                break;
            }
            prev = c;
            i += 1;
        }
        out.extend(bytes[..cut].iter());
        out.push('\n');
    }
    out
}

/// Extract the brace-balanced body text of the first `fn` whose signature starts with `sig_prefix`.
/// Used by the source-review assertions above (a pragmatic scan; the real guarantee is the type system
/// + the structural separation of the heartbeat path from the record buffer).
fn extract_fn_body<'a>(src: &'a str, sig_prefix: &str) -> Option<&'a str> {
    let start = src.find(sig_prefix)?;
    let open_rel = src[start..].find('{')?;
    let open = start + open_rel;
    let bytes = src.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[open..=i]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}
