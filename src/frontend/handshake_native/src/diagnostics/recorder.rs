//! The `DiagnosticsRecorder` — the process-global Tier 2 internal_diagnostics facade and its OPEN
//! `record()` API (Master Spec v02.196 §5.8.2 "Open diagnostic-event API" + §5.8.4).
//!
//! # What this is
//!
//! Tier 2 of Handshake's three-tier diagnostic model. Any feature in the running app — a backend
//! call site, a widget, a GUI action, a job bridge — calls [`record`] (or the ergonomic
//! [`record_with`]) and the event is:
//!
//! 1. appended to a small BOUNDED in-process ring buffer (cap [`BUFFER_CAP`]) that the in-app
//!    Diagnostics Panel (MT-087) reads via [`snapshot_last_n`]; and
//! 2. written to the MT-081 shared-memory ring ([`handshake_diag_ring::DiagRingWriter`]) IF a writer
//!    has been [`install`]ed, so the external Palmistry watcher (Tier 3) sees it with zero cooperation.
//!
//! # Why a process-global (`OnceLock`)
//!
//! Per §5.8.2 the API is OPEN BY DESIGN: *any* feature MAY call it, from any module, without
//! threading a handle through every call site. That is exactly the standard Rust global-logger /
//! global-metrics pattern (e.g. `tracing` / `metrics`). So the recorder lives behind a process-global
//! [`OnceLock<DiagnosticsRecorder>`] and the public surface is a set of FREE functions ([`record`],
//! [`record_with`], [`install`], [`snapshot_last_n`], [`dropped_count`]) that operate on the global.
//! The recorder uses interior mutability (`Mutex<VecDeque>` + atomics) so `record()` needs no `&mut`.
//!
//! # The hard invariants (red-team controls)
//!
//! - **Non-blocking + panic-free on the hot path (RISK-002-1 / RISK-002-3).** The UI thread calls
//!   `record()` every frame (MT-084/005). It therefore must never allocate heavily, never block on a
//!   contended lock, and never panic. A poisoned buffer mutex is recovered (`into_inner`); a full
//!   buffer drops the OLDEST event; a missing writer is a silent no-op on the ring side. [`DiagEvent`]
//!   is a fixed-size `Copy` POD, so pushing one is allocation-free into a pre-reserved `VecDeque`.
//! - **Graceful degradation (RISK-002-5 / AC-002-3).** Before [`install`], and in any headless/test
//!   shell that created no ring, `record()` still buffers in-process — it just is not visible to
//!   Palmistry yet. A missing or failed ring NEVER crashes the app.
//! - **Typed-allowlist preserved at the API boundary (RISK-002-4 / AC-002-5).** The recorder stores
//!   and accepts only [`DiagEvent`] / the typed enums from MT-081. There is deliberately NO public
//!   surface that accepts free text (`String`/`&str`) or a byte blob, so the no-sensitive-data
//!   invariant (§5.8.3) is structural, not a runtime check.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use handshake_diag_ring::{
    DiagEvent, DiagEventCode, DiagPhase, DiagRingWriter, DiagSeverity,
};

/// Capacity of the in-process last-N ring the Diagnostics Panel (MT-087) reads. Bounded so a
/// long-running session can never grow it unbounded; the oldest event is dropped on overflow. Mirrors
/// the `event_emitter` cap-20 error-ring shape, sized up to the §5.8.4 panel budget.
pub const BUFFER_CAP: usize = 512;

/// The per-session diagnostic identity that MT-094 passes to `palmistry.exe`: the session id (so a
/// watcher can correlate) and the backing-file PATH of the shared-memory ring (so the watcher maps the
/// SAME ring). Stored on [`crate::app::HandshakeApp`] at install time. Carries NO sensitive data — a
/// uuid and a filesystem path only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagSession {
    /// The per-session id (a uuid v4) used to name the ring backing file and correlate Palmistry.
    pub session_id: String,
    /// The filesystem path of the MT-081 ring backing file Handshake created (what Palmistry maps).
    pub ring_path: PathBuf,
}

/// The Tier 2 internal_diagnostics recorder. One per process, behind the [`OnceLock`] global below.
///
/// Interior-mutable: `record()` takes `&self` (the global is shared `&'static`), pushes into the
/// `Mutex<VecDeque>` buffer, and (if installed) writes the MT-081 ring. `dropped` counts events shed
/// when the buffer is at cap, so the panel can show "N events dropped" rather than silently losing the
/// count.
pub struct DiagnosticsRecorder {
    /// The MT-081 shared-memory ring writer. `None` until [`install`]; a headless/test shell that
    /// created no ring leaves it `None` and records in-process only (graceful degradation).
    writer: Option<DiagRingWriter>,
    /// The bounded in-process last-N buffer the Diagnostics Panel reads. Drop-oldest on overflow.
    buffer: Mutex<VecDeque<DiagEvent>>,
    /// Count of events shed because the buffer was at cap (never grows the buffer; just accounts).
    dropped: AtomicU64,
}

impl DiagnosticsRecorder {
    /// Construct an empty recorder with no ring writer (in-process-only until a writer is installed).
    fn new(writer: Option<DiagRingWriter>) -> Self {
        Self {
            writer,
            buffer: Mutex::new(VecDeque::with_capacity(BUFFER_CAP)),
            dropped: AtomicU64::new(0),
        }
    }

    /// Construct a recorder around a real MT-081 ring writer. This is the standalone instance the
    /// production global wraps; it is also the construction AC-002-1 uses to prove a `record()` call
    /// both buffers in-process AND writes the shared-memory ring (read back by a separate
    /// `DiagRingReader` on the same backing file). Exposed (rather than only the global free functions)
    /// so the ring-read-back proof is DETERMINISTIC and does not fight the process-global `OnceLock`
    /// that the live-consumer test (AC-002-4) owns.
    pub fn with_writer(writer: DiagRingWriter) -> Self {
        Self::new(Some(writer))
    }

    /// Construct a recorder with NO ring writer — the graceful-degradation instance (AC-002-3): records
    /// buffer in-process only and never touch a ring. Exposed for the no-writer instance proof.
    pub fn in_process_only() -> Self {
        Self::new(None)
    }

    /// Record one event. NON-BLOCKING + PANIC-FREE: pushes into the bounded buffer (drop-oldest on
    /// overflow, never grows, never blocks the caller longer than the short buffer lock) AND writes the
    /// MT-081 ring if a writer is installed (the ring writer is itself wait-free by design).
    ///
    /// A poisoned buffer mutex is RECOVERED via `into_inner` rather than unwrapped, so a panic in some
    /// unrelated holder of the lock can never propagate a poison panic into a diagnostics call on the
    /// UI thread.
    pub fn record(&self, event: DiagEvent) {
        // In-process buffer (what the panel reads). Recover a poisoned lock instead of panicking.
        let mut guard = match self.buffer.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if guard.len() >= BUFFER_CAP {
            guard.pop_front();
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
        guard.push_back(event);
        // Drop the buffer lock BEFORE touching the ring so the (wait-free) ring write never holds the
        // buffer lock, keeping the contended section minimal.
        drop(guard);

        // Shared-memory ring (what Palmistry reads). Silent no-op when no writer is installed.
        if let Some(writer) = &self.writer {
            writer.write(event);
        }
    }

    /// Publish the heartbeat slot (MT-084). Called from the UI thread every egui frame.
    ///
    /// CRITICAL — this is the single most important producer in the diagnostic substrate (the
    /// liveness signal Palmistry polls): it MUST be wait-free and allocation-free on the frame path.
    /// It therefore does EXACTLY one thing — forward the two integers to the MT-081 ring's dedicated
    /// `write_heartbeat` (a single seqlock store of two `u64`s into the mapped header) — and TOUCHES
    /// NEITHER the in-process buffer NOR its `Mutex` (so the per-frame heartbeat never contends with
    /// `record()` on the record buffer; RISK-004-3 / AC-004-3). It is a silent no-op when no writer is
    /// installed (headless/test path; RISK-004-* graceful degradation / AC-004-5).
    ///
    /// `counter` is the monotonic frame counter; `timestamp_nanos` is a MONOTONIC nanosecond clock
    /// (process-start `Instant` elapsed — never a wall clock, so it cannot go backward on a clock
    /// change; the staleness math in Palmistry compares this value; RISK-004-2 / AC-004-2).
    #[inline]
    pub fn heartbeat(&self, counter: u64, timestamp_nanos: u64) {
        // No buffer lock, no allocation, no format!: just the wait-free ring header seqlock store.
        if let Some(writer) = &self.writer {
            writer.write_heartbeat(counter, timestamp_nanos);
        }
    }

    /// A snapshot of up to the last `n` events, oldest-first (the order the panel renders). Cheap
    /// clone of `Copy` POD events under a brief lock.
    pub fn snapshot_last_n(&self, n: usize) -> Vec<DiagEvent> {
        let guard = match self.buffer.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        let len = guard.len();
        let start = len.saturating_sub(n);
        guard.iter().skip(start).copied().collect()
    }

    /// The number of events shed because the buffer was at cap.
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

/// The process-global recorder. Initialized lazily on first use (so `record()` works even before
/// [`install`]), then [`install`] swaps in the ring writer once at startup.
static RECORDER: OnceLock<DiagnosticsRecorder> = OnceLock::new();

/// Get-or-init the global recorder. Before [`install`] runs, this initializes a writer-less recorder
/// so `record()` buffers in-process from the very first call (graceful degradation / AC-002-3).
#[inline]
fn global() -> &'static DiagnosticsRecorder {
    RECORDER.get_or_init(|| DiagnosticsRecorder::new(None))
}

/// Install the MT-081 ring writer onto the process-global recorder. Called ONCE at startup
/// ([`crate::app::HandshakeApp::new`]) after the ring is created. Returns `true` if the writer was
/// installed, `false` if a recorder was already initialized (writer-less or already installed) — in
/// which case the buffered-only path stays in effect and the caller logs the (non-fatal) condition.
///
/// This is intentionally best-effort: a process may only install a ring writer once (the `OnceLock`
/// can be set once). If the global was already initialized writer-less by an early `record()` call,
/// the ring writer cannot be retrofitted into the existing instance, so this returns `false` and the
/// app degrades to in-process-only diagnostics for the session (it never panics or aborts startup).
pub fn install(writer: DiagRingWriter) -> bool {
    RECORDER.set(DiagnosticsRecorder::new(Some(writer))).is_ok()
}

/// THE open public API (§5.8.2). Record a pre-built typed [`DiagEvent`] from ANY module without a
/// handle. Non-blocking, panic-free, and a silent no-op on the ring side when no writer is installed.
#[inline]
pub fn record(event: DiagEvent) {
    global().record(event);
}

/// Ergonomic helper so call sites do not hand-roll a [`DiagEvent`]. Builds a generic typed event from
/// a closed [`DiagEventCode`] + [`DiagPhase`] + [`DiagSeverity`] plus the typed integer counters, then
/// records it. The argument list is all typed primitives — there is NO free-text/blob parameter, which
/// is what keeps the typed-allowlist invariant (§5.8.3) at the API boundary (AC-002-5).
///
/// `counter_a` / `counter_b` / `metric_micros` carry the numeric payload (meaning is per `code`, as
/// documented on the MT-081 constructors). `thread_id` / `sequence_id` are opaque numeric identifiers.
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn record_with(
    code: DiagEventCode,
    phase: DiagPhase,
    severity: DiagSeverity,
    thread_id: u64,
    sequence_id: u64,
    counter_a: u64,
    counter_b: u64,
    metric_micros: u64,
    timestamp_nanos: u64,
) {
    let event = DiagEvent::generic(
        code,
        phase,
        severity,
        thread_id,
        sequence_id,
        counter_a,
        counter_b,
        metric_micros,
        timestamp_nanos,
    );
    record(event);
}

/// THE per-frame liveness producer (§5.8.2 UI-thread heartbeat). Publish the heartbeat slot of the
/// MT-081 ring from the UI thread, called every egui frame by [`crate::app::HandshakeApp::update`]
/// (MT-084). Forwards to [`DiagRingWriter::write_heartbeat`] — a single wait-free, allocation-free
/// seqlock store of two integers into the mapped ring header — so a stalled UI thread stops advancing
/// the counter and the staleness is observable out-of-process by Palmistry (Tier 3) with ZERO
/// cooperation. A silent no-op when no ring writer is installed (headless/test path; AC-004-5).
///
/// `counter` is the monotonic frame counter; `timestamp_nanos` MUST be a MONOTONIC source (a
/// process-start [`std::time::Instant`] elapsed in nanos) so it never goes backward on a wall-clock
/// change — the staleness threshold (MT-091) compares this value (AC-004-2).
#[inline]
pub fn heartbeat(counter: u64, timestamp_nanos: u64) {
    global().heartbeat(counter, timestamp_nanos);
}

/// A snapshot of up to the last `n` recorded events (oldest-first) — what the Diagnostics Panel
/// (MT-087) reads. Works whether or not a ring writer is installed.
#[inline]
pub fn snapshot_last_n(n: usize) -> Vec<DiagEvent> {
    global().snapshot_last_n(n)
}

/// The number of events shed because the in-process buffer was at [`BUFFER_CAP`] — what the panel
/// shows so a dropped event is visible, not silently lost.
#[inline]
pub fn dropped_count() -> u64 {
    global().dropped_count()
}

/// Whether a ring writer has been installed on the global recorder (i.e. Palmistry can see events).
/// Used by tests + the install site's logging; not a hot-path API.
#[inline]
pub fn has_ring_writer() -> bool {
    RECORDER.get().map(|r| r.writer.is_some()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    //! In-crate unit tests for the hot-path invariants that do NOT need a real ring (graceful
    //! degradation, bounded buffer, dropped accounting, panic-free under contention). The
    //! ring-read-back + live-consumer proofs live in `tests/test_internal_diagnostics.rs` because they
    //! need a real `DiagRingWriter`/`DiagRingReader` pair and the real `HandshakeApp` startup path
    //! (which would re-init the same process-global and collide across in-crate unit tests).

    use super::*;

    /// A throwaway recorder instance (NOT the global) so unit tests can exercise the buffer/drop logic
    /// deterministically without racing the process-global `OnceLock` that the integration tests own.
    fn local_recorder() -> DiagnosticsRecorder {
        DiagnosticsRecorder::new(None)
    }

    fn marker(seq: u64) -> DiagEvent {
        DiagEvent::generic(
            DiagEventCode::Other,
            DiagPhase::Tick,
            DiagSeverity::Info,
            0,
            seq,
            0,
            0,
            0,
            seq,
        )
    }

    #[test]
    fn buffers_in_process_with_no_writer() {
        let rec = local_recorder();
        rec.record(marker(1));
        rec.record(marker(2));
        let snap = rec.snapshot_last_n(10);
        assert_eq!(snap.len(), 2, "both events buffered in-process without a writer");
        assert_eq!(snap[0].sequence_id, 1);
        assert_eq!(snap[1].sequence_id, 2);
        assert_eq!(rec.dropped_count(), 0, "nothing dropped below cap");
    }

    #[test]
    fn buffer_stays_bounded_and_counts_drops() {
        let rec = local_recorder();
        let total = (BUFFER_CAP + 100) as u64;
        for i in 0..total {
            rec.record(marker(i));
        }
        let snap = rec.snapshot_last_n(BUFFER_CAP * 2);
        assert_eq!(snap.len(), BUFFER_CAP, "buffer never exceeds the cap");
        // The oldest 100 were dropped; the surviving window is the most-recent BUFFER_CAP.
        assert_eq!(rec.dropped_count(), 100, "every shed event is accounted");
        assert_eq!(
            snap.first().unwrap().sequence_id,
            total - BUFFER_CAP as u64,
            "the window is the most-recent events (drop-oldest)"
        );
        assert_eq!(snap.last().unwrap().sequence_id, total - 1);
    }

    #[test]
    fn snapshot_last_n_clamps_to_available() {
        let rec = local_recorder();
        for i in 0..5 {
            rec.record(marker(i));
        }
        assert_eq!(rec.snapshot_last_n(3).len(), 3);
        assert_eq!(rec.snapshot_last_n(100).len(), 5);
        assert_eq!(rec.snapshot_last_n(0).len(), 0);
    }
}
