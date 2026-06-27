//! Palmistry FREEZE DETECTION (MT-091, §6.13.5 — the double-signal gate, NO false positives).
//!
//! Master Spec v02.196 §6.13.5: poll the ring heartbeat; when it goes STALE (the heartbeat counter /
//! monotonic timestamp stops advancing for longer than a threshold) AND an OS hung-window probe
//! corroborates, declare a FREEZE. The double-signal — *"Staleness alone is corroborated by an OS
//! hung-window probe ... before Palmistry declares a freeze"* — is what avoids crying wolf on a
//! legitimate long frame. A healthy, advancing heartbeat must NEVER trip a freeze (AC-011-1, the gate:
//! a detector that fires on a healthy heartbeat is worse than none).
//!
//! # The monotonic staleness reference (AC-011-5, RISK-011-2 — never a wall clock)
//!
//! Staleness is measured against PALMISTRY'S OWN MONOTONIC CLOCK ([`std::time::Instant`]), NOT a wall
//! clock that can jump (NTP step, DST, suspend/resume). On every poll tick the detector reads the
//! heartbeat counter via the MT-090 reader; whenever the counter CHANGES from the last-seen value it
//! records "advanced at `now`" (`last_advance`). Staleness is then `now - last_advance`. Reset-on-
//! advance is the invariant: as long as the counter keeps moving, `last_advance` keeps moving with it
//! and staleness never grows — so a healthy/idle app (whose heartbeat advances every ~250ms, MT-084)
//! can never cross the ~5s threshold. We deliberately key off the COUNTER, not the heartbeat's embedded
//! monotonic timestamp, for the staleness clock, because the counter advancing is the liveness signal
//! and Palmistry's own `Instant` is the authoritative non-jumping reference for elapsed wall-ish time;
//! the heartbeat's own monotonic nanos are carried in the [`FreezeReport`] for correlation.
//!
//! # The cadence relationship (AC-011-1 / AC-011-5 — provably consistent)
//!
//! Three time constants stack, and their ordering is what makes a false positive impossible on a
//! healthy heartbeat:
//!
//! ```text
//!   MT-084 idle heartbeat cadence : ~250ms   (the writer bumps the counter at least this often, even
//!                                              when idle, via egui request_repaint_after)
//!   this poll cadence (POLL)      : ~300ms   (between 200-500ms; how often THIS detector samples)
//!   freeze threshold (THRESHOLD)  : ~5000ms  (>> both of the above)
//! ```
//!
//! Because `THRESHOLD (5000ms) >> idle cadence (250ms)`, a healthy idle app advances its counter roughly
//! 20 times within one threshold window, so `last_advance` is continuously refreshed and staleness never
//! approaches the threshold. The poll cadence being slightly ABOVE the idle cadence only affects how
//! quickly the detector NOTICES an advance; it does not affect the staleness math, which is anchored to
//! the monotonic time of the last observed advance. See [`POLL_INTERVAL`], [`FREEZE_THRESHOLD`], and
//! [`MT084_IDLE_HEARTBEAT_CADENCE`] and the compile-time consistency assertions below.
//!
//! # The double-signal state machine
//!
//! ```text
//!   Healthy   : counter advancing (or stale < THRESHOLD).
//!   Suspected : counter stale >= THRESHOLD, but the hung-window probe says RESPONDING (or could not be
//!               resolved). A legitimate long frame looks like this — staleness alone is NOT a confirmed
//!               freeze (§6.13.5 double-signal gate, AC-011-3).
//!   Frozen    : counter stale >= THRESHOLD AND the hung-window probe says NOT RESPONDING. The confirmed
//!               freeze (AC-011-2).
//! ```
//!
//! A freeze can RECOVER: when the counter advances again, the detector clears back to `Healthy`
//! (AC-011-4) — it does NOT latch `Frozen` forever (RISK-011-3).

use std::time::{Duration, Instant};

use handshake_diag_ring::Heartbeat;

use crate::hung_window_probe::{HungWindowProbe, ProbeResult};

/// The MT-084 idle heartbeat cadence: the Handshake UI thread bumps the heartbeat counter at LEAST this
/// often even when idle (via egui `request_repaint_after`). Kept here as a named const so the freeze
/// threshold's relationship to it is machine-checkable (the compile-time asserts below), satisfying the
/// AC-011-5 "documented relationship" requirement in code rather than prose alone.
pub const MT084_IDLE_HEARTBEAT_CADENCE: Duration = Duration::from_millis(250);

/// How often the freeze detector samples the heartbeat. Chosen in the §6.13.5 ~200-500ms band, slightly
/// ABOVE the MT-084 idle cadence (so it does not over-sample) and FAR below the freeze threshold. The
/// poll cadence governs only detection LATENCY, not the staleness math (which is anchored to the
/// monotonic time of the last observed counter advance), so a healthy idle app is never flagged.
pub const POLL_INTERVAL: Duration = Duration::from_millis(300);

/// The staleness threshold: the heartbeat counter must be stale (not advanced) for at least this long
/// before the detector even SUSPECTS a freeze. The default ~5s is >> the MT-084 idle cadence (~250ms),
/// so a healthy idle app — which advances its counter ~20x within this window — can never cross it
/// (AC-011-1). Long enough that a genuine multi-second long frame is not instantly flagged either; the
/// hung-window probe is the second gate (§6.13.5).
pub const FREEZE_THRESHOLD: Duration = Duration::from_millis(5000);

// --- Compile-time consistency of the cadence relationship (AC-011-5) --------------------------------
// These asserts make the §6.13.5 ordering a MACHINE-CHECKED invariant, not just a comment: if a future
// edit set the threshold below the idle cadence (which would let a healthy heartbeat trip a freeze) or
// pushed the poll interval outside the ~200-500ms band, the crate would FAIL TO COMPILE.
const _: () = assert!(
    FREEZE_THRESHOLD.as_millis() > MT084_IDLE_HEARTBEAT_CADENCE.as_millis() * 10,
    "freeze threshold must be far above the idle heartbeat cadence so a healthy idle app never goes stale"
);
const _: () = assert!(
    POLL_INTERVAL.as_millis() >= 200 && POLL_INTERVAL.as_millis() <= 500,
    "poll cadence must sit in the §6.13.5 ~200-500ms band"
);
const _: () = assert!(
    POLL_INTERVAL.as_millis() < FREEZE_THRESHOLD.as_millis(),
    "poll cadence must be far below the freeze threshold"
);

/// The freeze verdict the detector publishes. A small closed enum so the watch loop + survivor store
/// reason over a typed state, never a bare bool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreezeState {
    /// The heartbeat is advancing (or has been stale for less than [`FREEZE_THRESHOLD`]). All is well.
    Healthy,
    /// The heartbeat has been stale for >= [`FREEZE_THRESHOLD`], but the hung-window probe says the
    /// window is RESPONDING (or no window could be resolved). This is the §6.13.5 single-signal state: a
    /// legitimate long frame looks exactly like this, so it is SUSPECTED, NOT a confirmed hard freeze
    /// (AC-011-3). Carries the stale duration in milliseconds for visibility.
    Suspected {
        /// How long the heartbeat counter has been stale, in milliseconds.
        stale_ms: u64,
    },
    /// CONFIRMED FREEZE (AC-011-2): the heartbeat has been stale for >= [`FREEZE_THRESHOLD`] AND the
    /// hung-window probe corroborates with NOT RESPONDING. Carries the freeze evidence.
    Frozen(FreezeReport),
}

impl FreezeState {
    /// Whether this state is the confirmed hard freeze. Convenience for the watch loop / tests.
    #[inline]
    pub fn is_frozen(&self) -> bool {
        matches!(self, FreezeState::Frozen(_))
    }

    /// Whether this state is the suspected (single-signal) state.
    #[inline]
    pub fn is_suspected(&self) -> bool {
        matches!(self, FreezeState::Suspected { .. })
    }
}

/// The typed evidence captured when a freeze is CONFIRMED. All fields are integers (the substrate's
/// typed-allowlist stance — no project content): the stale duration, and a snapshot of the last
/// heartbeat the writer published before it froze. MT-093 (the survivor store) persists this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreezeReport {
    /// How long the heartbeat counter had been stale when the freeze was confirmed, in milliseconds
    /// (measured against Palmistry's monotonic clock, AC-011-5).
    pub stale_ms: u64,
    /// The last heartbeat COUNTER the writer published before it froze (the value that stopped
    /// advancing).
    pub last_heartbeat_counter: u64,
    /// The last heartbeat's embedded monotonic timestamp in nanoseconds (the writer's own clock at the
    /// last advance), carried for correlation with the ring's monotonic timeline.
    pub last_heartbeat_ts_nanos: u64,
}

/// The freeze detector (§6.13.5). It is a PURE state machine over (poll time, current heartbeat,
/// hung-window probe result): the watch loop calls [`poll`](FreezeDetector::poll) every
/// [`POLL_INTERVAL`] with the current monotonic `now`, the heartbeat read from the MT-090 reader, and
/// the probe; the detector tracks counter advances against its monotonic clock and returns the typed
/// [`FreezeState`]. Keeping the time + heartbeat + probe INJECTED (rather than read inside) is what makes
/// the no-false-positive proof (AC-011-1) and the double-signal proof (AC-011-3) deterministic — a test
/// drives a virtual clock + a fake probe and asserts the verdict, with no sleeps or real Win32.
pub struct FreezeDetector {
    /// The last heartbeat COUNTER value observed. `None` until the first heartbeat is seen. A CHANGE in
    /// this value is what counts as an "advance" and resets the staleness clock.
    last_counter: Option<u64>,
    /// The last heartbeat's monotonic timestamp nanos, snapshotted alongside `last_counter` so the
    /// confirmed-freeze report can carry the writer's own last clock value.
    last_ts_nanos: u64,
    /// Palmistry's MONOTONIC time at which the counter was last observed to ADVANCE (or first seen). The
    /// staleness reference (AC-011-5): staleness = `now - last_advance`. Never a wall clock.
    last_advance: Option<Instant>,
    /// The threshold beyond which staleness is considered for a freeze. Configurable (tests use a tiny
    /// threshold) but defaults to [`FREEZE_THRESHOLD`].
    threshold: Duration,
}

impl Default for FreezeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl FreezeDetector {
    /// A fresh detector with the production [`FREEZE_THRESHOLD`]. No heartbeat seen yet.
    pub fn new() -> Self {
        Self::with_threshold(FREEZE_THRESHOLD)
    }

    /// A detector with an explicit staleness `threshold`. Tests use a small threshold to drive the
    /// stale/freeze paths quickly without waiting real seconds; production uses [`FREEZE_THRESHOLD`].
    pub fn with_threshold(threshold: Duration) -> Self {
        Self {
            last_counter: None,
            last_ts_nanos: 0,
            last_advance: None,
            threshold,
        }
    }

    /// Poll the detector with the current monotonic `now`, the `heartbeat` read from the MT-090 reader
    /// (`None` = no heartbeat readable this tick), and a hung-window `probe`. Returns the typed
    /// [`FreezeState`].
    ///
    /// The logic, in order:
    /// 1. **Advance tracking (reset-on-advance, AC-011-5).** If the heartbeat counter CHANGED from the
    ///    last-seen value (or this is the first heartbeat), record `last_advance = now` and return
    ///    `Healthy`. This is the no-false-positive guarantee: an advancing counter ALWAYS resets the
    ///    clock, so a healthy/idle heartbeat can never accumulate staleness (AC-011-1) — and it also
    ///    RECOVERS a prior freeze (AC-011-4), because an advance unconditionally returns to `Healthy`.
    /// 2. **Staleness (monotonic).** If the counter did NOT advance, compute `stale = now - last_advance`
    ///    against Palmistry's monotonic clock. If `stale < threshold`, still `Healthy` (a short stall /
    ///    one slow frame is not yet suspicious).
    /// 3. **Double-signal gate (§6.13.5).** If `stale >= threshold`, run the hung-window `probe`:
    ///    - probe `NotResponding` => CONFIRMED [`FreezeState::Frozen`] (both signals agree, AC-011-2).
    ///    - probe `Responding` or `WindowNotFound` => [`FreezeState::Suspected`] ONLY (staleness alone is
    ///      not a hard freeze — a legitimate long frame, AC-011-3 — and a missing window cannot
    ///      corroborate, RISK-011-5).
    pub fn poll(
        &mut self,
        now: Instant,
        heartbeat: Option<Heartbeat>,
        probe: &dyn HungWindowProbe,
    ) -> FreezeState {
        // 1) ADVANCE TRACKING. A heartbeat whose counter changed (or the first heartbeat ever) resets the
        // staleness clock and is unconditionally Healthy — the recovery + no-false-positive guarantee.
        if let Some(hb) = heartbeat {
            let advanced = match self.last_counter {
                None => true,                 // first heartbeat ever seen.
                Some(prev) => hb.counter != prev, // any change = an advance (covers wrap; equality = stall).
            };
            if advanced {
                self.last_counter = Some(hb.counter);
                self.last_ts_nanos = hb.timestamp_nanos;
                self.last_advance = Some(now);
                return FreezeState::Healthy;
            }
            // Same counter as last time => stalled; fall through to the staleness check.
        } else if self.last_advance.is_none() {
            // No heartbeat has EVER been read (Handshake may not have published one yet). With no
            // baseline there is no staleness to measure — treat as Healthy (the MT-091 stall policy: a
            // never-started heartbeat is not a freeze of a running app). We do NOT anchor `last_advance`
            // here, so the clock only starts once a real heartbeat is seen.
            return FreezeState::Healthy;
        }
        // If `heartbeat` is None but we HAVE seen one before, the writer has stopped publishing a
        // readable heartbeat — that is itself staleness; keep `last_advance` anchored at the last advance
        // and let the staleness check below decide.

        // 2) STALENESS against the MONOTONIC clock (AC-011-5). `last_advance` is always Some here (we
        // returned early above if it was None and no prior heartbeat existed).
        let last_advance = match self.last_advance {
            Some(t) => t,
            None => return FreezeState::Healthy,
        };
        let stale = now.saturating_duration_since(last_advance);
        if stale < self.threshold {
            return FreezeState::Healthy;
        }

        // 3) DOUBLE-SIGNAL GATE (§6.13.5). Staleness has crossed the threshold; corroborate with the OS
        // hung-window probe before declaring a HARD freeze.
        let stale_ms = stale.as_millis().min(u64::MAX as u128) as u64;
        match probe.probe() {
            ProbeResult::NotResponding => {
                // BOTH signals agree => CONFIRMED freeze (AC-011-2).
                FreezeState::Frozen(FreezeReport {
                    stale_ms,
                    last_heartbeat_counter: self.last_counter.unwrap_or(0),
                    last_heartbeat_ts_nanos: self.last_ts_nanos,
                })
            }
            ProbeResult::Responding | ProbeResult::WindowNotFound => {
                // Stale BUT the window still pumps messages (or no window to corroborate) => a legitimate
                // long frame, NOT a confirmed freeze. SUSPECTED only (AC-011-3 / RISK-011-5).
                FreezeState::Suspected { stale_ms }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hung_window_probe::FakeHungWindowProbe;

    fn hb(counter: u64) -> Heartbeat {
        Heartbeat {
            counter,
            timestamp_nanos: counter.wrapping_mul(1000),
        }
    }

    /// A healthy ADVANCING heartbeat never trips a freeze, even across many ticks past the threshold in
    /// wall time — because each advance resets the monotonic clock (AC-011-1, the in-crate fast proof of
    /// the gate; the integration test repeats it at the ~250ms idle cadence).
    #[test]
    fn advancing_heartbeat_never_freezes() {
        let mut det = FreezeDetector::with_threshold(Duration::from_secs(5));
        let not_responding = FakeHungWindowProbe::new(ProbeResult::NotResponding);
        let base = Instant::now();
        // 100 ticks, 300ms apart = 30s of wall time, FAR past the 5s threshold — but the counter advances
        // every tick, so staleness is always ~0 and a freeze is impossible even with a NotResponding probe.
        for i in 0..100u64 {
            let now = base + Duration::from_millis(300 * i);
            let state = det.poll(now, Some(hb(i + 1)), &not_responding);
            assert_eq!(state, FreezeState::Healthy, "an advancing heartbeat must never freeze (tick {i})");
        }
    }

    /// Stale counter + NotResponding probe => CONFIRMED Frozen with the correct stale duration + last
    /// heartbeat snapshot (AC-011-2).
    #[test]
    fn stale_plus_not_responding_confirms_freeze() {
        let mut det = FreezeDetector::with_threshold(Duration::from_secs(5));
        let not_responding = FakeHungWindowProbe::new(ProbeResult::NotResponding);
        let base = Instant::now();
        // Establish a baseline heartbeat (counter 42).
        assert_eq!(det.poll(base, Some(hb(42)), &not_responding), FreezeState::Healthy);
        // Writer freezes: counter stuck at 42. 6s later (> 5s threshold) the probe corroborates.
        let now = base + Duration::from_secs(6);
        let state = det.poll(now, Some(hb(42)), &not_responding);
        match state {
            FreezeState::Frozen(report) => {
                assert!(report.stale_ms >= 6000, "stale duration ~6s, got {}", report.stale_ms);
                assert_eq!(report.last_heartbeat_counter, 42);
                assert_eq!(report.last_heartbeat_ts_nanos, 42_000);
            }
            other => panic!("expected Frozen, got {other:?}"),
        }
    }

    /// Stale counter BUT Responding probe => SUSPECTED only, not a confirmed freeze (AC-011-3, the
    /// double-signal gate that prevents false positives on a legitimate long frame).
    #[test]
    fn stale_but_responding_is_suspected_not_frozen() {
        let mut det = FreezeDetector::with_threshold(Duration::from_secs(5));
        let responding = FakeHungWindowProbe::new(ProbeResult::Responding);
        let base = Instant::now();
        det.poll(base, Some(hb(7)), &responding);
        let now = base + Duration::from_secs(6);
        let state = det.poll(now, Some(hb(7)), &responding);
        assert!(state.is_suspected(), "stale + responding window must be SUSPECTED, got {state:?}");
        assert!(!state.is_frozen(), "a legitimate long frame must NOT confirm a hard freeze");
    }

    /// A missing window cannot corroborate, so even a stale counter is only SUSPECTED, never Frozen
    /// (RISK-011-5).
    #[test]
    fn stale_but_no_window_is_suspected_not_frozen() {
        let mut det = FreezeDetector::with_threshold(Duration::from_secs(5));
        let no_window = FakeHungWindowProbe::new(ProbeResult::WindowNotFound);
        let base = Instant::now();
        det.poll(base, Some(hb(1)), &no_window);
        let state = det.poll(base + Duration::from_secs(6), Some(hb(1)), &no_window);
        assert!(state.is_suspected(), "a missing window cannot confirm a freeze, got {state:?}");
    }

    /// After a confirmed freeze, the heartbeat resuming clears back to Healthy (AC-011-4 — a freeze can
    /// recover; the detector must not latch Frozen forever, RISK-011-3).
    #[test]
    fn freeze_recovers_when_heartbeat_resumes() {
        let mut det = FreezeDetector::with_threshold(Duration::from_secs(5));
        let not_responding = FakeHungWindowProbe::new(ProbeResult::NotResponding);
        let base = Instant::now();
        det.poll(base, Some(hb(10)), &not_responding);
        // Freeze confirmed at +6s (counter still 10).
        let frozen = det.poll(base + Duration::from_secs(6), Some(hb(10)), &not_responding);
        assert!(frozen.is_frozen(), "freeze must be confirmed first");
        // The app unfreezes: the counter advances again at +6.3s.
        let recovered = det.poll(base + Duration::from_millis(6300), Some(hb(11)), &not_responding);
        assert_eq!(recovered, FreezeState::Healthy, "an advancing counter must clear the freeze (recovery)");
        // And it stays healthy as the counter keeps advancing.
        let still = det.poll(base + Duration::from_millis(6600), Some(hb(12)), &not_responding);
        assert_eq!(still, FreezeState::Healthy);
    }

    /// A short stall BELOW the threshold is still Healthy (one slow frame is not a freeze).
    #[test]
    fn short_stall_below_threshold_is_healthy() {
        let mut det = FreezeDetector::with_threshold(Duration::from_secs(5));
        let not_responding = FakeHungWindowProbe::new(ProbeResult::NotResponding);
        let base = Instant::now();
        det.poll(base, Some(hb(3)), &not_responding);
        // 2s stall < 5s threshold: not yet suspicious even with a not-responding probe.
        let state = det.poll(base + Duration::from_secs(2), Some(hb(3)), &not_responding);
        assert_eq!(state, FreezeState::Healthy, "a sub-threshold stall must stay Healthy");
    }

    /// A heartbeat that becomes UNREADABLE (None) after being seen is treated as staleness (the writer
    /// stopped publishing), and with corroboration confirms a freeze — the `None`-after-seen path.
    #[test]
    fn unreadable_heartbeat_after_seen_can_freeze() {
        let mut det = FreezeDetector::with_threshold(Duration::from_secs(5));
        let not_responding = FakeHungWindowProbe::new(ProbeResult::NotResponding);
        let base = Instant::now();
        det.poll(base, Some(hb(5)), &not_responding); // baseline
        // The reader returns None now (writer stalled mid-publish / stopped). Staleness still accrues
        // against last_advance.
        let state = det.poll(base + Duration::from_secs(6), None, &not_responding);
        assert!(state.is_frozen(), "an unreadable heartbeat after a baseline + corroboration is a freeze, got {state:?}");
    }

    /// Before ANY heartbeat is seen, the detector is Healthy (a never-started heartbeat is not a freeze)
    /// and does not anchor its clock — so a later first heartbeat starts fresh.
    #[test]
    fn no_heartbeat_yet_is_healthy() {
        let mut det = FreezeDetector::with_threshold(Duration::from_secs(5));
        let not_responding = FakeHungWindowProbe::new(ProbeResult::NotResponding);
        let base = Instant::now();
        // Many ticks with NO heartbeat ever: never a freeze (no baseline to be stale against).
        for i in 0..50u64 {
            let state = det.poll(base + Duration::from_secs(i), None, &not_responding);
            assert_eq!(state, FreezeState::Healthy, "no heartbeat yet must be Healthy (tick {i})");
        }
        // The first heartbeat arrives at +50s; it is Healthy and anchors the clock.
        assert_eq!(det.poll(base + Duration::from_secs(50), Some(hb(1)), &not_responding), FreezeState::Healthy);
    }
}
