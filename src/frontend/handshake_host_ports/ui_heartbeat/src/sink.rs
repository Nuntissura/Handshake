//! Where a beat is written: the heartbeat slot of the existing diagnostics ring, through its existing typed API.

use handshake_diag_ring::ring::DiagRingWriterV2;
use handshake_diag_ring::schema::DiagEventV2;
use handshake_diag_ring::{DiagEvent, DiagRingWriter, Heartbeat};

/// Writes one beat. Must not block, allocate or panic: it runs on the UI thread.
///
/// A beat is two integers. There is no field for content, names or paths, here or in the ring.
///
/// # One publisher (MT-215 HB-7)
///
/// The ring's heartbeat slot takes one writer: two that race can leave its sequence odd, and the watcher then reads
/// no heartbeat at all. So a ring writer given to the port is the port's, by value: the implementations here are for
/// owned writers, not for references or shared handles, and nothing else in the process may write the slot. A host
/// whose ring has other producers on other threads gives the port its serialized recorder ([`RecorderHeartbeat`]).
pub trait HeartbeatSink {
    /// Publish `beat` as the current heartbeat.
    fn publish(&self, beat: Heartbeat);

    /// Record a typed event about the heartbeat in the same ring: a present stall began or ended. Rare, never per beat.
    /// The default records nothing.
    fn record(&self, event: DiagEvent) {
        let _ = event;
    }
}

impl HeartbeatSink for DiagRingWriter {
    fn publish(&self, beat: Heartbeat) {
        self.write_heartbeat(beat.counter, beat.timestamp_nanos);
    }

    fn record(&self, event: DiagEvent) {
        self.write(event);
    }
}

impl HeartbeatSink for DiagRingWriterV2 {
    fn publish(&self, beat: Heartbeat) {
        self.write_heartbeat(beat.counter, beat.timestamp_nanos);
    }

    fn record(&self, event: DiagEvent) {
        if let Some(event) = DiagEventV2::ordinary(event) {
            let _ = self.write(event);
        }
    }
}

/// `None` is a host without a ring: the port runs and stays silent.
impl<S: HeartbeatSink> HeartbeatSink for Option<S> {
    fn publish(&self, beat: Heartbeat) {
        if let Some(sink) = self {
            sink.publish(beat);
        }
    }

    fn record(&self, event: DiagEvent) {
        if let Some(sink) = self {
            sink.record(event);
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
    fn publish(&self, beat: Heartbeat) {
        if (self.has_ring_writer)() {
            (self.heartbeat)(beat.counter, beat.timestamp_nanos);
        }
    }

    fn record(&self, event: DiagEvent) {
        if let Some(record) = self.record {
            if (self.has_ring_writer)() {
                record(event);
            }
        }
    }
}
