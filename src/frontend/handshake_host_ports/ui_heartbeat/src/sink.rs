//! Where a beat is written: the heartbeat slot of the existing diagnostics ring, through its existing typed API.

use handshake_diag_ring::ring::DiagRingWriterV2;
use handshake_diag_ring::schema::DiagEventV2;
use handshake_diag_ring::{DiagEvent, DiagRingWriter, Heartbeat};

/// The port's leave to write, lent to a sink for one call (MT-215 E3).
///
/// Only [`crate::UiHeartbeat`] makes one, and it lends it by reference, so a sink is reached through
/// [`HeartbeatSink`] from inside a call of the port and from nowhere else: no other code can publish a beat or record an
/// event through a sink, whatever sink it holds. Neither `Clone` nor `Copy`.
///
/// ```compile_fail,E0624
/// let token = ui_heartbeat::PortToken::new();
/// ```
///
/// ```compile_fail,E0061
/// fn second_publisher<S: ui_heartbeat::HeartbeatSink>(sink: &S) {
///     sink.publish(ui_heartbeat::Heartbeat { counter: 1, timestamp_nanos: 1 });
/// }
/// ```
///
/// Nothing else makes one either (MT-215 N3: adding `Default`, `Clone` or a public field broke no test before these
/// three). No `Default`:
///
/// ```compile_fail,E0599
/// let token = ui_heartbeat::PortToken::default();
/// ```
///
/// No copy of a lent token:
///
/// ```compile_fail,E0308
/// fn copy(token: &ui_heartbeat::PortToken) -> ui_heartbeat::PortToken {
///     token.clone()
/// }
/// ```
///
/// No struct literal:
///
/// ```compile_fail,E0451
/// let token = ui_heartbeat::PortToken { _sealed: () };
/// ```
#[derive(Debug)]
pub struct PortToken {
    _sealed: (),
}

impl PortToken {
    pub(crate) const fn new() -> Self {
        Self { _sealed: () }
    }
}

/// Writes one beat. Must not block, allocate or panic: it runs on the UI thread.
///
/// A beat is two integers. There is no field for content, names or paths, here or in the ring.
///
/// # One publisher (MT-215 HB-7, E3)
///
/// The ring's heartbeat slot takes one writer: two that race can leave its sequence odd, and the watcher then reads
/// no heartbeat at all. So a ring writer given to the port is the port's, by value: the implementations here are for
/// owned writers, not for references or shared handles, and nothing else in the process may write the slot. Each call
/// needs a [`PortToken`], which only the port makes, so nothing but the port calls a sink. A host whose ring has other
/// producers on other threads gives the port its serialized recorder ([`RecorderHeartbeat`]).
pub trait HeartbeatSink {
    /// Publish `beat` as the current heartbeat.
    fn publish(&self, beat: Heartbeat, token: &PortToken);

    /// Record a typed event about the heartbeat in the same ring: a present stall began or ended. Rare, never per beat.
    /// The default records nothing.
    fn record(&self, event: DiagEvent, token: &PortToken) {
        let _ = (event, token);
    }
}

impl HeartbeatSink for DiagRingWriter {
    fn publish(&self, beat: Heartbeat, _: &PortToken) {
        self.write_heartbeat(beat.counter, beat.timestamp_nanos);
    }

    fn record(&self, event: DiagEvent, _: &PortToken) {
        self.write(event);
    }
}

impl HeartbeatSink for DiagRingWriterV2 {
    fn publish(&self, beat: Heartbeat, _: &PortToken) {
        self.write_heartbeat(beat.counter, beat.timestamp_nanos);
    }

    fn record(&self, event: DiagEvent, _: &PortToken) {
        if let Some(event) = DiagEventV2::ordinary(event) {
            let _ = self.write(event);
        }
    }
}

/// `None` is a host without a ring: the port runs and stays silent.
impl<S: HeartbeatSink> HeartbeatSink for Option<S> {
    fn publish(&self, beat: Heartbeat, token: &PortToken) {
        if let Some(sink) = self {
            sink.publish(beat, token);
        }
    }

    fn record(&self, event: DiagEvent, token: &PortToken) {
        if let Some(sink) = self {
            sink.record(event, token);
        }
    }
}

/// The host's diagnostics recorder, which owns the process's one ring writer.
///
/// Wraps its existing free functions: `fn heartbeat(counter: u64, timestamp_nanos: u64)`, `fn has_ring_writer() ->
/// bool` and, optionally, `fn record(event: DiagEvent)`. Neither a beat nor an event reaches them until
/// `has_ring_writer` says the writer is installed (MT-215 F2): the recorder creates itself on its first call, without
/// a writer if none is installed yet, and can never take one afterwards; that first call also allocates. So a beat
/// that comes before the install is withheld, not lost to the ring for the whole session, and the next beat after the
/// install is written.
///
/// `has_ring_writer` must only look: it must not create the recorder (the native host's reads its `OnceLock` with
/// `get`, which does not). Neither `Clone` nor `Copy`: one port per recorder.
#[derive(Debug)]
pub struct RecorderHeartbeat {
    heartbeat: fn(u64, u64),
    has_ring_writer: fn() -> bool,
    record: Option<fn(DiagEvent)>,
}

impl RecorderHeartbeat {
    /// `heartbeat` takes the counter first and the monotonic nanoseconds second; `has_ring_writer` says whether the
    /// recorder has its ring writer, without creating the recorder.
    pub fn new(heartbeat: fn(u64, u64), has_ring_writer: fn() -> bool) -> Self {
        Self {
            heartbeat,
            has_ring_writer,
            record: None,
        }
    }

    /// Also record the port's typed events through the recorder's `record`.
    pub fn recording(self, record: fn(DiagEvent)) -> Self {
        Self {
            record: Some(record),
            ..self
        }
    }
}

impl HeartbeatSink for RecorderHeartbeat {
    fn publish(&self, beat: Heartbeat, _: &PortToken) {
        if (self.has_ring_writer)() {
            (self.heartbeat)(beat.counter, beat.timestamp_nanos);
        }
    }

    fn record(&self, event: DiagEvent, _: &PortToken) {
        if let Some(record) = self.record {
            if (self.has_ring_writer)() {
                record(event);
            }
        }
    }
}
