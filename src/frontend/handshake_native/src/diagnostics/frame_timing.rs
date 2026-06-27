//! WP-KERNEL-012 MT-085 (D2 — internal_diagnostics, Tier 2): per-frame FRAME-TIME tracking + a typed
//! `SlowFrame` diagnostic event (Master Spec v02.196 §5.8.2 "frame-time and resource counters" +
//! §5.8.4 panel stats).
//!
//! # What this is — the in-process degradation signal BELOW a full freeze
//!
//! Frame-time tracking is the cheap in-process complement to Palmistry's external freeze detection
//! (Tier 3, MT-091). A frame that takes 250ms is NOT yet the ~5s freeze Palmistry watches for, but it
//! IS a stutter the operator/agent feels and the Diagnostics Panel (MT-087, §5.8.4) must surface. So
//! this module measures the WORK TIME of every frame on the UI thread, keeps rolling stats
//! (last/min/max + p50/p95 over a small ring), and — when a frame exceeds the [`SLOW_FRAME_THRESHOLD`]
//! — emits ONE typed [`DiagEventCode::SlowFrame`] event through the MT-082 recorder so it lands in the
//! in-process buffer (panel) AND the MT-081 ring (Palmistry).
//!
//! # Why WORK time, not the inter-frame PERIOD (RISK-005-1 / AC-005-3)
//!
//! egui repaints on demand. MT-084 schedules an idle keep-alive repaint every ~250ms
//! ([`crate::app::HEARTBEAT_IDLE_REPAINT_INTERVAL`]) so the heartbeat keeps advancing on an idle app.
//! If we measured the inter-frame PERIOD, that idle 250ms gap (the time the app spends *waiting* for
//! the next frame, doing nothing) would itself look like a 250ms "slow frame" and FLOOD the bounded
//! ring with false `SlowFrame` events, evicting real ones (RISK-005-1). We therefore measure the WORK
//! time INSIDE `update` — the wall duration of `self.ui(ctx)` — which is small on an idle frame
//! regardless of how long the app then waits before the next repaint. The idle keep-alive does trivial
//! work, so it never flags (proven by AC-005-3). This is the cleaner choice the MT recommends.
//!
//! # Debounce (RISK-005-2 / AC-005-4)
//!
//! A sustained slow streak (e.g. a heavy operation taking 30 slow frames in a row) must NOT emit one
//! `SlowFrame` per frame — that would flood the bounded ring (MT-081) and evict useful events. A
//! last-emit `Instant` gate ([`SLOW_FRAME_EMIT_DEBOUNCE`]) bounds the emit rate to at most one
//! `SlowFrame` per ~1s window. The rolling STATS still update every frame (they are free); only the
//! ring EMIT is debounced.
//!
//! # Typed-allowlist (no content)
//!
//! Everything exposed is typed integers in MICROS ([`FrameStats`] is all `u64`). The emitted event
//! reuses the MT-081 [`DiagEvent::slow_frame`] constructor (frame index + `frame_micros`), which
//! carries NO text/blob — the privacy invariant (§5.8.3) is structural at the boundary.

use std::time::{Duration, Instant};

use handshake_diag_ring::{DiagEvent, DiagEventCode, DiagPhase, DiagSeverity};

/// The slow-frame threshold: a frame whose WORK time exceeds this is a stutter worth surfacing.
///
/// Tuned (RISK-005-3) to sit ABOVE a normal frame so 60fps (16.7ms) and 30fps (33.3ms) frames NEVER
/// flag, and above the trivial work an idle keep-alive frame (MT-084) does — only a GENUINE stutter
/// (work below ~10fps) flags. ~100ms is the standard "perceptible jank" boundary used by GUI/game
/// profilers. A named const so it is trivial to re-tune.
pub const SLOW_FRAME_THRESHOLD: Duration = Duration::from_millis(100);

/// Debounce window for the `SlowFrame` RING EMIT (RISK-005-2 / AC-005-4). During a sustained slow
/// streak, at most one `SlowFrame` event is emitted per this window, so a long slow period emits a
/// BOUNDED count rather than one event per frame (which would flood the bounded MT-081 ring and evict
/// useful events). The rolling stats still update every frame; only the emit is gated.
pub const SLOW_FRAME_EMIT_DEBOUNCE: Duration = Duration::from_millis(1000);

/// Size of the fixed rolling ring of recent frame durations the p50/p95 are computed over. ~120 frames
/// is ~2s at 60fps — enough for a stable percentile the panel (MT-087) reads, small enough that the
/// per-frame insert + the occasional percentile sort are negligible.
pub const FRAME_RING_CAPACITY: usize = 120;

/// A typed snapshot of the frame-time stats the Diagnostics Panel (MT-087) reads (§5.8.4). EVERY field
/// is an integer in MICROSECONDS — no content, no text, no blob (the typed-allowlist invariant). All
/// zero before the first frame is recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameStats {
    /// Number of frames recorded so far (a monotonic counter; saturates at `u64::MAX`).
    pub frame_count: u64,
    /// The most recently recorded frame WORK time, in microseconds.
    pub last_micros: u64,
    /// The minimum frame WORK time observed over the whole session, in microseconds.
    pub min_micros: u64,
    /// The maximum frame WORK time observed over the whole session, in microseconds.
    pub max_micros: u64,
    /// The p50 (median) frame WORK time over the recent ring, in microseconds.
    pub p50_micros: u64,
    /// The p95 frame WORK time over the recent ring, in microseconds.
    pub p95_micros: u64,
    /// Number of `SlowFrame` events actually EMITTED to the ring (after debouncing). Lets the panel
    /// show "N slow frames flagged" distinct from the count of slow frames seen.
    pub slow_emit_count: u64,
}

/// Rolling per-frame frame-time tracker. Owns the recent-duration ring, the session min/max, the
/// monotonic frame counter, and the debounce gate. Updated once per frame from the UI thread via
/// [`FrameTimer::record_frame`]; the panel reads [`FrameTimer::stats`].
///
/// Holds NO `Instant` of its own for the duration measurement — the caller passes the measured
/// [`Duration`] (the WORK time of `self.ui(ctx)`), keeping the type clock-source-agnostic and trivially
/// unit-testable with synthetic durations (AC-005-1). The only `Instant` it owns is the debounce
/// last-emit marker.
pub struct FrameTimer {
    /// Fixed-capacity ring of the most recent frame durations (micros) for the p50/p95. Drop-oldest.
    recent_micros: std::collections::VecDeque<u64>,
    /// Monotonic count of frames recorded.
    frame_count: u64,
    /// Session minimum frame time in micros (`None` until the first frame).
    min_micros: Option<u64>,
    /// Session maximum frame time in micros (`None` until the first frame).
    max_micros: Option<u64>,
    /// The most recent frame time in micros (`None` until the first frame).
    last_micros: Option<u64>,
    /// When the last `SlowFrame` ring emit happened (`None` until the first emit). The debounce gate
    /// compares against this so a sustained slow streak emits a bounded count (AC-005-4).
    last_slow_emit: Option<Instant>,
    /// Count of `SlowFrame` events actually emitted (after debouncing).
    slow_emit_count: u64,
    /// Process-relative monotonic anchor captured at construction. The emitted `SlowFrame` event's
    /// `timestamp_nanos` is `now - created_at` (the same MONOTONIC-Instant approach the MT-084 heartbeat
    /// uses), so it strictly increases and never goes backward on a wall-clock change — and it is the
    /// time the slow frame was OBSERVED, not the frame's own duration.
    created_at: Instant,
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameTimer {
    /// A fresh tracker with empty stats.
    pub fn new() -> Self {
        Self {
            recent_micros: std::collections::VecDeque::with_capacity(FRAME_RING_CAPACITY),
            frame_count: 0,
            min_micros: None,
            max_micros: None,
            last_micros: None,
            last_slow_emit: None,
            slow_emit_count: 0,
            created_at: Instant::now(),
        }
    }

    /// Record one frame's WORK time and, if it is slow (and the debounce window allows), emit a typed
    /// `SlowFrame` event through the closure `emit`.
    ///
    /// This is the CLOCK-AGNOSTIC, side-effect-injected core used by the unit test (AC-005-1): the
    /// caller passes the measured `work` duration and a `now` instant (for the debounce gate) plus an
    /// `emit` closure that records the built [`DiagEvent`]. The live frame path
    /// ([`record_frame_live`]) supplies the production wiring (`Instant::now()` + the global recorder);
    /// the test supplies `Instant::now()` + a capturing closure so it can assert exactly which events
    /// were emitted with no process-global coupling.
    ///
    /// Returns `true` if a `SlowFrame` event was emitted for this frame (slow AND past the debounce
    /// window), `false` otherwise (fast frame, or slow but inside the debounce window).
    ///
    /// STATS update EVERY frame (cheap); only the EMIT is gated by the slow threshold + debounce. The
    /// emitted event carries `frame_index` (the frame counter) + `frame_micros` (this frame's work) and
    /// NO text — the typed-allowlist holds.
    pub fn record_frame<F: FnOnce(DiagEvent)>(
        &mut self,
        work: Duration,
        now: Instant,
        emit: F,
    ) -> bool {
        // u64 micros, saturating so an absurd duration can never panic the frame path.
        let micros = u64::try_from(work.as_micros()).unwrap_or(u64::MAX);

        // ── stats: update every frame (free, no threshold) ──
        self.frame_count = self.frame_count.saturating_add(1);
        self.last_micros = Some(micros);
        self.min_micros = Some(self.min_micros.map_or(micros, |m| m.min(micros)));
        self.max_micros = Some(self.max_micros.map_or(micros, |m| m.max(micros)));
        if self.recent_micros.len() >= FRAME_RING_CAPACITY {
            self.recent_micros.pop_front();
        }
        self.recent_micros.push_back(micros);

        // ── slow-frame emit: gated by the threshold AND the debounce window ──
        if work <= SLOW_FRAME_THRESHOLD {
            return false; // a normal/fast frame — never flags (RISK-005-3 / AC-005-2 fast half).
        }
        // Slow frame. Debounce: only emit if past the window since the last emit (AC-005-4).
        let allow = match self.last_slow_emit {
            None => true,
            Some(prev) => now.duration_since(prev) >= SLOW_FRAME_EMIT_DEBOUNCE,
        };
        if !allow {
            return false; // slow, but inside the debounce window — counted in stats, not emitted.
        }
        self.last_slow_emit = Some(now);
        self.slow_emit_count = self.slow_emit_count.saturating_add(1);

        // Monotonic timestamp: nanos elapsed since the timer's construction, measured from the caller's
        // `now` Instant (immune to wall-clock changes, like the MT-084 heartbeat). This is the time the
        // slow frame was OBSERVED, not the frame's duration. The MT-081 `slow_frame` constructor takes
        // (thread_id, sequence_id, frame_index, frame_micros, timestamp_nanos).
        let timestamp_nanos = u64::try_from(now.duration_since(self.created_at).as_nanos())
            .unwrap_or(u64::MAX);
        let event = DiagEvent::slow_frame(
            /* thread_id  */ 0,
            /* sequence_id*/ self.slow_emit_count,
            /* frame_index*/ self.frame_count,
            /* frame_micros */ micros,
            /* timestamp_nanos */ timestamp_nanos,
        );
        // Belt-and-suspenders: the constructor already sets these, but assert the typed shape so a
        // future constructor change cannot silently drift the event code/severity off the allowlist.
        debug_assert_eq!(event.event_code, DiagEventCode::SlowFrame.as_u16());
        debug_assert_eq!(event.phase_marker, DiagPhase::Tick.as_u8());
        debug_assert_eq!(event.severity, DiagSeverity::Warn.as_u8());
        emit(event);
        true
    }

    /// The live-frame wiring: measure-and-record using `Instant::now()` for the debounce gate and the
    /// process-global MT-082 recorder ([`crate::diagnostics::record`]) as the emit sink. Called from
    /// [`crate::app::HandshakeApp::update`] with the WORK time of `self.ui(ctx)`. Returns whether a
    /// `SlowFrame` was emitted this frame.
    ///
    /// The whole call is allocation-free on a fast frame (the overwhelmingly common case): two integer
    /// reads + a bounded `VecDeque` push (pre-reserved, drop-oldest) + a few comparisons. No `format!`,
    /// no heap growth, no lock on the fast path (the `record` ring write only happens on a debounced
    /// slow frame). RISK-005-4: the measurement adds two `Instant` reads + an integer push, negligible.
    pub fn record_frame_live(&mut self, work: Duration) -> bool {
        self.record_frame(work, Instant::now(), |event| {
            crate::diagnostics::record(event);
        })
    }

    /// A typed snapshot of the current stats for the Diagnostics Panel (MT-087). All micros (u64), no
    /// content. p50/p95 are computed over the recent ring; min/max are session-wide; `last` is the most
    /// recent frame. All zero before the first `record_frame`.
    pub fn stats(&self) -> FrameStats {
        let (p50, p95) = self.percentiles();
        FrameStats {
            frame_count: self.frame_count,
            last_micros: self.last_micros.unwrap_or(0),
            min_micros: self.min_micros.unwrap_or(0),
            max_micros: self.max_micros.unwrap_or(0),
            p50_micros: p50,
            p95_micros: p95,
            slow_emit_count: self.slow_emit_count,
        }
    }

    /// Compute (p50, p95) over the recent-duration ring using the nearest-rank method on a sorted copy.
    /// Returns (0, 0) when no frame has been recorded. The sort is over at most [`FRAME_RING_CAPACITY`]
    /// `u64`s (~120) — negligible, and only invoked when the panel reads stats, never on the frame path.
    fn percentiles(&self) -> (u64, u64) {
        if self.recent_micros.is_empty() {
            return (0, 0);
        }
        let mut sorted: Vec<u64> = self.recent_micros.iter().copied().collect();
        sorted.sort_unstable();
        let n = sorted.len();
        // Nearest-rank: index = ceil(p/100 * n) - 1, clamped into [0, n-1].
        let rank = |p: usize| -> u64 {
            let idx = (p * n).div_ceil(100).saturating_sub(1).min(n - 1);
            sorted[idx]
        };
        (rank(50), rank(95))
    }
}

#[cfg(test)]
mod tests {
    //! In-crate unit tests for the pure stats + debounce logic that need no ring and no app. The
    //! end-to-end live-frame proofs (a real slow `update` frame flags; idle does not) live in
    //! `tests/test_frame_timing.rs` because they drive the real `HandshakeApp` through egui_kittest.

    use super::*;

    #[test]
    fn fast_frames_never_flag_and_stats_track() {
        let mut timer = FrameTimer::new();
        let now = Instant::now();
        let mut emitted = 0u32;
        // A spread of normal frames: 16ms (60fps), 8ms, 33ms (30fps) — all below the 100ms threshold.
        for ms in [16u64, 8, 33, 16, 8] {
            let flagged = timer.record_frame(Duration::from_millis(ms), now, |_| emitted += 1);
            assert!(!flagged, "{ms}ms is a normal frame and must NOT flag");
        }
        assert_eq!(emitted, 0, "no SlowFrame events for normal 8/16/33ms frames (RISK-005-3)");
        let s = timer.stats();
        assert_eq!(s.frame_count, 5);
        assert_eq!(s.last_micros, 8_000);
        assert_eq!(s.min_micros, 8_000, "min is the 8ms frame");
        assert_eq!(s.max_micros, 33_000, "max is the 33ms frame");
        assert_eq!(s.slow_emit_count, 0);
        // p50/p95 are within the observed range and ordered.
        assert!(s.p50_micros >= 8_000 && s.p50_micros <= 33_000);
        assert!(s.p95_micros >= s.p50_micros, "p95 >= p50");
    }

    #[test]
    fn one_slow_frame_emits_exactly_one_event_with_typed_micros() {
        let mut timer = FrameTimer::new();
        let now = Instant::now();
        let mut events: Vec<DiagEvent> = Vec::new();
        // Two normal frames, then one 250ms slow frame, then a normal frame.
        assert!(!timer.record_frame(Duration::from_millis(16), now, |e| events.push(e)));
        assert!(!timer.record_frame(Duration::from_millis(20), now, |e| events.push(e)));
        let flagged = timer.record_frame(Duration::from_millis(250), now, |e| events.push(e));
        assert!(flagged, "the 250ms frame is slow and must emit (first slow -> past debounce)");
        assert!(!timer.record_frame(Duration::from_millis(16), now, |e| events.push(e)));

        // Exactly ONE SlowFrame event, typed micros, no content (AC-005-1).
        assert_eq!(events.len(), 1, "exactly one SlowFrame event for the one slow frame");
        let e = events[0];
        assert_eq!(e.event_code, DiagEventCode::SlowFrame.as_u16());
        assert_eq!(e.phase_marker, DiagPhase::Tick.as_u8());
        assert_eq!(e.severity, DiagSeverity::Warn.as_u8());
        assert_eq!(e.metric_micros, 250_000, "frame_micros == 250ms in micros");
        assert_eq!(e.counter_a, 3, "frame_index is the 3rd recorded frame");

        // Stats correctness across the slow + fast frames (AC-005-1 stats half).
        let s = timer.stats();
        assert_eq!(s.frame_count, 4);
        assert_eq!(s.last_micros, 16_000, "last is the final 16ms frame");
        assert_eq!(s.min_micros, 16_000);
        assert_eq!(s.max_micros, 250_000, "max is the slow frame");
        assert_eq!(s.slow_emit_count, 1);
    }

    #[test]
    fn sustained_slow_streak_is_debounced() {
        // A long streak of slow frames at the SAME instant (zero elapsed) must emit only ONCE — the
        // debounce window has not elapsed between them (AC-005-4). The stats still see every frame.
        let mut timer = FrameTimer::new();
        let now = Instant::now();
        let mut emitted = 0u32;
        for _ in 0..50 {
            timer.record_frame(Duration::from_millis(200), now, |_| emitted += 1);
        }
        assert_eq!(
            emitted, 1,
            "50 consecutive slow frames within one debounce window emit exactly ONE event (RISK-005-2)"
        );
        let s = timer.stats();
        assert_eq!(s.frame_count, 50, "stats still counted all 50 frames");
        assert_eq!(s.slow_emit_count, 1, "but only one was emitted to the ring");
        assert_eq!(s.max_micros, 200_000);
    }

    #[test]
    fn debounce_window_allows_a_second_emit_after_the_gap() {
        // Two slow frames separated by MORE than the debounce window both emit (the gate is time-based,
        // not a permanent one-shot). Uses synthetic instants to make the gap deterministic.
        let mut timer = FrameTimer::new();
        let t0 = Instant::now();
        let t1 = t0 + SLOW_FRAME_EMIT_DEBOUNCE + Duration::from_millis(10);
        let mut emitted = 0u32;
        assert!(timer.record_frame(Duration::from_millis(200), t0, |_| emitted += 1));
        assert!(timer.record_frame(Duration::from_millis(200), t1, |_| emitted += 1));
        assert_eq!(emitted, 2, "a slow frame past the debounce window emits again");
        assert_eq!(timer.stats().slow_emit_count, 2);
    }

    #[test]
    fn threshold_boundary_is_exclusive_and_tuned_above_30fps() {
        // A frame exactly AT the threshold does NOT flag (`<=` is fast); just over it flags. Confirms
        // the threshold sits above 30fps (33ms) and 60fps (16ms) so normal frames never flag.
        assert!(SLOW_FRAME_THRESHOLD > Duration::from_millis(33), "above a 30fps (33ms) frame");
        assert!(SLOW_FRAME_THRESHOLD > Duration::from_millis(16), "above a 60fps (16ms) frame");
        let mut timer = FrameTimer::new();
        let now = Instant::now();
        let mut emitted = 0u32;
        assert!(
            !timer.record_frame(SLOW_FRAME_THRESHOLD, now, |_| emitted += 1),
            "a frame exactly at the threshold is NOT slow (boundary is exclusive)"
        );
        assert!(
            timer.record_frame(SLOW_FRAME_THRESHOLD + Duration::from_millis(1), now, |_| emitted += 1),
            "a frame just over the threshold IS slow"
        );
        assert_eq!(emitted, 1);
    }

    #[test]
    fn empty_stats_are_all_zero() {
        let timer = FrameTimer::new();
        let s = timer.stats();
        assert_eq!(s, FrameStats::default(), "no frames recorded -> all-zero typed stats");
    }
}
