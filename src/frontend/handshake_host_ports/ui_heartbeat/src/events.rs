//! What the port records in the ring besides the beat, through the existing typed allowlist (MT-215 AC215-10).
//!
//! | Outcome | `DiagEventCode` | `DiagPhase` | `DiagSeverity` | `counter_a` | `metric_micros` |
//! | --- | --- | --- | --- | --- | --- |
//! | [`HeartbeatEvent::PresentStalled`] | `SlowFrame` | `Degraded` | `Warn` | last beat's counter | how long the frame waited |
//! | [`HeartbeatEvent::PresentRecovered`] | `SlowFrame` | `Recovered` | `Info` | last beat's counter | longest wait seen |
//!
//! A frame wanted for [`crate::PRESENT_STALL`] without a present is a frame over its budget, which is what `SlowFrame`
//! names; the phase tells it from the frame-time monitor's `Tick`. Both are ordinary survivable codes, so they reach
//! Palmistry's survivor record as well as the internal diagnostics. No other code was needed, and none was added.

use std::time::Duration;

use handshake_diag_ring::{DiagEvent, DiagEventCode, DiagPhase, DiagSeverity};

/// A typed outcome of the port for the diagnostics ring. Integers only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeartbeatEvent {
    /// A frame had been wanted for [`crate::PRESENT_STALL`] without a present: beats are withheld from here on.
    PresentStalled {
        /// The counter of the last beat published before.
        last_counter: u64,
        /// How long the frame had waited.
        waited: Duration,
    },
    /// No frame waits that long any more: beats are published again.
    PresentRecovered {
        /// The counter of the last beat published before the stall.
        last_counter: u64,
        /// The longest wait seen during the stall.
        waited: Duration,
    },
}

impl HeartbeatEvent {
    /// The ring record, with the port's own sequence number and monotonic time.
    pub fn to_diag_event(self, sequence_id: u64, timestamp_nanos: u64) -> DiagEvent {
        let (phase, severity, last_counter, waited) = match self {
            Self::PresentStalled {
                last_counter,
                waited,
            } => (DiagPhase::Degraded, DiagSeverity::Warn, last_counter, waited),
            Self::PresentRecovered {
                last_counter,
                waited,
            } => (DiagPhase::Recovered, DiagSeverity::Info, last_counter, waited),
        };
        DiagEvent::generic(
            DiagEventCode::SlowFrame,
            phase,
            severity,
            0,
            sequence_id,
            last_counter,
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
        let stalled = HeartbeatEvent::PresentStalled {
            last_counter: 41,
            waited,
        }
        .to_diag_event(1, 9);
        let recovered = HeartbeatEvent::PresentRecovered {
            last_counter: 41,
            waited,
        }
        .to_diag_event(2, 10);
        for event in [stalled, recovered] {
            assert_eq!(event.event_code, DiagEventCode::SlowFrame.as_u16());
            assert!(event.is_survivable_ordinary());
            assert_eq!((event.counter_a, event.metric_micros), (41, 2_500_000));
        }
        assert_eq!(stalled.phase_marker, DiagPhase::Degraded.as_u8());
        assert_eq!(stalled.severity, DiagSeverity::Warn.as_u8());
        assert_eq!(recovered.phase_marker, DiagPhase::Recovered.as_u8());
        assert_eq!((stalled.sequence_id, recovered.timestamp_nanos), (1, 10));
    }
}
