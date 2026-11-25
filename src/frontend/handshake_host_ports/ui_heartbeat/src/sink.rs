//! Where a beat is written: the heartbeat slot of the existing diagnostics ring, through its existing typed API.

use handshake_diag_ring::ring::DiagRingWriterV2;
use handshake_diag_ring::{DiagRingWriter, Heartbeat};

/// Writes one beat. Must not block, allocate or panic: it runs on the UI thread.
///
/// A beat is two integers. There is no field for content, names or paths, here or in the ring.
pub trait HeartbeatSink {
    /// Publish `beat` as the current heartbeat.
    fn publish(&self, beat: Heartbeat);
}

impl HeartbeatSink for DiagRingWriter {
    fn publish(&self, beat: Heartbeat) {
        self.write_heartbeat(beat.counter, beat.timestamp_nanos);
    }
}

impl HeartbeatSink for DiagRingWriterV2 {
    fn publish(&self, beat: Heartbeat) {
        self.write_heartbeat(beat.counter, beat.timestamp_nanos);
    }
}

/// `None` is a host without a ring: the port runs and stays silent.
impl<S: HeartbeatSink> HeartbeatSink for Option<S> {
    fn publish(&self, beat: Heartbeat) {
        if let Some(sink) = self {
            sink.publish(beat);
        }
    }
}

/// The host's diagnostics recorder, which owns the process's one ring writer.
///
/// Wraps its existing `fn heartbeat(counter: u64, timestamp_nanos: u64)`. The recorder stays silent when it has no
/// writer.
#[derive(Clone, Copy, Debug)]
pub struct RecorderHeartbeat(fn(u64, u64));

impl RecorderHeartbeat {
    /// `heartbeat` takes the counter first and the monotonic nanoseconds second.
    pub fn new(heartbeat: fn(u64, u64)) -> Self {
        Self(heartbeat)
    }
}

impl HeartbeatSink for RecorderHeartbeat {
    fn publish(&self, beat: Heartbeat) {
        (self.0)(beat.counter, beat.timestamp_nanos);
    }
}
