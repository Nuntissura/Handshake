//! When a wake is posted, and when it is posted again. Decisions only; `platform::wake` carries them out.
//!
//! A wake that was accepted is dispatched, unless it is cancelled, replaced or the loop ends. Posting it can fail (a
//! full message queue) and a posted message can be thrown away by somebody else's loop, so the slot keeps posting
//! until the event loop dispatched it (MT-215 F1), with a bound on how many are under way (F4).

use std::time::Duration;

/// How long after a refused post the next one follows.
pub(crate) const RETRY: Duration = Duration::from_millis(10);

/// How long a post may go undispatched before it is repeated.
pub(crate) const REPEAT: Duration = Duration::from_millis(500);

/// The most posts of one lane that may be in the message queue. A loop that does not run gets no more.
pub(crate) const MAX_IN_QUEUE: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Armed {
    value: u64,
    due: u64,
}

/// What to do now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// Nothing is wanted: no timer.
    Idle,
    /// Look again at this time.
    LookAt(u64),
    /// Post now, then report with [`WakeSlot::posted`].
    Post,
}

/// A wake that the loop dispatched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Taken {
    pub(crate) value: u64,
    /// How long after it was due.
    pub(crate) late: u64,
}

/// One wake at a time: the newest.
#[derive(Debug, Default)]
pub(crate) struct WakeSlot {
    armed: Option<Armed>,
    in_queue: u32,
    last_post: u64,
}

impl WakeSlot {
    /// Ask for a wake. One that has not been dispatched yet is replaced.
    pub(crate) fn request(&mut self, now: u64, value: u64, delay: u64) -> Step {
        self.armed = Some(Armed {
            value,
            due: now.saturating_add(delay),
        });
        self.step(now)
    }

    /// What is needed at this time.
    pub(crate) fn step(&self, now: u64) -> Step {
        let Some(armed) = self.armed else {
            return Step::Idle;
        };
        if now < armed.due {
            return Step::LookAt(armed.due);
        }
        if self.in_queue == 0 {
            return Step::Post;
        }
        let repeat_at = self.last_post.saturating_add(nanos(REPEAT));
        if now < repeat_at {
            Step::LookAt(repeat_at)
        } else if self.in_queue < MAX_IN_QUEUE {
            Step::Post
        } else {
            Step::LookAt(now.saturating_add(nanos(REPEAT)))
        }
    }

    /// The outcome of a post. Returns when to look again.
    pub(crate) fn posted(&mut self, now: u64, taken: bool) -> u64 {
        if taken {
            self.in_queue = self.in_queue.saturating_add(1);
            self.last_post = now;
            now.saturating_add(nanos(REPEAT))
        } else {
            now.saturating_add(nanos(RETRY))
        }
    }

    /// The loop dispatched a post of this slot. Returns the wake, if one is due.
    pub(crate) fn dispatched(&mut self, now: u64) -> Option<Taken> {
        self.in_queue = self.in_queue.saturating_sub(1);
        let armed = self.armed.filter(|armed| now >= armed.due)?;
        self.armed = None;
        Some(Taken {
            value: armed.value,
            late: now.saturating_sub(armed.due),
        })
    }

    /// Withdraw the wake. A post that is under way then dispatches nothing.
    pub(crate) fn cancel(&mut self) {
        self.armed = None;
    }
}

/// Whole nanoseconds, as far as they fit.
pub(crate) fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    /// Runs the slot's timer until `until` against a queue that takes posts when `takes` says so, and dispatches
    /// nothing. Returns the times of the accepted posts.
    fn post_times(
        slot: &mut WakeSlot,
        from: u64,
        until: u64,
        takes: impl Fn(u64) -> bool,
    ) -> Vec<u64> {
        let mut times = Vec::new();
        let mut now = from;
        while now <= until {
            now = match slot.step(now) {
                Step::Idle => break,
                Step::LookAt(at) => at,
                Step::Post => {
                    let taken = takes(now);
                    if taken {
                        times.push(now);
                    }
                    slot.posted(now, taken)
                }
            };
        }
        times
    }

    #[test]
    fn a_wake_is_posted_when_due_and_not_before() {
        let mut slot = WakeSlot::default();
        assert_eq!(slot.request(10 * MS, 7, 200 * MS), Step::LookAt(210 * MS));
        assert_eq!(slot.step(209 * MS), Step::LookAt(210 * MS));
        assert_eq!(slot.step(210 * MS), Step::Post);
        slot.posted(210 * MS, true);
        assert_eq!(
            slot.dispatched(225 * MS),
            Some(Taken {
                value: 7,
                late: 15 * MS
            })
        );
        assert_eq!(slot.step(226 * MS), Step::Idle, "nothing keeps ticking");
    }

    #[test]
    fn a_refused_post_is_retried_until_one_is_taken() {
        let mut slot = WakeSlot::default();
        assert_eq!(slot.request(0, 1, 0), Step::Post);
        let times = post_times(&mut slot, 0, 400 * MS, |now| now >= 300 * MS);
        assert_eq!(
            times.first(),
            Some(&(300 * MS)),
            "the first chance after the queue had room"
        );
        assert!(slot.dispatched(301 * MS).is_some());
    }

    #[test]
    fn a_post_that_is_never_dispatched_is_repeated_within_a_bound() {
        let mut slot = WakeSlot::default();
        slot.request(0, 1, 0);
        let times = post_times(&mut slot, 0, 60_000 * MS, |_| true);
        assert_eq!(
            times.len(),
            MAX_IN_QUEUE as usize,
            "a dead loop does not fill its own queue"
        );
        assert!(times
            .windows(2)
            .all(|pair| pair[1] - pair[0] >= nanos(REPEAT)));
    }

    #[test]
    fn the_loop_coming_back_dispatches_the_wake_once() {
        let mut slot = WakeSlot::default();
        slot.request(0, 9, 0);
        post_times(&mut slot, 0, 5_000 * MS, |_| true);
        assert_eq!(
            slot.dispatched(5_001 * MS).map(|taken| taken.value),
            Some(9)
        );
        assert_eq!(slot.dispatched(5_002 * MS), None);
        assert_eq!(slot.dispatched(5_003 * MS), None);
        assert_eq!(
            slot.request(5_004 * MS, 10, 0),
            Step::Post,
            "no post is thought to be under way"
        );
    }

    #[test]
    fn a_post_of_an_earlier_wake_does_not_bring_the_next_one_early() {
        let mut slot = WakeSlot::default();
        slot.request(0, 1, 0);
        slot.posted(0, true);
        slot.request(MS, 2, 200 * MS);
        assert_eq!(slot.dispatched(2 * MS), None);
        assert_eq!(slot.step(201 * MS), Step::Post);
        slot.posted(201 * MS, true);
        assert_eq!(slot.dispatched(202 * MS).map(|taken| taken.value), Some(2));
    }

    #[test]
    fn the_newest_request_wins() {
        let mut slot = WakeSlot::default();
        slot.request(0, 1, 500 * MS);
        assert_eq!(slot.request(MS, 2, 0), Step::Post);
        slot.posted(MS, true);
        assert_eq!(slot.dispatched(2 * MS).map(|taken| taken.value), Some(2));
        assert_eq!(slot.step(600 * MS), Step::Idle);
    }

    #[test]
    fn a_cancelled_wake_is_not_dispatched() {
        let mut slot = WakeSlot::default();
        slot.request(0, 1, 0);
        slot.posted(0, true);
        slot.cancel();
        assert_eq!(slot.dispatched(MS), None);
        assert_eq!(slot.step(MS), Step::Idle);
    }

    #[test]
    fn a_dispatch_nobody_asked_for_gives_nothing() {
        let mut slot = WakeSlot::default();
        assert_eq!(slot.dispatched(5), None);
        assert_eq!(slot.request(6, 1, 0), Step::Post);
    }
}
