//! Passive child-process stall detection (WP-KERNEL-012 MT-106).
//!
//! Palmistry already watches Handshake's own liveness through the shared diagnostic ring. This module
//! adds the same "observe, never ask" rule for spawned children: a child is considered stalled only when
//! Palmistry can see that the process is still alive AND a passive progress source stopped advancing past
//! a bounded deadline. Missing progress before the first baseline is not a stall.

use std::time::{Duration, Instant};

/// Poll cadence for child-process stall detection. Short enough to catch a hang promptly, slow enough to
/// stay quiet in the background.
pub const CHILD_STALL_POLL_INTERVAL: Duration = Duration::from_millis(CHILD_STALL_POLL_INTERVAL_MS);
pub const CHILD_STALL_POLL_INTERVAL_MS: u64 = 300;
/// Default no-progress threshold for a watched child.
pub const CHILD_STALL_THRESHOLD: Duration = Duration::from_millis(CHILD_STALL_THRESHOLD_MS);
pub const CHILD_STALL_THRESHOLD_MS: u64 = 5_000;

const _: () = assert!(CHILD_STALL_POLL_INTERVAL_MS >= 200);
const _: () = assert!(CHILD_STALL_POLL_INTERVAL_MS <= 500);
const _: () = assert!(CHILD_STALL_POLL_INTERVAL_MS < CHILD_STALL_THRESHOLD_MS);

/// Process-liveness verdict supplied by the OS process probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildProcessState {
    /// The process handle is signaled as still running.
    Alive,
    /// The process has exited and should be removed from the watch registry.
    Exited,
    /// The process state could not be proven. A stale progress source in this state is only suspected,
    /// never confirmed, because MT-106 requires process-alive + no-progress for a `ChildStall`.
    Unknown,
}

/// One passive progress reading from a child liveness source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildProgress {
    /// Monotonic-ish counter published by the child or its launcher-side adapter.
    pub counter: u64,
    /// Source timestamp in nanoseconds where available. This is evidence only; advancement is keyed by
    /// the counter so a rewritten same-value file does not mask a stall.
    pub timestamp_nanos: u64,
}

/// Stable reason code for a durable `ChildStall` survivor record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ChildStallReasonCode {
    /// The child process was alive while its passive progress counter stayed stale past the threshold.
    ProgressStaleWhileAlive = 1,
}

impl ChildStallReasonCode {
    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

/// Durable child-stall evidence emitted on the healthy -> stalled edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildStallReport {
    pub child_pid: u32,
    pub child_session_id: u64,
    pub stale_ms: u64,
    pub last_progress_counter: u64,
    pub last_progress_ts_nanos: u64,
    pub reason_code: ChildStallReasonCode,
}

/// Current detector state after one poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChildStallState {
    Healthy,
    Suspected { stale_ms: u64 },
    Stalled(ChildStallReport),
    Exited,
}

/// Result of one detector poll. `report` is populated only on a new confirmed-stall edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildStallPoll {
    pub state: ChildStallState,
    pub report: Option<ChildStallReport>,
}

/// Per-child edge detector.
pub struct ChildStallDetector {
    child_pid: u32,
    child_session_id: u64,
    threshold: Duration,
    last_counter: Option<u64>,
    last_progress_ts_nanos: u64,
    last_advance: Option<Instant>,
    reported_current_stall: bool,
}

impl ChildStallDetector {
    pub fn new(child_pid: u32, child_session_id: u64, threshold: Duration) -> Self {
        Self {
            child_pid,
            child_session_id,
            threshold,
            last_counter: None,
            last_progress_ts_nanos: 0,
            last_advance: None,
            reported_current_stall: false,
        }
    }

    pub fn poll(
        &mut self,
        now: Instant,
        process_state: ChildProcessState,
        progress: Option<ChildProgress>,
    ) -> ChildStallPoll {
        if process_state == ChildProcessState::Exited {
            self.reported_current_stall = false;
            return ChildStallPoll {
                state: ChildStallState::Exited,
                report: None,
            };
        }

        let source_available = progress.is_some();
        if let Some(progress) = progress {
            let advanced = self.last_counter != Some(progress.counter);
            if advanced {
                self.last_counter = Some(progress.counter);
                self.last_progress_ts_nanos = progress.timestamp_nanos;
                self.last_advance = Some(now);
                self.reported_current_stall = false;
                return ChildStallPoll {
                    state: ChildStallState::Healthy,
                    report: None,
                };
            }
        }

        let Some(last_advance) = self.last_advance else {
            return ChildStallPoll {
                state: ChildStallState::Healthy,
                report: None,
            };
        };

        let stale = now.checked_duration_since(last_advance).unwrap_or_default();
        if stale < self.threshold {
            return ChildStallPoll {
                state: ChildStallState::Healthy,
                report: None,
            };
        }

        let stale_ms = stale.as_millis() as u64;
        if process_state != ChildProcessState::Alive || !source_available {
            return ChildStallPoll {
                state: ChildStallState::Suspected { stale_ms },
                report: None,
            };
        }

        let report = ChildStallReport {
            child_pid: self.child_pid,
            child_session_id: self.child_session_id,
            stale_ms,
            last_progress_counter: self.last_counter.unwrap_or(0),
            last_progress_ts_nanos: self.last_progress_ts_nanos,
            reason_code: ChildStallReasonCode::ProgressStaleWhileAlive,
        };
        let edge = if self.reported_current_stall {
            None
        } else {
            self.reported_current_stall = true;
            Some(report.clone())
        };
        ChildStallPoll {
            state: ChildStallState::Stalled(report),
            report: edge,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress(counter: u64) -> ChildProgress {
        ChildProgress {
            counter,
            timestamp_nanos: counter * 10,
        }
    }

    #[test]
    fn missing_baseline_never_confirms_a_stall() {
        let start = Instant::now();
        let mut detector = ChildStallDetector::new(42, 7, Duration::from_millis(100));
        let poll = detector.poll(
            start + Duration::from_millis(500),
            ChildProcessState::Alive,
            None,
        );
        assert_eq!(poll.state, ChildStallState::Healthy);
        assert!(poll.report.is_none());
    }

    #[test]
    fn stale_progress_with_alive_process_reports_once() {
        let start = Instant::now();
        let mut detector = ChildStallDetector::new(42, 7, Duration::from_millis(100));
        assert!(detector
            .poll(start, ChildProcessState::Alive, Some(progress(1)))
            .report
            .is_none());

        let first = detector.poll(
            start + Duration::from_millis(150),
            ChildProcessState::Alive,
            Some(progress(1)),
        );
        assert!(first.report.is_some(), "first stale edge emits a report");
        assert!(matches!(first.state, ChildStallState::Stalled(_)));

        let second = detector.poll(
            start + Duration::from_millis(300),
            ChildProcessState::Alive,
            Some(progress(1)),
        );
        assert!(second.report.is_none(), "same stale edge is debounced");
    }

    #[test]
    fn unknown_process_state_is_only_suspected() {
        let start = Instant::now();
        let mut detector = ChildStallDetector::new(42, 7, Duration::from_millis(100));
        detector.poll(start, ChildProcessState::Alive, Some(progress(1)));
        let poll = detector.poll(
            start + Duration::from_millis(150),
            ChildProcessState::Unknown,
            Some(progress(1)),
        );
        assert_eq!(poll.state, ChildStallState::Suspected { stale_ms: 150 });
        assert!(poll.report.is_none());
    }

    #[test]
    fn unavailable_source_after_baseline_is_only_suspected() {
        let start = Instant::now();
        let mut detector = ChildStallDetector::new(42, 7, Duration::from_millis(100));
        detector.poll(start, ChildProcessState::Alive, Some(progress(1)));
        let poll = detector.poll(
            start + Duration::from_millis(150),
            ChildProcessState::Alive,
            None,
        );
        assert_eq!(poll.state, ChildStallState::Suspected { stale_ms: 150 });
        assert!(poll.report.is_none());
    }

    #[test]
    fn progress_after_stall_recovers_and_can_report_again() {
        let start = Instant::now();
        let mut detector = ChildStallDetector::new(42, 7, Duration::from_millis(100));
        detector.poll(start, ChildProcessState::Alive, Some(progress(1)));
        assert!(detector
            .poll(
                start + Duration::from_millis(150),
                ChildProcessState::Alive,
                Some(progress(1)),
            )
            .report
            .is_some());
        assert!(detector
            .poll(
                start + Duration::from_millis(160),
                ChildProcessState::Alive,
                Some(progress(2)),
            )
            .report
            .is_none());
        assert!(detector
            .poll(
                start + Duration::from_millis(300),
                ChildProcessState::Alive,
                Some(progress(2)),
            )
            .report
            .is_some());
    }
}
