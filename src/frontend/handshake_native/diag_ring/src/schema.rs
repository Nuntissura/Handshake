//! Typed-allowlist `DiagEvent` record schema — the HARD privacy invariant of the diagnostic
//! substrate (Master Spec v02.196 §5.8.3 + §6.13.8).
//!
//! # The allowlist invariant (why there is no text field)
//!
//! A diagnostic event MUST carry NO project content, NO sensitive data, NO free text, NO file
//! contents, NO prompts, NO document bodies. Every field on [`DiagEvent`] is a mechanical
//! fixed-width primitive: an event code from a CLOSED enum, two small enum markers, and a handful
//! of `u64` counters/metrics. There is *no* `String`, `Vec`, `&str`, or `[u8]` blob field, so a
//! caller physically cannot smuggle arbitrary text through this record. Adding a new kind of event
//! means adding a new variant to [`DiagEventCode`] — a code change reviewed against the allowlist —
//! not stuffing a string into a payload.
//!
//! # Compile-time enforcement (lean on bytemuck)
//!
//! [`DiagEvent`] derives [`bytemuck::Pod`] + [`bytemuck::Zeroable`]. bytemuck's derive REFUSES to
//! compile a `Pod` type that contains any non-POD field (a `String`/`Vec`/`&str`/pointer is not
//! `Pod`), so the allowlist is enforced by the *type system*: if someone adds a `String` field to
//! `DiagEvent`, the crate stops compiling. `Pod` additionally forbids implicit padding bytes, which
//! is why the struct carries an explicit `_reserved` field (see the layout note below) — this also
//! makes the record ABI-stable for the cross-process shared-memory map.
//!
//! # Layout (ABI-stable, `#[repr(C)]`, zero implicit padding)
//!
//! ```text
//! offset  size  field
//!      0     2  event_code     (u16)
//!      2     1  phase_marker   (u8)
//!      3     1  severity       (u8)
//!      4     4  _reserved      ([u8; 4])  <- fills the gap before the 8-aligned u64s; NOT content
//!      8     8  thread_id      (u64)
//!     16     8  sequence_id    (u64)
//!     24     8  counter_a      (u64)
//!     32     8  counter_b      (u64)
//!     40     8  metric_micros  (u64)
//!     48     8  timestamp_nanos(u64)
//! total = 56 bytes, align = 8
//! ```

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

/// Fixed, compiled-in size of a [`DiagEvent`] in bytes. Asserted at compile time below and in the
/// allowlist test; the cross-process reader validates `record_size` in the ring header against the
/// size of its own compiled `RecordSlot` so a layout drift is refused rather than read as garbage.
pub const DIAG_EVENT_SIZE: usize = 56;

/// Closed set of diagnostic event kinds. `#[repr(u16)]` so the discriminant stores into the
/// `event_code` field exactly. A caller cannot invent a code outside this enum at the typed API
/// boundary; the raw `u16` in the POD record is only ever produced from one of these variants via
/// the constructors on [`DiagEvent`].
///
/// Adding a variant is a deliberate, reviewed code change — this is the allowlist gate. New
/// discriminants MUST be appended with explicit values so existing codes never shift (forward/
/// backward compat for the shared map across `version` bumps, per WP-016).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum DiagEventCode {
    /// UI-thread heartbeat tick (MT-084 writes this every egui frame).
    Heartbeat = 0,
    /// A panic was caught by the panic hook (MT-083).
    PanicCaught = 1,
    /// A frame exceeded the slow-frame budget (frame-time monitor).
    SlowFrame = 2,
    /// A periodic CPU/RSS/GPU resource sample.
    ResourceSample = 3,
    /// The backend became unreachable (the motivating 2026-06-26 freeze incident).
    BackendUnreachable = 4,
    /// The backend recovered after being unreachable.
    BackendRecovered = 5,
    /// A pane was mounted in the shell.
    PaneMounted = 6,
    /// The external watcher suspects a freeze (heartbeat stalled) — written by Palmistry's view,
    /// reserved here so the code space is shared and stable.
    FreezeSuspected = 7,
    /// A crash was detected (process gone / minidump produced) — Palmistry side.
    CrashDetected = 8,
    /// Palmistry completed its handshake with the ring (MT-090).
    PalmistryHandshake = 9,
    /// Orderly shutdown marker.
    Shutdown = 10,
    /// A deadline-bounded in-app operation stopped making progress (MT-105 operation watchdog).
    StalledOperation = 11,
    /// Escape hatch for an event that does not fit a named code. Still carries NO text — only the
    /// numeric counters — so it cannot become a free-text smuggling channel.
    Other = 65535,
}

impl DiagEventCode {
    /// Raw `u16` discriminant as stored in the POD record's `event_code` field.
    #[inline]
    pub const fn as_u16(self) -> u16 {
        self as u16
    }
}

/// Coarse phase marker for an event (where in a lifecycle the event sits). Small closed enum; the
/// raw `u8` is only ever produced from one of these variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DiagPhase {
    /// Beginning of a tracked span/operation.
    Start = 0,
    /// A periodic tick within a span (e.g. heartbeat, resource sample).
    Tick = 1,
    /// End of a tracked span/operation.
    End = 2,
    /// A previously-degraded condition recovered.
    Recovered = 3,
    /// A degraded condition was entered.
    Degraded = 4,
}

impl DiagPhase {
    /// Raw `u8` as stored in the record's `phase_marker` field.
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Severity of an event. Small closed enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DiagSeverity {
    /// Informational, expected.
    Info = 0,
    /// Warning, recoverable / non-fatal.
    Warn = 1,
    /// Error / fatal condition.
    Error = 2,
}

impl DiagSeverity {
    /// Raw `u8` as stored in the record's `severity` field.
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// The typed-allowlist diagnostic record. POD, `#[repr(C)]`, fixed 56-byte size (see
/// [`DIAG_EVENT_SIZE`]). Every field is a fixed-width integer; there is deliberately NO text/blob
/// field (see the module docs for the privacy invariant). Construct via the named helpers
/// ([`DiagEvent::heartbeat`], [`DiagEvent::resource_sample`], etc.) so call sites cannot hand-roll
/// a malformed event.
///
/// The integer fields are deliberately generic counters so MT-082+ can map domain quantities onto
/// them without ever introducing a string:
/// - `thread_id`     : opaque numeric thread identifier of the producer.
/// - `sequence_id`   : monotonic producer-assigned sequence number for the event.
/// - `counter_a` / `counter_b` : two general-purpose numeric payloads (meaning is per `event_code`,
///   documented at each constructor, e.g. cpu_milli / rss_kb for a resource sample).
/// - `metric_micros` : a duration/metric in microseconds (e.g. frame time).
/// - `timestamp_nanos`: a monotonic timestamp in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize)]
#[repr(C)]
pub struct DiagEvent {
    /// Numeric event kind — a [`DiagEventCode`] discriminant.
    pub event_code: u16,
    /// Coarse phase — a [`DiagPhase`] discriminant.
    pub phase_marker: u8,
    /// Severity — a [`DiagSeverity`] discriminant.
    pub severity: u8,
    /// Explicit padding so the struct has ZERO implicit padding (required for `bytemuck::Pod` and
    /// for a stable cross-process ABI). NOT a content field; always zero.
    pub _reserved: [u8; 4],
    /// Opaque numeric producer thread identifier.
    pub thread_id: u64,
    /// Monotonic producer-assigned sequence number for this event.
    pub sequence_id: u64,
    /// General-purpose numeric payload A (meaning per `event_code`).
    pub counter_a: u64,
    /// General-purpose numeric payload B (meaning per `event_code`).
    pub counter_b: u64,
    /// A metric/duration in microseconds (meaning per `event_code`).
    pub metric_micros: u64,
    /// Monotonic timestamp in nanoseconds.
    pub timestamp_nanos: u64,
}

// Compile-time guarantee that the record size never drifts (the cross-process reader relies on this
// equalling the writer's `record_size`). If a field is added/removed/reordered, this fails to
// compile until `DIAG_EVENT_SIZE` and the layout doc are updated deliberately.
const _: () = assert!(core::mem::size_of::<DiagEvent>() == DIAG_EVENT_SIZE);
// Align is 8 (driven by the u64 fields); assert it so a future #[repr] change cannot silently break
// the shared-memory alignment contract.
const _: () = assert!(core::mem::align_of::<DiagEvent>() == 8);

impl DiagEvent {
    /// Construct a fully-typed event from its parts. Internal helper used by the named constructors;
    /// keeps the `_reserved` padding zeroed in one place.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn new(
        code: DiagEventCode,
        phase: DiagPhase,
        severity: DiagSeverity,
        thread_id: u64,
        sequence_id: u64,
        counter_a: u64,
        counter_b: u64,
        metric_micros: u64,
        timestamp_nanos: u64,
    ) -> Self {
        Self {
            event_code: code.as_u16(),
            phase_marker: phase.as_u8(),
            severity: severity.as_u8(),
            _reserved: [0; 4],
            thread_id,
            sequence_id,
            counter_a,
            counter_b,
            metric_micros,
            timestamp_nanos,
        }
    }

    /// A heartbeat tick (MT-084). `counter` is the monotonic frame counter; `timestamp_nanos` the
    /// monotonic timestamp. Encoded into `counter_a` + `timestamp_nanos` so a reader can correlate.
    #[inline]
    pub fn heartbeat(thread_id: u64, sequence_id: u64, counter: u64, timestamp_nanos: u64) -> Self {
        Self::new(
            DiagEventCode::Heartbeat,
            DiagPhase::Tick,
            DiagSeverity::Info,
            thread_id,
            sequence_id,
            counter,
            0,
            0,
            timestamp_nanos,
        )
    }

    /// A periodic resource sample. `cpu_milli` = CPU usage in milli-units; `rss_kb` = resident set
    /// size in KiB; `metric_micros` = an optional accompanying duration metric.
    #[inline]
    pub fn resource_sample(
        thread_id: u64,
        sequence_id: u64,
        cpu_milli: u64,
        rss_kb: u64,
        metric_micros: u64,
        timestamp_nanos: u64,
    ) -> Self {
        Self::new(
            DiagEventCode::ResourceSample,
            DiagPhase::Tick,
            DiagSeverity::Info,
            thread_id,
            sequence_id,
            cpu_milli,
            rss_kb,
            metric_micros,
            timestamp_nanos,
        )
    }

    /// A slow-frame event. `frame_micros` is the frame time in microseconds.
    #[inline]
    pub fn slow_frame(
        thread_id: u64,
        sequence_id: u64,
        frame_index: u64,
        frame_micros: u64,
        timestamp_nanos: u64,
    ) -> Self {
        Self::new(
            DiagEventCode::SlowFrame,
            DiagPhase::Tick,
            DiagSeverity::Warn,
            thread_id,
            sequence_id,
            frame_index,
            0,
            frame_micros,
            timestamp_nanos,
        )
    }

    /// A backend-unreachable event (the motivating freeze incident). `port` carries the numeric TCP
    /// port that was unreachable (still no text).
    #[inline]
    pub fn backend_unreachable(
        thread_id: u64,
        sequence_id: u64,
        port: u64,
        timestamp_nanos: u64,
    ) -> Self {
        Self::new(
            DiagEventCode::BackendUnreachable,
            DiagPhase::Degraded,
            DiagSeverity::Error,
            thread_id,
            sequence_id,
            port,
            0,
            0,
            timestamp_nanos,
        )
    }

    /// A backend-recovered event.
    #[inline]
    pub fn backend_recovered(
        thread_id: u64,
        sequence_id: u64,
        port: u64,
        timestamp_nanos: u64,
    ) -> Self {
        Self::new(
            DiagEventCode::BackendRecovered,
            DiagPhase::Recovered,
            DiagSeverity::Info,
            thread_id,
            sequence_id,
            port,
            0,
            0,
            timestamp_nanos,
        )
    }

    /// A stalled operation event (MT-105 operation watchdog). The payload is typed integers only:
    /// `sequence_id` is the opaque operation id, `operation_code` is the closed operation enum
    /// discriminant, `last_progress_ms` is the monotonic gap since the last tick, and `elapsed_ms` is
    /// encoded in `metric_micros` as `elapsed_ms * 1000`.
    #[inline]
    pub fn stalled_operation(
        thread_id: u64,
        sequence_id: u64,
        operation_code: u64,
        elapsed_ms: u64,
        last_progress_ms: u64,
        timestamp_nanos: u64,
    ) -> Self {
        Self::new(
            DiagEventCode::StalledOperation,
            DiagPhase::Degraded,
            DiagSeverity::Error,
            thread_id,
            sequence_id,
            operation_code,
            last_progress_ms,
            elapsed_ms.saturating_mul(1000),
            timestamp_nanos,
        )
    }

    /// A generic event with an explicit code. `counter_a`/`counter_b`/`metric_micros` carry the
    /// numeric payload; still no text. Use a named constructor when one exists. The argument count
    /// mirrors the typed allowlist fields one-to-one (every field is an explicit primitive), so a
    /// flat parameter list is clearer here than a builder.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn generic(
        code: DiagEventCode,
        phase: DiagPhase,
        severity: DiagSeverity,
        thread_id: u64,
        sequence_id: u64,
        counter_a: u64,
        counter_b: u64,
        metric_micros: u64,
        timestamp_nanos: u64,
    ) -> Self {
        Self::new(
            code,
            phase,
            severity,
            thread_id,
            sequence_id,
            counter_a,
            counter_b,
            metric_micros,
            timestamp_nanos,
        )
    }
}
