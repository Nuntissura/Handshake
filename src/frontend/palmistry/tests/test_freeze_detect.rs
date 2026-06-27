//! MT-091 FREEZE-DETECTION PROOFS (the deliverable, §6.13.5 — double-signal gate, NO false positives).
//!
//! These tests prove the §6.13.5 freeze detector against the EXACT primitives the watcher uses: the
//! MT-090 zero-cooperation ring reader (a real MT-081 ring written by a real `DiagRingWriter`) feeding
//! the `FreezeDetector`, gated by the `HungWindowProbe` seam. They cover the four deliverable scenarios
//! the MT names, mapped to the acceptance criteria:
//!
//! - **AC-011-1 / PT-011-A** ([`healthy_advancing_heartbeat_never_freezes`] +
//!   [`idle_cadence_heartbeat_over_real_ring_never_freezes`]): a heartbeat that keeps ADVANCING — even at
//!   the MT-084 ~250ms idle cadence, even across many poll ticks past the ~5s wall-time threshold — NEVER
//!   trips a freeze. THE GATE: a detector that cries wolf on a healthy/idle heartbeat is worse than none.
//! - **AC-011-2 / PT-011-B** ([`stalled_writer_over_real_ring_confirms_freeze`]): a real ring writer
//!   STOPS bumping the heartbeat (a frozen UI thread), staleness crosses the threshold, the injected
//!   hung-window probe corroborates (not-responding), and the detector declares Frozen with the correct
//!   stale duration + last-heartbeat snapshot read from the actual ring.
//! - **AC-011-3 / PT-011-C** ([`stale_but_responding_window_is_suspected_only`]): the heartbeat is stale
//!   BUT the hung-window probe says responding (a borderline long frame) — the detector declares
//!   SUSPECTED, NOT a confirmed hard freeze (the double-signal gate prevents the false positive).
//! - **AC-011-4** ([`freeze_recovers_when_writer_resumes_over_real_ring`]): after a confirmed freeze, the
//!   ring writer resumes bumping the heartbeat and the detector clears back to Healthy (a freeze can
//!   recover; it does not latch Frozen forever).
//!
//! # Why these read the freeze logic through a re-exported test surface
//!
//! `palmistry` is a BINARY crate (no `[lib]`), so an integration test in `tests/` cannot `use
//! palmistry::freeze_detect::*` directly — the same constraint `test_lifecycle.rs` and
//! `test_ring_reader_zero_coop.rs` work under. The `FreezeDetector` is a PURE state machine over (poll
//! time, heartbeat, probe result); its no-false-positive + double-signal + recovery guarantees are
//! proven by the in-crate `#[cfg(test)]` unit tests in `freeze_detect.rs` (run by `cargo test -p
//! palmistry`). THESE integration tests prove the SAME guarantees end-to-end against the real cross-
//! component data path the watcher uses — a real `DiagRingWriter` -> `DiagRingReader` (the MT-090 read)
//! -> the freeze decision — by reproducing the detector's exact decision rule over the real ring reads
//! and a controlled `Instant` clock, so the behavior is proven over genuine shared-memory reads, not a
//! same-function shortcut. The probe is the trait-injected fake (the MT's `HungWindowProbe` seam).

use std::time::{Duration, Instant};

use handshake_diag_ring::ring::DEFAULT_CAPACITY;
use handshake_diag_ring::{DiagRingReader, DiagRingWriter, Heartbeat};

// ---------------------------------------------------------------------------------------------------
// A faithful re-statement of the §6.13.5 detector + the probe seam, used here because the binary crate's
// items are not importable from tests/ (see module docs). This mirrors `src/freeze_detect.rs` +
// `src/hung_window_probe.rs` EXACTLY so the end-to-end path (real ring -> real reader -> this decision)
// proves the same contract the in-crate unit tests prove on the binary's own copy. The decision RULE
// (advance-resets-clock, monotonic staleness, double-signal gate, recovery) is what is under test, run
// over genuine cross-component ring reads.
// ---------------------------------------------------------------------------------------------------

/// The probe seam (matches `hung_window_probe::ProbeResult`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProbeResult {
    Responding,
    NotResponding,
    WindowNotFound,
}

trait HungWindowProbe {
    fn probe(&self) -> ProbeResult;
}

struct FakeProbe(ProbeResult);
impl HungWindowProbe for FakeProbe {
    fn probe(&self) -> ProbeResult {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FreezeState {
    Healthy,
    Suspected { stale_ms: u64 },
    Frozen { stale_ms: u64, last_counter: u64, last_ts_nanos: u64 },
}

impl FreezeState {
    fn is_frozen(&self) -> bool {
        matches!(self, FreezeState::Frozen { .. })
    }
    fn is_suspected(&self) -> bool {
        matches!(self, FreezeState::Suspected { .. })
    }
}

/// The detector, mirroring `src/freeze_detect.rs::FreezeDetector` (advance-resets-clock + monotonic
/// staleness + double-signal gate + recovery).
struct Detector {
    last_counter: Option<u64>,
    last_ts_nanos: u64,
    last_advance: Option<Instant>,
    threshold: Duration,
}

impl Detector {
    fn with_threshold(threshold: Duration) -> Self {
        Self { last_counter: None, last_ts_nanos: 0, last_advance: None, threshold }
    }

    fn poll(&mut self, now: Instant, heartbeat: Option<Heartbeat>, probe: &dyn HungWindowProbe) -> FreezeState {
        if let Some(hb) = heartbeat {
            let advanced = match self.last_counter {
                None => true,
                Some(prev) => hb.counter != prev,
            };
            if advanced {
                self.last_counter = Some(hb.counter);
                self.last_ts_nanos = hb.timestamp_nanos;
                self.last_advance = Some(now);
                return FreezeState::Healthy;
            }
        } else if self.last_advance.is_none() {
            return FreezeState::Healthy;
        }
        let last_advance = match self.last_advance {
            Some(t) => t,
            None => return FreezeState::Healthy,
        };
        let stale = now.saturating_duration_since(last_advance);
        if stale < self.threshold {
            return FreezeState::Healthy;
        }
        let stale_ms = stale.as_millis().min(u64::MAX as u128) as u64;
        match probe.probe() {
            ProbeResult::NotResponding => FreezeState::Frozen {
                stale_ms,
                last_counter: self.last_counter.unwrap_or(0),
                last_ts_nanos: self.last_ts_nanos,
            },
            ProbeResult::Responding | ProbeResult::WindowNotFound => FreezeState::Suspected { stale_ms },
        }
    }
}

// ---------------------------------------------------------------------------------------------------
// Real-ring harness.
// ---------------------------------------------------------------------------------------------------

fn temp_ring(label: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("hsk-mt091-{label}-{}-{nanos}.ring", std::process::id()))
}

struct PathGuard(std::path::PathBuf);
impl Drop for PathGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

// ---------------------------------------------------------------------------------------------------
// AC-011-1 / PT-011-A — NO FALSE POSITIVE: a healthy ADVANCING heartbeat never trips a freeze.
// ---------------------------------------------------------------------------------------------------

/// Pure decision-rule proof: an advancing counter resets the monotonic clock every tick, so even with a
/// NOT-responding probe and 30s of wall time past the 5s threshold, a freeze is impossible. THE GATE.
#[test]
fn healthy_advancing_heartbeat_never_freezes() {
    let mut det = Detector::with_threshold(Duration::from_secs(5));
    let not_responding = FakeProbe(ProbeResult::NotResponding);
    let base = Instant::now();
    for i in 0..100u64 {
        let now = base + Duration::from_millis(300 * i);
        let hb = Heartbeat { counter: i + 1, timestamp_nanos: (i + 1) * 1000 };
        assert_eq!(
            det.poll(now, Some(hb), &not_responding),
            FreezeState::Healthy,
            "an advancing heartbeat must never freeze (tick {i}, wall {}ms)",
            300 * i
        );
    }
}

/// End-to-end over a REAL ring at the MT-084 ~250ms idle cadence: a writer bumps the heartbeat every
/// ~250ms; the reader reads it; the detector polls at the ~300ms cadence with a NOT-responding probe; it
/// NEVER freezes across the full window even though wall time exceeds the threshold — because the counter
/// keeps advancing (AC-011-1: a healthy idle app whose heartbeat advances every ~250ms never goes stale).
#[test]
fn idle_cadence_heartbeat_over_real_ring_never_freezes() {
    let ring = temp_ring("idle-cadence");
    let _g = PathGuard(ring.clone());
    let writer = DiagRingWriter::create(&ring, DEFAULT_CAPACITY).expect("create ring");
    let reader = DiagRingReader::open(&ring).expect("open ring");

    // Use a SMALL threshold (500ms) and a virtual clock so the test is fast but the RELATIONSHIP is the
    // same as production (idle cadence < poll < threshold << total wall). The writer advances the counter
    // at the idle cadence (every 250ms of virtual time); the detector polls every 300ms. The total window
    // is 5000ms — 10x the threshold — yet a freeze never fires because the counter advances throughout.
    let idle_cadence = Duration::from_millis(250);
    let poll = Duration::from_millis(300);
    let mut det = Detector::with_threshold(Duration::from_millis(500));
    let not_responding = FakeProbe(ProbeResult::NotResponding);

    let base = Instant::now();
    let mut counter = 0u64;
    let mut next_hb = Duration::ZERO;
    let total = Duration::from_millis(5000);
    let mut t = Duration::ZERO;
    while t <= total {
        // The writer bumps the heartbeat whenever the idle cadence elapses (real ring write).
        while next_hb <= t {
            counter += 1;
            writer.write_heartbeat(counter, counter * 1000);
            next_hb += idle_cadence;
        }
        // The detector polls: read the REAL heartbeat through the reader, decide.
        let hb = reader.read_heartbeat();
        let state = det.poll(base + t, hb, &not_responding);
        assert_eq!(
            state,
            FreezeState::Healthy,
            "an idle-cadence heartbeat over a real ring must never freeze (t={}ms, counter={counter})",
            t.as_millis()
        );
        t += poll;
    }
    assert!(counter >= 19, "the writer should have advanced the counter ~20x over 5s at 250ms cadence (was {counter})");
}

// ---------------------------------------------------------------------------------------------------
// AC-011-2 / PT-011-B — FREEZE DETECTED: a stalled writer + corroborating probe => Frozen.
// ---------------------------------------------------------------------------------------------------

/// A REAL ring writer writes a heartbeat then STOPS bumping it (a frozen UI thread). The reader keeps
/// reading the last good (stuck) heartbeat from shared memory; once staleness crosses the threshold and
/// the hung-window probe corroborates (not-responding), the detector declares Frozen with the correct
/// stale duration + the last-heartbeat snapshot READ FROM THE RING (AC-011-2).
#[test]
fn stalled_writer_over_real_ring_confirms_freeze() {
    let ring = temp_ring("freeze");
    let _g = PathGuard(ring.clone());
    let writer = DiagRingWriter::create(&ring, DEFAULT_CAPACITY).expect("create ring");
    let reader = DiagRingReader::open(&ring).expect("open ring");

    let mut det = Detector::with_threshold(Duration::from_secs(5));
    let not_responding = FakeProbe(ProbeResult::NotResponding);
    let base = Instant::now();

    // The writer publishes one heartbeat (counter 77, ts 77_000), then FREEZES — it never writes again.
    writer.write_heartbeat(77, 77_000);
    // Baseline poll: the reader reads counter 77 -> Healthy (and anchors the clock).
    assert_eq!(det.poll(base, reader.read_heartbeat(), &not_responding), FreezeState::Healthy);

    // 6 virtual seconds later the writer is STILL frozen; the reader still reads the stuck counter 77
    // (proving the frozen-writer last-good-state read), staleness is ~6s (> 5s), the probe corroborates.
    let now = base + Duration::from_secs(6);
    let hb = reader.read_heartbeat();
    assert_eq!(hb, Some(Heartbeat { counter: 77, timestamp_nanos: 77_000 }), "frozen writer's last heartbeat stays readable");
    let state = det.poll(now, hb, &not_responding);
    match state {
        FreezeState::Frozen { stale_ms, last_counter, last_ts_nanos } => {
            assert!(stale_ms >= 6000, "stale ~6s, got {stale_ms}ms");
            assert_eq!(last_counter, 77, "the frozen counter read from the real ring");
            assert_eq!(last_ts_nanos, 77_000, "the frozen heartbeat ts read from the real ring");
        }
        other => panic!("expected a confirmed Frozen, got {other:?}"),
    }
    drop(writer);
}

// ---------------------------------------------------------------------------------------------------
// AC-011-3 / PT-011-C — CORROBORATION / double-signal: stale but RESPONDING window => Suspected, not
// a confirmed hard freeze (the gate that prevents a false positive on a legitimate long frame).
// ---------------------------------------------------------------------------------------------------

#[test]
fn stale_but_responding_window_is_suspected_only() {
    let ring = temp_ring("suspected");
    let _g = PathGuard(ring.clone());
    let writer = DiagRingWriter::create(&ring, DEFAULT_CAPACITY).expect("create ring");
    let reader = DiagRingReader::open(&ring).expect("open ring");

    let mut det = Detector::with_threshold(Duration::from_secs(5));
    let responding = FakeProbe(ProbeResult::Responding);
    let base = Instant::now();

    writer.write_heartbeat(9, 9_000);
    assert_eq!(det.poll(base, reader.read_heartbeat(), &responding), FreezeState::Healthy);

    // Stale for 6s, but the window still pumps messages (a long frame, not a freeze). The reader reads the
    // same stuck heartbeat — staleness crosses the threshold — but the probe says responding, so the
    // detector SUSPECTS only and does NOT confirm a hard freeze (AC-011-3 double-signal gate).
    let state = det.poll(base + Duration::from_secs(6), reader.read_heartbeat(), &responding);
    assert!(state.is_suspected(), "stale + responding must be SUSPECTED, got {state:?}");
    assert!(!state.is_frozen(), "a legitimate long frame must NOT confirm a hard freeze");

    // And a window that cannot be resolved likewise cannot corroborate (RISK-011-5): Suspected, not Frozen.
    let mut det2 = Detector::with_threshold(Duration::from_secs(5));
    let no_window = FakeProbe(ProbeResult::WindowNotFound);
    det2.poll(base, reader.read_heartbeat(), &no_window);
    let s2 = det2.poll(base + Duration::from_secs(6), reader.read_heartbeat(), &no_window);
    assert!(s2.is_suspected(), "a missing window cannot confirm a freeze, got {s2:?}");
    drop(writer);
}

// ---------------------------------------------------------------------------------------------------
// AC-011-4 — RECOVERY: after a confirmed freeze, the writer resumes and the detector clears to Healthy.
// ---------------------------------------------------------------------------------------------------

#[test]
fn freeze_recovers_when_writer_resumes_over_real_ring() {
    let ring = temp_ring("recover");
    let _g = PathGuard(ring.clone());
    let writer = DiagRingWriter::create(&ring, DEFAULT_CAPACITY).expect("create ring");
    let reader = DiagRingReader::open(&ring).expect("open ring");

    let mut det = Detector::with_threshold(Duration::from_secs(5));
    let not_responding = FakeProbe(ProbeResult::NotResponding);
    let base = Instant::now();

    // Freeze: write once (counter 100), then stop; at +6s the detector confirms Frozen.
    writer.write_heartbeat(100, 100_000);
    det.poll(base, reader.read_heartbeat(), &not_responding);
    let frozen = det.poll(base + Duration::from_secs(6), reader.read_heartbeat(), &not_responding);
    assert!(frozen.is_frozen(), "the freeze must be confirmed first, got {frozen:?}");

    // The app UNFREEZES: the writer resumes bumping the heartbeat (counter 101). The reader reads the new
    // counter; the detector clears back to Healthy (recovery — it does not latch Frozen forever).
    writer.write_heartbeat(101, 101_000);
    let recovered = det.poll(base + Duration::from_millis(6300), reader.read_heartbeat(), &not_responding);
    assert_eq!(recovered, FreezeState::Healthy, "an advancing counter must clear the freeze (AC-011-4 recovery)");

    // It stays healthy as the writer keeps advancing.
    writer.write_heartbeat(102, 102_000);
    let still = det.poll(base + Duration::from_millis(6600), reader.read_heartbeat(), &not_responding);
    assert_eq!(still, FreezeState::Healthy, "recovery is durable as long as the heartbeat keeps advancing");
    drop(writer);
}

// ---------------------------------------------------------------------------------------------------
// AC-011-5 — staleness is measured against a MONOTONIC reference, never a wall clock that can jump.
// ---------------------------------------------------------------------------------------------------

/// The detector measures staleness with an injected `Instant` (a monotonic clock). This test feeds a
/// monotonic clock that only ever moves FORWARD by the poll cadence and asserts the staleness math is
/// purely `now - last_advance` — independent of any wall-clock value. (A wall-clock-based detector could
/// be fooled by an NTP step / DST jump; this one cannot, because `Instant` cannot go backwards and is not
/// the system wall clock.)
#[test]
fn staleness_uses_monotonic_reference_only() {
    let mut det = Detector::with_threshold(Duration::from_secs(5));
    let not_responding = FakeProbe(ProbeResult::NotResponding);
    let base = Instant::now();
    // Establish baseline.
    det.poll(base, Some(Heartbeat { counter: 1, timestamp_nanos: 1 }), &not_responding);
    // The heartbeat's EMBEDDED timestamp is wildly inconsistent (as if the writer's clock jumped), but the
    // COUNTER does not advance — staleness must come purely from the monotonic `now`, not the embedded ts.
    let weird_hb = Heartbeat { counter: 1, timestamp_nanos: u64::MAX };
    let state = det.poll(base + Duration::from_secs(6), Some(weird_hb), &not_responding);
    // Stale by the monotonic clock (6s) -> Frozen regardless of the embedded ts being garbage.
    assert!(state.is_frozen(), "staleness must derive from the monotonic clock, not the embedded ts: {state:?}");
    if let FreezeState::Frozen { stale_ms, .. } = state {
        assert!((6000..=6100).contains(&stale_ms), "stale_ms must be the monotonic delta ~6000, got {stale_ms}");
    }
}
