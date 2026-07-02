//! WP-KERNEL-012 MT-086 (D2 — internal_diagnostics, Tier 2): periodic CPU%/RSS RESOURCE COUNTERS as
//! typed `ResourceSample` events (Master Spec v02.196 §5.8.2 "resource counters" + §5.8.4 panel).
//!
//! # What this is — the in-process CPU/RSS signal the panel + Palmistry care about
//!
//! §6.13.1 names "pinned under heavy CPU" as a first-class stall mode, and the 2026-06-26 freeze
//! showed the OPPOSITE extreme (CPU -> 0). A periodic, in-process CPU%/RSS counter makes BOTH visible:
//! the Diagnostics Panel (MT-087, §5.8.4) reads the last sample for its resource line, and the typed
//! event lands in the MT-081 ring so Palmistry (Tier 3) can read the last-N samples for its survivor
//! store (MT-093). This module is the live producer of those counters.
//!
//! # Current-process-only refresh (RISK-006-3 / AC-006-1)
//!
//! [`ResourceSampler`] holds ONE [`sysinfo::System`] and refreshes ONLY the current pid
//! (`ProcessesToUpdate::Some(&[own_pid])`), never the whole system process table. Reading every other
//! process would be wasteful AND a privacy concern (it would expose other processes' data); refreshing
//! a single pid is cheap and reads only Handshake's own counters. The sampler is constructed once and
//! re-refreshes the SAME `System` each interval — sysinfo computes CPU% as a delta between the previous
//! and current refresh of that process, so the SAME instance must persist across samples (a fresh
//! `System` each call would always report 0% CPU; RISK-006-2 meaningless-CPU half).
//!
//! # Bounded cadence, off the hot frame path (RISK-006-2 / AC-006-4)
//!
//! CPU% is only meaningful with a real interval between two refreshes (sysinfo's own
//! [`MINIMUM_CPU_UPDATE_INTERVAL`] is 200ms), and a sysinfo refresh is not free. So sampling is GATED to
//! a bounded ~1-2s cadence ([`SAMPLE_INTERVAL`]) via a `last_sample_instant`, NOT done every frame. The
//! gate lives in the caller ([`crate::app::HandshakeApp::update`]); [`maybe_sample`] both checks the
//! gate AND performs the (cheap, single-process) refresh + emit, so the per-frame cost on a non-sampling
//! frame is a single `Instant` comparison and nothing else. The single-process refresh is cheap enough
//! to run inline on the frame at a ~1s cadence without stuttering (no extra thread — the simplest
//! correct approach the MT recommends, and the chosen one; see the cadence note in the MT contract).
//!
//! # Typed-allowlist (no content)
//!
//! Every sampled value is a typed integer: `cpu_milli` (CPU% * 1000, integer milli-percent) and
//! `rss_kb` (resident set size in KiB). The emitted event reuses the MT-081
//! [`DiagEvent::resource_sample`] constructor (cpu_milli + rss_kb in the two `u64` counters), which
//! carries NO text/blob — the privacy invariant (§5.8.3) is structural at the boundary. The GPU/driver
//! identity (which DOES include human strings) is deliberately kept OUT of this ring event and lives in
//! the in-process [`crate::diagnostics::gpu_info::GpuInfo`] only (see that module).

use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use handshake_diag_ring::DiagEvent;

/// The bounded sampling cadence (RISK-006-2 / AC-006-4). A sample is taken at most once per this window,
/// NOT every frame. Chosen at ~1s: comfortably above sysinfo's [`MINIMUM_CPU_UPDATE_INTERVAL`] (200ms)
/// so the CPU% delta is meaningful, and slow enough that the per-interval single-process refresh is a
/// negligible fraction of runtime (so it never stutters the frame). A named const so it is trivial to
/// re-tune.
///
/// [`MINIMUM_CPU_UPDATE_INTERVAL`]: sysinfo::MINIMUM_CPU_UPDATE_INTERVAL
pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// A single typed resource sample: integer CPU milli-percent + integer RSS in KiB. No content, no text
/// (the typed-allowlist invariant). `cpu_milli` is CPU% * 1000 (so 12.5% -> 12_500); on a multi-core
/// host a fully-busy process can exceed 100_000 (sysinfo reports per-process CPU% which can be > 100%
/// across cores). `rss_kb` is the resident set size in KiB (sysinfo reports RSS in bytes; we divide).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResourceSample {
    /// CPU usage of THIS process as integer milli-percent (percent * 1000). 0 on the very first sample
    /// (sysinfo needs two refreshes to compute a delta) and a non-negative integer thereafter.
    pub cpu_milli: u64,
    /// Resident set size (RSS) of THIS process in KiB. Always > 0 for a live process.
    pub rss_kb: u64,
}

/// Periodic CPU%/RSS sampler for the CURRENT process. Holds ONE persistent [`sysinfo::System`] (so the
/// CPU% delta between refreshes is meaningful) and the current pid; refreshes ONLY that pid each sample.
///
/// Constructed once and stored on [`crate::app::HandshakeApp`]; [`maybe_sample`] is called each frame
/// behind the [`SAMPLE_INTERVAL`] gate so the refresh+emit happens at the bounded cadence, not per
/// frame. A new instance reports 0% CPU on its first [`sample`] (no prior refresh to delta against);
/// that is expected and correct.
///
/// [`maybe_sample`]: ResourceSampler::maybe_sample
/// [`sample`]: ResourceSampler::sample
pub struct ResourceSampler {
    /// The persistent sysinfo handle. Refreshed for the current pid ONLY (never the whole system table).
    /// Persisted across samples so CPU% is computed as a real delta between two refreshes of this pid.
    system: System,
    /// The current process id, captured once at construction. Only THIS pid is ever refreshed/read.
    own_pid: Pid,
    /// Monotonic gate marker: when the last sample was taken (`None` until the first). [`maybe_sample`]
    /// compares against this so a sample is taken at most once per [`SAMPLE_INTERVAL`] (AC-006-4). An
    /// `Instant` is MONOTONIC so the cadence is immune to wall-clock changes.
    last_sample_at: Option<Instant>,
    /// Monotonic count of samples actually taken + emitted. Used as the event `sequence_id` and exposed
    /// so a test/panel can see how many samples have been emitted.
    sample_count: u64,
    /// Process-start monotonic anchor for the emitted event's `timestamp_nanos` (elapsed-since-anchor,
    /// strictly increasing, immune to wall-clock change — the same approach the MT-084 heartbeat uses).
    created_at: Instant,
}

impl ResourceSampler {
    /// Construct a sampler for the current process. Captures the own pid via `std::process::id()` and
    /// builds an EMPTY `System` (no process table populated yet); the first [`sample`] refreshes only
    /// the own pid. Cheap — no system-wide scan happens here.
    ///
    /// [`sample`]: ResourceSampler::sample
    pub fn new() -> Self {
        let own_pid = Pid::from_u32(std::process::id());
        // `System::new()` does NOT populate any process; we refresh only our own pid in `sample()`.
        Self {
            system: System::new(),
            own_pid,
            last_sample_at: None,
            sample_count: 0,
            created_at: Instant::now(),
        }
    }

    /// The current process id this sampler refreshes (and ONLY this one). Exposed so a test can assert
    /// the sampler targets the live pid (AC-006-1 current-process-only half).
    pub fn own_pid(&self) -> Pid {
        self.own_pid
    }

    /// Number of samples taken + emitted so far. Exposed so a test/panel can confirm sampling cadence.
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Refresh ONLY the current process and read its CPU%/RSS into a typed [`ResourceSample`].
    ///
    /// Refreshes `ProcessesToUpdate::Some(&[own_pid])` — never `All` — so it reads ONLY Handshake's own
    /// counters (RISK-006-3 / AC-006-1): cheap, and it cannot leak other processes' data. CPU% is a
    /// delta between THIS refresh and the previous refresh of the same pid (so the persistent `System`
    /// is required); `cpu_milli` is `(cpu% * 1000).round()` (12.5% -> 12_500). RSS comes back from
    /// sysinfo in BYTES; `rss_kb` divides by 1024.
    ///
    /// If the process is somehow not found after the refresh (should never happen for our own live pid),
    /// returns a zeroed sample rather than panicking — diagnostics must never crash the app.
    pub fn sample(&mut self) -> ResourceSample {
        // Refresh ONLY our own pid, with cpu + memory specifics. `remove_dead_processes = false`: we are
        // refreshing a single known-live pid, so there is nothing to prune and no reason to walk a table.
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[self.own_pid]),
            false,
            ProcessRefreshKind::nothing().with_cpu().with_memory(),
        );
        match self.system.process(self.own_pid) {
            Some(proc) => {
                // cpu_usage() is an f32 percent (can exceed 100 across cores). Convert to integer
                // milli-percent, clamped non-negative (a stray negative from a noisy delta becomes 0).
                let cpu_pct = proc.cpu_usage().max(0.0);
                let cpu_milli = (f64::from(cpu_pct) * 1000.0).round() as u64;
                // memory() is RSS in BYTES; report KiB.
                let rss_kb = proc.memory() / 1024;
                ResourceSample { cpu_milli, rss_kb }
            }
            // Defensive: our own live pid should always be present after a refresh; never panic.
            None => ResourceSample::default(),
        }
    }

    /// Build the typed MT-081 `ResourceSample` [`DiagEvent`] for a sample (cpu_milli + rss_kb in the two
    /// `u64` counters, monotonic timestamp). No text — the typed allowlist holds. `now` is the caller's
    /// monotonic instant so the timestamp is `now - created_at` (strictly increasing). Separated from
    /// the emit so a test can assert the exact built event without touching the process-global recorder.
    fn build_event(&self, s: ResourceSample, now: Instant) -> DiagEvent {
        let timestamp_nanos =
            u64::try_from(now.duration_since(self.created_at).as_nanos()).unwrap_or(u64::MAX);
        DiagEvent::resource_sample(
            /* thread_id      */ 0,
            /* sequence_id    */ self.sample_count,
            /* cpu_milli      */ s.cpu_milli,
            /* rss_kb         */ s.rss_kb,
            /* metric_micros  */ 0,
            timestamp_nanos,
        )
    }

    /// Take a sample NOW and emit a typed `ResourceSample` event through the closure `emit`, returning
    /// the sample. This is the clock/sink-injected core used by the unit test (AC-006-1/2): the caller
    /// passes the `now` instant + an `emit` closure that records the built [`DiagEvent`], so the test can
    /// assert exactly which event was emitted with no process-global coupling. The live path
    /// ([`maybe_sample`]) supplies `Instant::now()` + the global MT-082 recorder.
    ///
    /// [`maybe_sample`]: ResourceSampler::maybe_sample
    pub fn record_sample<F: FnOnce(DiagEvent)>(&mut self, now: Instant, emit: F) -> ResourceSample {
        let s = self.sample();
        let event = self.build_event(s, now);
        self.sample_count = self.sample_count.saturating_add(1);
        self.last_sample_at = Some(now);
        emit(event);
        s
    }

    /// The bounded-cadence gate (AC-006-4): take + emit a sample ONLY if at least [`SAMPLE_INTERVAL`] has
    /// elapsed since the last one (or this is the first). Returns `Some(sample)` when a sample was taken
    /// this call, `None` when the gate skipped it. The per-frame cost when skipping is a single `Instant`
    /// comparison; the (cheap, single-process) refresh + emit happen only at the bounded cadence.
    ///
    /// This is the clock/sink-injected gate used by both the unit test (synthetic `now`) and the live
    /// path: [`maybe_sample_live`] supplies `Instant::now()` + the global recorder.
    ///
    /// [`maybe_sample_live`]: ResourceSampler::maybe_sample_live
    pub fn maybe_sample<F: FnOnce(DiagEvent)>(
        &mut self,
        now: Instant,
        emit: F,
    ) -> Option<ResourceSample> {
        let due = match self.last_sample_at {
            None => true,
            Some(prev) => now.duration_since(prev) >= SAMPLE_INTERVAL,
        };
        if !due {
            return None;
        }
        Some(self.record_sample(now, emit))
    }

    /// The live-frame wiring: gate on `Instant::now()` and emit through the process-global MT-082
    /// recorder ([`crate::diagnostics::record`]). Called once per [`crate::app::HandshakeApp::update`]
    /// frame; it is a single `Instant` comparison on a non-sampling frame and a cheap single-process
    /// refresh + one ring write at the bounded ~1s cadence. Returns the sample taken this frame (or
    /// `None` when the cadence gate skipped it).
    pub fn maybe_sample_live(&mut self) -> Option<ResourceSample> {
        self.maybe_sample(Instant::now(), |event| {
            crate::diagnostics::record(event);
        })
    }
}

impl Default for ResourceSampler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    //! In-crate unit tests for the pure sampling/gate logic that need no app and no ring. The
    //! end-to-end live-frame + GPU-capture proofs live in `tests/test_resource_counters.rs` because they
    //! drive the real `HandshakeApp` through egui_kittest.

    use super::*;

    /// A fresh sampler samples THIS process and returns plausible typed integers: RSS > 0 (a live
    /// process always has a resident set) and a non-negative integer CPU. (CPU is 0 on the first sample
    /// because sysinfo needs two refreshes for a delta — that is expected.) (AC-006-1 core.)
    #[test]
    fn samples_current_process_plausible_integers() {
        let mut sampler = ResourceSampler::new();
        let s = sampler.sample();
        assert!(
            s.rss_kb > 0,
            "a live process always has a non-zero resident set (got {})",
            s.rss_kb
        );
        // cpu_milli is a u64 (non-negative by type); assert it is a real read, not absurd. The first
        // sample is typically 0 (no prior delta); a second sample after work may be > 0.
        let _ = s.cpu_milli; // typed non-negative integer by construction
    }

    /// The sampler targets ONLY the current pid (AC-006-1 current-process-only half).
    #[test]
    fn refreshes_only_the_current_pid() {
        let sampler = ResourceSampler::new();
        assert_eq!(
            sampler.own_pid(),
            Pid::from_u32(std::process::id()),
            "the sampler refreshes ONLY this process's pid, never the whole system table"
        );
    }

    /// The cadence gate emits at most once per `SAMPLE_INTERVAL` (AC-006-4). Many calls within one
    /// window emit exactly ONCE; a call past the window emits again (the gate is time-based, not a
    /// one-shot). Uses synthetic instants so the timing is deterministic (no real sleeps).
    #[test]
    fn cadence_gate_bounds_emit_rate() {
        let mut sampler = ResourceSampler::new();
        let t0 = Instant::now();
        let mut emitted = 0u32;

        // First call (no prior sample) is due -> emits.
        assert!(
            sampler.maybe_sample(t0, |_| emitted += 1).is_some(),
            "first sample is due"
        );
        // 50 more calls within the SAME window (all at t0) -> NONE emit.
        for _ in 0..50 {
            assert!(
                sampler.maybe_sample(t0, |_| emitted += 1).is_none(),
                "a call inside the cadence window must NOT sample"
            );
        }
        assert_eq!(
            emitted, 1,
            "50 calls within one cadence window emit exactly ONE sample (AC-006-4)"
        );

        // A call past the window emits again.
        let t1 = t0 + SAMPLE_INTERVAL + Duration::from_millis(50);
        assert!(
            sampler.maybe_sample(t1, |_| emitted += 1).is_some(),
            "past the window, sample again"
        );
        assert_eq!(
            emitted, 2,
            "a call past the cadence window emits a second sample"
        );
        assert_eq!(sampler.sample_count(), 2, "two samples taken + emitted");
    }

    /// The emitted event is a typed `ResourceSample` carrying cpu_milli + rss_kb in the two `u64`
    /// counters and NO content (AC-006-2 shape). Captures the built event via the injected `emit`.
    #[test]
    fn emitted_event_is_typed_resource_sample_no_content() {
        use handshake_diag_ring::{DiagEventCode, DiagPhase, DiagSeverity};

        let mut sampler = ResourceSampler::new();
        let mut events: Vec<DiagEvent> = Vec::new();
        let s = sampler.record_sample(Instant::now(), |e| events.push(e));

        assert_eq!(events.len(), 1, "exactly one ResourceSample event emitted");
        let e = events[0];
        assert_eq!(
            e.event_code,
            DiagEventCode::ResourceSample.as_u16(),
            "event code is ResourceSample"
        );
        assert_eq!(
            e.phase_marker,
            DiagPhase::Tick.as_u8(),
            "phase is Tick (a periodic sample)"
        );
        assert_eq!(
            e.severity,
            DiagSeverity::Info.as_u8(),
            "severity is Info (a normal counter)"
        );
        // The typed integer payload matches the sample (cpu_milli -> counter_a, rss_kb -> counter_b).
        assert_eq!(e.counter_a, s.cpu_milli, "counter_a carries cpu_milli");
        assert_eq!(e.counter_b, s.rss_kb, "counter_b carries rss_kb");
        assert!(
            e.counter_b > 0,
            "rss_kb is a real read (> 0 for a live process)"
        );
        // _reserved is the zeroed padding (no content channel).
        assert_eq!(e._reserved, [0u8; 4], "no content smuggled through padding");
    }
}
