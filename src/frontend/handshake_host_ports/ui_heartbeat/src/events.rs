//! What the port records in the ring besides the beat, through the existing typed allowlist (MT-215 AC215-10).
//!
//! | Outcome | `DiagEventCode` | `DiagPhase` | `DiagSeverity` | `counter_a` | `counter_b` | `metric_micros` |
//! | --- | --- | --- | --- | --- | --- | --- |
//! | [`HeartbeatEvent::PresentStalled`] | `SlowFrame` | `Degraded` | `Warn` | 0 | 0 | how long the frame has waited |
//! | [`HeartbeatEvent::PresentRecovered`] | `SlowFrame` | `Recovered` | `Info` | 0 | 0 | the longest wait seen |
//!
//! A frame wanted for [`crate::PRESENT_STALL`] without a present is a frame over its budget, which is what `SlowFrame`
//! names, and `metric_micros` is its frame time so far; the phase tells it from the frame-time monitor's `Tick`. Only
//! fields whose documented meaning matches are filled (MT-215 E4): `SlowFrame`'s `counter_a` is a frame index, which
//! the port does not know, so it stays 0. Both are ordinary survivable codes, so they reach Palmistry's survivor record
//! as well as the internal diagnostics.
//!
//! A dedicated typed code for a present stall, with the beat counter in a field of its own, is requested from the
//! diagnostics ring's owner (AC215-10: a missing code is requested, never worked around); the request is
//! `Handshake_Artifacts/WP-KERNEL-012/MT-215/gui-agent/diag-ring-typed-addition-request.json`. Until it is accepted
//! the outcomes keep `SlowFrame`'s documented payload.

use std::time::Duration;

use handshake_diag_ring::{DiagEvent, DiagEventCode, DiagPhase, DiagSeverity};

/// A typed outcome of the port for the diagnostics ring. Integers only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeartbeatEvent {
    /// A frame had been wanted for [`crate::PRESENT_STALL`] without a present. Beats go on (AC215-6).
    PresentStalled {
        /// How long the frame had waited.
        waited: Duration,
    },
    /// No frame waits that long any more.
    PresentRecovered {
        /// The longest wait seen during the stall.
        waited: Duration,
    },
}

impl HeartbeatEvent {
    /// The ring record, with the port's own sequence number and monotonic time.
    pub fn to_diag_event(self, sequence_id: u64, timestamp_nanos: u64) -> DiagEvent {
        let (phase, severity, waited) = match self {
            Self::PresentStalled { waited } => (DiagPhase::Degraded, DiagSeverity::Warn, waited),
            Self::PresentRecovered { waited } => (DiagPhase::Recovered, DiagSeverity::Info, waited),
        };
        DiagEvent::generic(
            DiagEventCode::SlowFrame,
            phase,
            severity,
            0,
            sequence_id,
            // SlowFrame's frame index: the port knows none.
            0,
            0,
            u64::try_from(waited.as_micros()).unwrap_or(u64::MAX),
            timestamp_nanos,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_outcomes_are_survivable_slow_frames_told_apart_by_phase() {
        let waited = Duration::from_millis(2_500);
        let stalled = HeartbeatEvent::PresentStalled { waited }.to_diag_event(1, 9);
        let recovered = HeartbeatEvent::PresentRecovered { waited }.to_diag_event(2, 10);
        for event in [stalled, recovered] {
            assert_eq!(event.event_code, DiagEventCode::SlowFrame.as_u16());
            assert!(event.is_survivable_ordinary());
            assert_eq!(event.metric_micros, 2_500_000);
            assert_eq!(
                (event.counter_a, event.counter_b),
                (0, 0),
                "SlowFrame's counter_a is a frame index, which the port does not know (MT-215 E4)"
            );
        }
        assert_eq!(stalled.phase_marker, DiagPhase::Degraded.as_u8());
        assert_eq!(stalled.severity, DiagSeverity::Warn.as_u8());
        assert_eq!(recovered.phase_marker, DiagPhase::Recovered.as_u8());
        assert_eq!((stalled.sequence_id, recovered.timestamp_nanos), (1, 10));
    }
}
