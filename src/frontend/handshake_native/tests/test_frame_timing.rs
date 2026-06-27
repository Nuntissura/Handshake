//! WP-KERNEL-012 MT-085 (D2 — internal_diagnostics, Tier 2: per-frame FRAME-TIME tracking +
//! `SlowFrame` flag, §5.8.2/§5.8.4) runtime proofs.
//!
//! Frame-time tracking is the cheap in-process degradation signal BELOW a full freeze: a frame whose
//! WORK time exceeds the slow threshold (~100ms) is a stutter the Diagnostics Panel (MT-087) surfaces
//! and Palmistry (Tier 3) sees on the ring — but it is NOT yet the ~5s freeze Palmistry's heartbeat
//! staleness (MT-091) detects. Each acceptance criterion maps to a REAL runtime proof (no tautologies):
//!
//! - AC-005-1 (`unit_slow_frame_emits_one_event_and_stats_are_correct`): feed the `FrameTimer` a
//!   sequence of synthetic durations incl. one over the threshold; assert (a) last/min/max/p50/p95 are
//!   correct and (b) EXACTLY one `SlowFrame` `DiagEvent` was emitted for the slow frame (typed micros,
//!   no content). This is the deterministic unit-level proof (no app, no ring).
//! - AC-005-2 (`live_slow_frame_flags_and_fast_frames_do_not`): drive the REAL `HandshakeApp` through
//!   egui_kittest with an artificially slow frame (a per-frame work injection past the threshold) and
//!   assert a `SlowFrame` event reaches the process-global in-process buffer FROM THE LIVE FRAME PATH —
//!   and that NORMAL fast frames do NOT flag (the delta over fast-only frames is zero).
//! - AC-005-3 (`idle_keep_alive_does_not_flag`): step the real app idle (no injected work) for many
//!   frames — the MT-084 ~250ms idle keep-alive repaints — and assert NO `SlowFrame` is emitted. The
//!   chosen measurement (WORK time of `self.ui(ctx)`, NOT the inter-frame period) excludes the idle
//!   period by construction (RISK-005-1).
//! - AC-005-4 (`sustained_slow_streak_is_debounced`): feed many consecutive slow frames within one
//!   debounce window and assert the emit count is GATED (one per ~1s window), so a sustained slow
//!   period emits a bounded count rather than one per frame (RISK-005-2).
//! - AC-005-5 (`frame_stats_are_typed_micros`): `frame_stats()` / `FrameStats` is all `u64` micros (no
//!   content); the panel can read it.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui_kittest::Harness;

use handshake_diag_ring::{DiagEvent, DiagEventCode, DiagPhase, DiagSeverity};
use handshake_native::app::{HandshakeApp, HealthDisplayState};
use handshake_native::diagnostics::{
    self, FrameStats, FrameTimer, BUFFER_CAP, SLOW_FRAME_EMIT_DEBOUNCE, SLOW_FRAME_THRESHOLD,
};

// ── artifact hygiene (CX-212E): no repo-local artifact dir may exist ───────────────────────────────

/// The external artifact root for any MT-085 test output. The proofs here are all in-memory / global
/// buffer reads (no screenshot/PNG is written), but the guard is invoked uniformly so the hygiene
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

/// Count the `SlowFrame` events currently visible in the process-global in-process diagnostics buffer
/// (what the Diagnostics Panel reads). The buffer is process-wide and shared across tests in this
/// binary, so the live-path proofs measure a DELTA (after - before), the way the MT-082/MT-084 live
/// tests do, to stay robust to test ordering.
fn global_slow_frame_count() -> usize {
    diagnostics::snapshot_last_n(BUFFER_CAP)
        .iter()
        .filter(|e| e.event_code == DiagEventCode::SlowFrame.as_u16())
        .count()
}

// ── AC-005-1: unit — stats correctness + exactly one SlowFrame for the one slow frame ──────────────

/// Feed the `FrameTimer` a known sequence of durations including ONE above the slow threshold and
/// assert (a) the stats are correct and (b) EXACTLY one `SlowFrame` `DiagEvent` was emitted for the
/// slow frame, with typed micros and no content. Deterministic — uses synthetic durations + a captured
/// emit closure, no app, no ring, no process-global coupling.
#[test]
fn unit_slow_frame_emits_one_event_and_stats_are_correct() {
    let mut timer = FrameTimer::new();
    let now = Instant::now();
    let mut events: Vec<DiagEvent> = Vec::new();

    // 16ms (60fps), 8ms, then a 250ms SLOW frame, then 33ms (30fps). Only the 250ms frame flags.
    let durations_ms = [16u64, 8, 250, 33];
    let mut flagged = Vec::new();
    for ms in durations_ms {
        flagged.push(timer.record_frame(Duration::from_millis(ms), now, |e| events.push(e)));
    }
    assert_eq!(
        flagged,
        vec![false, false, true, false],
        "only the 250ms frame is slow; the 8/16/33ms frames never flag (RISK-005-3)"
    );

    // (b) Exactly one SlowFrame DiagEvent, typed micros, no content.
    assert_eq!(events.len(), 1, "exactly one SlowFrame event for the one slow frame");
    let e = events[0];
    assert_eq!(e.event_code, DiagEventCode::SlowFrame.as_u16(), "event code is SlowFrame");
    assert_eq!(e.phase_marker, DiagPhase::Tick.as_u8(), "phase is Tick");
    assert_eq!(e.severity, DiagSeverity::Warn.as_u8(), "severity is Warn (a stutter, not fatal)");
    assert_eq!(e.metric_micros, 250_000, "frame_micros is the 250ms slow frame in micros");
    assert_eq!(e.counter_a, 3, "frame_index is the 3rd recorded frame (the slow one)");

    // (a) Stats correctness across the whole sequence.
    let s = timer.stats();
    assert_eq!(s.frame_count, 4, "all four frames counted");
    assert_eq!(s.last_micros, 33_000, "last is the final 33ms frame");
    assert_eq!(s.min_micros, 8_000, "min is the 8ms frame");
    assert_eq!(s.max_micros, 250_000, "max is the 250ms slow frame");
    assert_eq!(s.slow_emit_count, 1, "exactly one slow frame emitted");
    // p50/p95 are within the observed range and ordered (nearest-rank over [8,16,33,250]ms in micros).
    assert!(
        s.p50_micros >= 8_000 && s.p50_micros <= 250_000,
        "p50 ({}) within observed range",
        s.p50_micros
    );
    assert!(s.p95_micros >= s.p50_micros, "p95 ({}) >= p50 ({})", s.p95_micros, s.p50_micros);
    // The p95 of a 4-sample set [8,16,33,250]ms (nearest-rank) is the max, 250ms.
    assert_eq!(s.p95_micros, 250_000, "p95 over the 4-sample ring is the 250ms slow frame");

    assert_no_local_artifact_dir();
}

// ── AC-005-2: the LIVE frame path — a real slow frame flags; fast frames do not ────────────────────

/// Drive the REAL production `HandshakeApp` through egui_kittest. First step several NORMAL frames (no
/// injected work) and assert NO new `SlowFrame` appears in the global buffer (fast frames do not flag).
/// Then inject per-frame work PAST the slow threshold so `self.ui(ctx)` genuinely takes >100ms, step a
/// frame, and assert a `SlowFrame` event reached the global buffer FROM THE LIVE FRAME PATH (the
/// production `update` measured the slow work and emitted through the open recorder — not the test).
#[test]
fn live_slow_frame_flags_and_fast_frames_do_not() {
    let mut harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));

    // (1) FAST frames must NOT flag. Step a few normal frames (no injected work) and assert the global
    //     SlowFrame count does not grow across them.
    let fast_before = global_slow_frame_count();
    for _ in 0..3 {
        harness.step();
    }
    let fast_after = global_slow_frame_count();
    assert_eq!(
        fast_after, fast_before,
        "normal fast frames must NOT emit a SlowFrame (before={fast_before}, after={fast_after})"
    );

    // (2) A REAL slow frame MUST flag. Inject work well past the ~100ms threshold so the live
    //     `self.ui(ctx)` measurement genuinely exceeds it, then step ONE frame.
    let slow_micros = u64::try_from(SLOW_FRAME_THRESHOLD.as_micros()).unwrap() + 120_000; // +120ms
    harness.state_mut().set_extra_frame_work_for_test(slow_micros);
    let slow_before = global_slow_frame_count();
    harness.step();
    // Stop injecting so the in-app stats settle and later steps are fast again.
    harness.state_mut().set_extra_frame_work_for_test(0);
    let slow_after = global_slow_frame_count();
    assert!(
        slow_after > slow_before,
        "a REAL slow frame on the live path emitted a SlowFrame into the global buffer \
         (before={slow_before}, after={slow_after})"
    );

    // The in-app stats also recorded the slow frame (its max is at least the injected slow work).
    let stats = harness.state().frame_stats();
    assert!(
        stats.max_micros >= slow_micros,
        "the in-app frame stats recorded the slow frame work (max={} >= injected {})",
        stats.max_micros,
        slow_micros
    );
    assert!(stats.slow_emit_count >= 1, "the in-app tracker emitted at least one slow frame");

    assert_no_local_artifact_dir();
}

// ── AC-005-3: the MT-084 idle keep-alive does NOT produce spurious SlowFrame events ────────────────

/// Step the REAL app idle (NO injected work) for many frames. egui's on-demand repaint + the MT-084
/// ~250ms idle keep-alive (`HEARTBEAT_IDLE_REPAINT_INTERVAL`) mean the inter-frame PERIOD is long, but
/// the WORK time of each `self.ui(ctx)` is small — so the chosen measurement excludes the idle period
/// and NO `SlowFrame` is emitted (RISK-005-1). Built via the no-network headless ctor so no backend
/// race makes a frame slow for an unrelated reason. The proof: zero new SlowFrame events over the run,
/// and the in-app `slow_emit_count` stays zero.
#[test]
fn idle_keep_alive_does_not_flag() {
    let mut harness: Harness<HandshakeApp> = Harness::builder().build_eframe(|cc| {
        HandshakeApp::install_fonts(&cc.egui_ctx);
        // A non-Loading health state + the seeded clean layout: no 100ms health wake, no 600ms layout
        // wake, no live backend — a genuinely idle shell whose only scheduled repaint is the 250ms
        // heartbeat keep-alive (the exact idle cadence AC-005-3 must prove does not flag).
        HandshakeApp::with_health(HealthDisplayState::Error("idle-no-false-flag".to_owned()))
    });

    let before = global_slow_frame_count();
    let emit_before = harness.state().frame_stats().slow_emit_count;
    // Many idle frames — far more than the debounce window would gate, so a per-period mis-measure would
    // surface as multiple spurious SlowFrame events here.
    for _ in 0..30 {
        harness.step();
    }
    let after = global_slow_frame_count();
    let emit_after = harness.state().frame_stats().slow_emit_count;

    assert_eq!(
        after, before,
        "the idle keep-alive must NOT emit any SlowFrame (global buffer before={before}, after={after}); \
         the WORK-time measurement excludes the ~250ms idle repaint period (RISK-005-1 / AC-005-3)"
    );
    assert_eq!(
        emit_after, emit_before,
        "the in-app tracker emitted no slow frame over {} idle frames (emit before={emit_before}, after={emit_after})",
        30
    );
    // The app DID run frames (the heartbeat advanced) — proving these were real idle frames, not a
    // no-op test. The frame counter advanced by at least the idle steps we drove.
    assert!(
        harness.state().frame_counter() >= 30,
        "the idle steps ran real frames (frame_counter={})",
        harness.state().frame_counter()
    );

    assert_no_local_artifact_dir();
}

// ── AC-005-4: a sustained slow streak emits a DEBOUNCED, bounded number of events ──────────────────

/// Feed many consecutive slow frames at the SAME instant (zero elapsed between them) and assert the
/// emit count is GATED to one per debounce window — so a sustained slow period emits a BOUNDED count
/// rather than one event per frame (which would flood the bounded MT-081 ring, RISK-005-2). Then prove
/// the gate is TIME-based, not a permanent one-shot: a slow frame past the window emits again. Uses the
/// `FrameTimer` directly with synthetic instants so the timing is deterministic (no real sleeps).
#[test]
fn sustained_slow_streak_is_debounced() {
    let mut timer = FrameTimer::new();
    let t0 = Instant::now();
    let mut emitted = 0u32;

    // 60 consecutive slow frames within ONE debounce window (all at t0) -> exactly ONE emit.
    for _ in 0..60 {
        timer.record_frame(Duration::from_millis(200), t0, |_| emitted += 1);
    }
    assert_eq!(
        emitted, 1,
        "60 consecutive slow frames within one debounce window emit exactly ONE SlowFrame (RISK-005-2)"
    );
    assert_eq!(
        timer.stats().slow_emit_count,
        1,
        "the in-tracker emit count is gated to one per window"
    );
    // But every frame is still counted in the stats (the stats are free; only the emit is debounced).
    assert_eq!(timer.stats().frame_count, 60, "all 60 slow frames counted in the stats");

    // The gate is time-based: a slow frame past the debounce window emits a SECOND event.
    let t1 = t0 + SLOW_FRAME_EMIT_DEBOUNCE + Duration::from_millis(50);
    timer.record_frame(Duration::from_millis(200), t1, |_| emitted += 1);
    assert_eq!(emitted, 2, "a slow frame past the debounce window emits again (not a permanent one-shot)");
    assert_eq!(timer.stats().slow_emit_count, 2);

    // The emitted count over 61 slow frames is BOUNDED (2), not 61 — the core debounce guarantee.
    assert!(
        timer.stats().slow_emit_count < 61,
        "the emit count ({}) is bounded well below the 61 slow frames seen",
        timer.stats().slow_emit_count
    );

    assert_no_local_artifact_dir();
}

// ── AC-005-5: frame_stats() is typed integer micros only (no content) ──────────────────────────────

/// `FrameStats` / `frame_stats()` exposes ONLY `u64` micros — no `String`, no blob, no content — so the
/// Diagnostics Panel (MT-087) reads it as typed integers. Proven structurally: a default `FrameStats`
/// is all-zero, and after feeding known durations every field is a plain integer in micros matching the
/// fed values. (The typed-allowlist is enforced at compile time by the struct being all `u64`; this
/// asserts the values are the expected micros.)
#[test]
fn frame_stats_are_typed_micros() {
    // Default (no frames) -> all zero.
    assert_eq!(FrameStats::default().frame_count, 0);
    assert_eq!(FrameStats::default().last_micros, 0);

    let mut timer = FrameTimer::new();
    let now = Instant::now();
    for ms in [10u64, 50, 20] {
        timer.record_frame(Duration::from_millis(ms), now, |_| {});
    }
    let s: FrameStats = timer.stats();
    // Every value is a plain integer in micros (10ms -> 10_000 micros, etc.) — no content anywhere.
    assert_eq!(s.frame_count, 3);
    assert_eq!(s.last_micros, 20_000, "last is 20ms in micros");
    assert_eq!(s.min_micros, 10_000, "min is 10ms in micros");
    assert_eq!(s.max_micros, 50_000, "max is 50ms in micros");
    assert_eq!(s.slow_emit_count, 0, "no slow frames in this fast sequence");
    // p50/p95 are micros within range.
    assert!(s.p50_micros >= 10_000 && s.p50_micros <= 50_000);
    assert!(s.p95_micros >= s.p50_micros);

    assert_no_local_artifact_dir();
}
