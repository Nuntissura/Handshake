//! Which frames a window wants, and what became of them.
//!
//! Nothing is drawn unless something asked for it. A frame that is asked for and not presented is the sign of a loop
//! that lives and shows nothing: [`FrameProgress::waiting`] measures it, because a heartbeat cannot (MT-215 HB-4).

use std::time::Duration;

/// Names one due frame of one window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FrameTicket(pub(crate) u64);

/// Why a frame is due.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrameCause {
    /// The host asked for it.
    Requested,
    /// The operating system needs the window drawn: shown, uncovered or resized.
    Exposed,
}

/// What became of a due frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FrameOutcome {
    /// It was handed to presentation.
    Presented,
    /// Nothing had to change on screen.
    Unchanged,
    /// It could not be drawn or presented. It is still wanted: ask again when that has changed.
    Failed,
}

/// Frames of one window so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameProgress {
    /// Frames handed to the host.
    pub due: u64,
    /// Frames presented.
    pub presented: u64,
    /// Frames that were not presented because of a failure.
    pub failed: u64,
    /// How long a wanted frame has been waiting to be presented. `None` when none is wanted, and while the window
    /// cannot show one (hidden, minimised or empty).
    pub waiting: Option<Duration>,
}

/// The frames of one window.
#[derive(Debug, Default)]
pub(crate) struct FrameLedger {
    next: u64,
    /// Since when a frame is wanted and not presented.
    wanted_since: Option<u64>,
    /// The host asked for a frame since the last one became due.
    requested: bool,
    /// The frame the host has not answered yet.
    outstanding: Option<FrameTicket>,
    /// Since when another frame is wanted while one is outstanding.
    again_since: Option<u64>,
    due: u64,
    presented: u64,
    failed: u64,
}

impl FrameLedger {
    /// The host asks for a frame. Returns whether the operating system has to be asked for a paint now; when one is
    /// outstanding, that happens once it is answered.
    pub(crate) fn request(&mut self, now: u64) -> bool {
        self.requested = true;
        self.wanted_since.get_or_insert(now);
        if self.outstanding.is_some() {
            self.again_since.get_or_insert(now);
            return false;
        }
        true
    }

    /// The operating system lets the window paint. Returns the frame to hand to the host, if one is due.
    pub(crate) fn paint(&mut self, now: u64) -> Option<(FrameTicket, FrameCause)> {
        self.wanted_since.get_or_insert(now);
        if self.outstanding.is_some() {
            self.again_since.get_or_insert(now);
            return None;
        }
        let cause = if std::mem::take(&mut self.requested) {
            FrameCause::Requested
        } else {
            FrameCause::Exposed
        };
        self.next = self.next.wrapping_add(1);
        self.due = self.due.saturating_add(1);
        let ticket = FrameTicket(self.next);
        self.outstanding = Some(ticket);
        Some((ticket, cause))
    }

    /// The host says what became of a frame. Returns whether another paint is needed; `None` when it names a frame
    /// that is not outstanding.
    pub(crate) fn answer(&mut self, ticket: FrameTicket, outcome: FrameOutcome) -> Option<bool> {
        if self.outstanding != Some(ticket) {
            return None;
        }
        self.outstanding = None;
        match outcome {
            FrameOutcome::Presented => self.presented = self.presented.saturating_add(1),
            FrameOutcome::Unchanged => {}
            FrameOutcome::Failed => self.failed = self.failed.saturating_add(1),
        }
        let again_since = self.again_since.take();
        if outcome != FrameOutcome::Failed {
            self.wanted_since = again_since;
        }
        Some(again_since.is_some())
    }

    /// A frame is wanted and nothing is on its way to the host.
    pub(crate) fn wants_paint(&self) -> bool {
        self.wanted_since.is_some() && self.outstanding.is_none()
    }

    pub(crate) fn progress(&self, now: u64, can_show: bool) -> FrameProgress {
        FrameProgress {
            due: self.due,
            presented: self.presented,
            failed: self.failed,
            waiting: self
                .wanted_since
                .filter(|_| can_show)
                .map(|since| Duration::from_nanos(now.saturating_sub(since))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_due_or_waiting_until_something_asks() {
        let ledger = FrameLedger::default();
        assert!(!ledger.wants_paint());
        assert_eq!(ledger.progress(1_000, true), FrameProgress::default());
    }

    #[test]
    fn a_request_becomes_one_frame_however_often_it_is_made() {
        let mut ledger = FrameLedger::default();
        assert!(ledger.request(10));
        assert!(ledger.request(20));
        let (ticket, cause) = ledger.paint(30).unwrap();
        assert_eq!(cause, FrameCause::Requested);
        assert_eq!(ledger.paint(31), None, "one frame at a time");
        assert_eq!(ledger.answer(ticket, FrameOutcome::Presented), Some(true));
        let (_, cause) = ledger.paint(32).unwrap();
        assert_eq!(cause, FrameCause::Exposed);
    }

    #[test]
    fn a_request_during_a_frame_is_drawn_after_it() {
        let mut ledger = FrameLedger::default();
        ledger.request(10);
        let (ticket, _) = ledger.paint(11).unwrap();
        assert!(
            !ledger.request(12),
            "the paint is asked for once the frame is answered"
        );
        assert_eq!(ledger.answer(ticket, FrameOutcome::Presented), Some(true));
        assert!(ledger.wants_paint());
        assert_eq!(ledger.paint(13).unwrap().1, FrameCause::Requested);
    }

    #[test]
    fn waiting_runs_from_the_first_request_to_the_presented_frame() {
        let mut ledger = FrameLedger::default();
        ledger.request(1_000);
        ledger.request(5_000);
        let waited = |ledger: &FrameLedger, now| ledger.progress(now, true).waiting;
        assert_eq!(waited(&ledger, 9_000), Some(Duration::from_nanos(8_000)));
        let (ticket, _) = ledger.paint(9_000).unwrap();
        assert_eq!(
            waited(&ledger, 10_000),
            Some(Duration::from_nanos(9_000)),
            "handing a frame out is not presenting it"
        );
        ledger.answer(ticket, FrameOutcome::Presented);
        assert_eq!(waited(&ledger, 11_000), None);
    }

    #[test]
    fn a_presented_frame_ends_only_its_own_waiting() {
        let mut ledger = FrameLedger::default();
        ledger.request(1_000);
        let (ticket, _) = ledger.paint(2_000).unwrap();
        ledger.request(7_000);
        ledger.answer(ticket, FrameOutcome::Presented);
        assert_eq!(
            ledger.progress(9_000, true).waiting,
            Some(Duration::from_nanos(2_000)),
            "the frame asked for at 7000 is the one that waits"
        );
    }

    #[test]
    fn a_failed_frame_keeps_waiting() {
        let mut ledger = FrameLedger::default();
        ledger.request(1_000);
        let (ticket, _) = ledger.paint(2_000).unwrap();
        assert_eq!(ledger.answer(ticket, FrameOutcome::Failed), Some(false));
        let progress = ledger.progress(6_000, true);
        assert_eq!(progress.waiting, Some(Duration::from_nanos(5_000)));
        assert_eq!((progress.presented, progress.failed), (0, 1));
        assert!(ledger.wants_paint());
    }

    #[test]
    fn a_window_that_cannot_show_a_frame_is_not_waiting() {
        let mut ledger = FrameLedger::default();
        ledger.request(1_000);
        assert_eq!(ledger.progress(9_000, false).waiting, None);
        assert!(ledger.progress(9_000, true).waiting.is_some());
    }

    #[test]
    fn a_frame_is_answered_once_and_only_by_its_own_ticket() {
        let mut ledger = FrameLedger::default();
        ledger.request(1);
        let (first, _) = ledger.paint(2).unwrap();
        assert_eq!(
            ledger.answer(FrameTicket(99), FrameOutcome::Presented),
            None
        );
        assert_eq!(ledger.answer(first, FrameOutcome::Presented), Some(false));
        assert_eq!(ledger.answer(first, FrameOutcome::Presented), None);
        assert_eq!(ledger.progress(3, true).presented, 1);
    }
}
